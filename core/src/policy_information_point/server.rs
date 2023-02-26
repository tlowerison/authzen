use crate::policy_information_point::*;
use ::axum::extract::{Extension, RawBody};
use ::axum::routing::Router;
use ::axum::{error_handling::HandleErrorLayer, TypedHeader};
use ::futures::future::BoxFuture;
use ::hyper::http::{header, method::Method};
use ::serde::de::DeserializeOwned;
use ::std::net::SocketAddr;
use ::std::time::Duration;
use ::tower::ServiceBuilder;
use ::tower_http::catch_panic::CatchPanicLayer;
use ::tower_http::cors::{AllowMethods, AllowOrigin, CorsLayer};
use ::tower_http::trace::{DefaultOnFailure, DefaultOnRequest, DefaultOnResponse, TraceLayer};
use ::tower_http::{compression::CompressionLayer, LatencyUnit};

#[cfg(feature = "tracing")]
use ::tracing::Level;

pub async fn server<Q, Ctx, Id, Clients>(
    socket_addr: SocketAddr,
    clients: Clients,
    timeout_duration: impl Into<Duration>,
    allow_credentials: bool,
    allow_origin: impl Into<Option<AllowOrigin>>,
) -> Result<(), anyhow::Error>
where
    Id: DeserializeOwned + Send + Serialize + 'static,
    Clients: Clone + Send + Sync + 'static,
    Ctx: Send + Sync,
    Q: DeserializeOwned + Query<Ctx, Error = service_util::Error> + Send,
    (Clients, Option<Id>): Into<Ctx>,
{
    service_util::install_tracing(true)?;

    // note: ordering of middleware layers is important, see https://docs.rs/axum/latest/axum/middleware/index.html#ordering
    let app_middleware = ServiceBuilder::new()
        // compress responses
        .layer(CompressionLayer::new())
        .layer({
            let layer = CorsLayer::new()
                .allow_methods(AllowMethods::list([Method::GET, Method::OPTIONS, Method::POST]))
                .allow_headers([header::CONTENT_TYPE, X_TRANSACTION_ID.clone()])
                .allow_credentials(allow_credentials);
            match allow_origin.into() {
                Some(allow_origin) => layer.allow_origin(allow_origin),
                None => layer,
            }
        })
        // handle panics by responding with a 500 instead of aborting connection
        .layer(CatchPanicLayer::new());

    #[cfg(feature = "tracing")]
    let app_middleware = app_middleware
        // add high level tracing of requests and responses
        .layer(
            TraceLayer::new_for_http()
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
        );

    let app_middleware = app_middleware
        // handle errors produced by fallible middleware layers (e.g. timeout)
        .layer(HandleErrorLayer::new(service_util::handle_middleware_error))
        // timeout requests after specified duration
        .timeout(timeout_duration.into())
        .into_inner();

    let router = Router::new().route(
        "/",
        axum::routing::post(
            |Extension(clients): Extension<Clients>,
             transaction_id: Option<TypedHeader<TransactionId<Id>>>,
             raw_body: RawBody| {
                async move {
                    let transaction_id = transaction_id.map(|x| x.0 .0);
                    let ctx = Into::<Ctx>::into((clients, transaction_id));
                    let query: Q = service_util::from_body(raw_body)
                        .await
                        .map_err(QueryError::Deserialization)?;
                    Ok::<_, QueryError<service_util::Error>>(query.fetch(&ctx).await?)
                }
            },
        ),
    );

    let app = router.layer(Extension(clients)).layer(app_middleware);

    // info!("running policy information point server on {rest_socket_addr}");

    axum::Server::bind(&socket_addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(service_util::shutdown_signal())
        .await?;

    Ok(())
}

pub async fn query_handler<Q, Ctx, Id, E, Clients>() -> Box<
    dyn Fn(
            Extension<Clients>,
            RawBody,
            Option<TypedHeader<TransactionId<Id>>>,
        ) -> BoxFuture<'static, Result<Response, QueryError<E>>>
        + 'static,
>
where
    Id: Send + 'static,
    Clients: Send + 'static,
    Ctx: Send + Sync,
    Q: DeserializeOwned + Query<Ctx, Error = E> + Send,
    (Clients, Option<Id>): Into<Ctx>,
{
    Box::new(
        |Extension(clients): Extension<Clients>,
         raw_body: RawBody,
         transaction_id: Option<TypedHeader<TransactionId<Id>>>| {
            Box::pin(async move {
                let transaction_id = transaction_id.map(|x| x.0 .0);
                let ctx = Into::<Ctx>::into((clients, transaction_id));
                let query: Q = service_util::from_body(raw_body)
                    .await
                    .map_err(QueryError::Deserialization)?;
                Ok(query.fetch(&ctx).await?)
            })
        },
    )
}
