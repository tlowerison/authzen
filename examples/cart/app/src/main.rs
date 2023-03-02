#[macro_use]
extern crate tracing;

use ::authzen::decision_makers::opa::OPAClient;
use ::authzen::service_util::{make_account_span, try_join_safe};
use ::authzen::session::{redis_store, RedisStoreConfig, RedisStoreNodeConfig, SessionLayer};
use ::authzen::transaction_caches::mongodb::{mongodb_client, MongodbConfig};
use ::axum::{error_handling::HandleErrorLayer, extract::Extension};
use ::dotenv::dotenv;
use ::hyper::http::{header, method::Method};
use ::std::net::SocketAddr;
use ::std::time::Duration;
use ::tower::ServiceBuilder;
use ::tower_http::catch_panic::CatchPanicLayer;
use ::tower_http::cors::{AllowMethods, CorsLayer};
use ::tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use ::tower_http::{compression::CompressionLayer, LatencyUnit};
use ::tracing::Level;
use cart_app::*;

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

    let (db, session_store, (mongodb_db, mongodb_collection)) = try_join_safe!(
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
        mongodb_client(MongodbConfig {
            scheme: env::mongodb_scheme()?,
            username: env::mongodb_username()?,
            password: env::mongodb_password()?,
            host: env::mongodb_host()?,
            port: env::mongodb_port()?,
            args: env::mongodb_args()?,
            database: env::mongodb_database()?,
            collection: env::mongodb_collection()?,
        }),
    )?;

    #[cfg(feature = "graphql")]
    let graphql_schema = api::graphql::schema(db.clone());

    let allowed_origin = env::allowed_origin()?;
    info!("allowed origin: {allowed_origin:?}");

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
                .allow_origin(allowed_origin),
        )
        .layer(SessionLayer::<AccountSession, _, _, _>::encoded(
            session_store.clone(),
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

    let router = cart_app::api::router();

    let app = router
        .layer(Extension(Clients {
            db: db.clone(),
            opa_client: OPAClient::new(
                &env::opa_scheme()?,
                &env::opa_host()?,
                &env::opa_port()?,
                &env::opa_data_path()?,
                &env::opa_query()?,
            )?,
            session_store,
            tx_cache_client: ApiTxCacheClient {
                db: mongodb_db,
                collection: mongodb_collection,
            },
        }))
        .layer(app_middleware);

    let rest_socket_addr = SocketAddr::new(env::ip_addr()?, env::port()?);
    info!("running rest server on {rest_socket_addr}");

    axum::Server::bind(&rest_socket_addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(authzen::service_util::shutdown_signal())
        .await?;

    Ok(())
}
