#[macro_use]
extern crate tracing;

use authzen::decision_makers::opa::OPAClient;
use authzen::service_util::{make_account_span, try_join_safe};
use authzen_examples::*;
use axum::{error_handling::HandleErrorLayer, extract::Extension};
use dotenv::dotenv;
use hyper::http::{header, method::Method};
use session_util::{redis_store, RedisStoreConfig, RedisStoreNodeConfig, SessionLayer};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::cors::{AllowMethods, CorsLayer};
use tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use tower_http::{compression::CompressionLayer, LatencyUnit};
use tracing::Level;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();
    authzen::service_util::install_tracing(env::otel_enabled()?)?;

    let redis_is_cluster = env::redis_is_cluster()?;
    let redis_username = env::redis_username()?;
    let redis_password = env::redis_password()?;
    let redis_host = env::redis_host()?;
    let redis_port = env::redis_port()?;
    let session_secret = env::session_secret()?;

    let (db, account_session_store, tx_cache_client) = try_join_safe!(
        async { db().await.map_err(anyhow::Error::from) },
        redis_store(
            RedisStoreConfig {
                key_name: "session_id",
                key: session_secret.clone(),
                username: redis_username.clone(),
                password: redis_password.clone(),
            },
            [RedisStoreNodeConfig {
                host: redis_host.clone(),
                port: redis_port,
                db: None,
            }],
            redis_is_cluster,
        ),
        get_mongodb_client(),
    )?;

    #[cfg(feature = "graphql")]
    let graphql_schema = api::graphql::schema(db.clone());

    let allowed_origins = env::allowed_origins()?;
    info!("allowed origins: {allowed_origins:?}");

    // note: ordering of middleware layers is important, see https://docs.rs/axum/latest/axum/middleware/index.html#ordering
    let app_middleware = ServiceBuilder::new()
        // limit how many concurrent responses the service can handle
        .concurrency_limit(env::concurrency_limit()?)
        // compress responses
        .layer(CompressionLayer::new())
        .layer(
            CorsLayer::new()
                .allow_methods(AllowMethods::list([
                    Method::DELETE,
                    Method::GET,
                    Method::HEAD,
                    Method::OPTIONS,
                    Method::PATCH,
                    Method::POST,
                    Method::PUT,
                ]))
                .allow_credentials(env::allow_credentials()?)
                .allow_headers([header::CONTENT_TYPE])
                .allow_origin(allowed_origins),
        )
        .layer(SessionLayer::<AccountSession, _, _, _>::encoded(
            account_session_store.clone(),
            env::session_jwt_public_certificate()?,
            service::SESSION_JWT_VALIDATION.clone(),
        ))
        // handle panics by responding with a 500 instead of aborting connection
        .layer(CatchPanicLayer::new())
        // add high level tracing of requests and responses
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(make_account_span::info::<AccountId>)
                .on_response(DefaultOnRequest::new().level(Level::INFO))
                .on_response(
                    DefaultOnResponse::new()
                        .level(Level::INFO)
                        .latency_unit(LatencyUnit::Micros),
                )
                .on_failure(
                    DefaultOnFailure::new()
                        .level(Level::ERROR)
                        .latency_unit(LatencyUnit::Micros),
                ),
        )
        // handle errors produced by fallible middleware layers (e.g. timeout)
        .layer(HandleErrorLayer::new(authzen::service_util::handle_middleware_error))
        // timeout requests after specified duration
        .timeout(Duration::from_secs(env::request_timeout_in_secs()?))
        .into_inner();

    let router = authzen_examples::api::router();

    let app = router
        .layer(Extension(Clients {
            account_session_store,
            db: db.clone(),
            opa_client: OPAClient::new(
                &env::opa_scheme()?,
                &env::opa_host()?,
                &env::opa_port()?,
                &env::opa_data_path()?,
                &env::opa_query()?,
            )?,
            tx_cache_client,
        }))
        .layer(app_middleware);

    let rest_socket_addr = SocketAddr::new(IpAddr::V4(env::rest_ipv4_addr()?), env::rest_port()?);
    info!("running rest server on {rest_socket_addr}");

    axum::Server::bind(&rest_socket_addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(authzen::service_util::shutdown_signal())
        .await?;

    Ok(())
}

async fn get_mongodb_client() -> Result<ApiTxCacheClient, anyhow::Error> {
    let mut connection_string = url::Url::parse("mongodb://localhost")?;

    connection_string
        .set_scheme(&env::mongodb_scheme()?)
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url scheme"))?;

    if let Some(username) = env::mongodb_username()? {
        connection_string
            .set_username(&username)
            .map_err(|_| anyhow::Error::msg("unable to set mongodb url username"))?;
    }

    let password = env::mongodb_username()?;
    connection_string
        .set_password(password.as_ref().map(|x| &**x))
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url password"))?;

    connection_string
        .set_host(Some(&env::mongodb_host()?))
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url host"))?;
    connection_string.set_path("/");
    connection_string
        .set_port(env::mongodb_port()?)
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url port"))?;

    let args = env::mongodb_args()?;
    connection_string.set_query(args.as_ref().map(|x| &**x));

    info!("connecting to mongodb");

    let mut mongodb_client_options = mongodb::options::ClientOptions::parse(connection_string).await?;
    mongodb_client_options.app_name = Some("accounts".into());
    let client = mongodb::Client::with_options(mongodb_client_options)?;

    let mongodb_database = env::mongodb_database()?;
    let db = client.database(&mongodb_database);

    info!("pinging mongodb");
    db.run_command(mongodb::bson::doc! {"ping": 1}, None).await?;

    info!("connected to mongodb successfully");

    let mongodb_collection = env::mongodb_collection()?;

    let has_collection = db
        .list_collection_names(None)
        .await?
        .into_iter()
        .any(|collection_name| collection_name == mongodb_collection);
    if !has_collection {
        info!("creating mongodb collection `{mongodb_collection}`");
        db.create_collection(&mongodb_collection, None).await?;
        info!("created mongodb collection `{mongodb_collection}`");
    }

    let collection = db.collection::<authzen::transaction_caches::mongodb::TxEntityFull>(&mongodb_collection);

    info!("initializing mongodb ttl index");
    authzen::transaction_caches::mongodb::initialize_ttl_index(&collection, None).await?;
    info!("initialized mongodb ttl index");

    Ok(ApiTxCacheClient { db, collection })
}
