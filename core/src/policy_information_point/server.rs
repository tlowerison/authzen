use crate::policy_information_point::*;
use ::axum::extract::{Extension, RawBody};
use ::axum::routing::Router;
use ::axum::{error_handling::HandleErrorLayer, TypedHeader};
use ::futures::future::BoxFuture;
use ::hyper::http::header::{self, HeaderName, HeaderValue};
use ::hyper::http::method::Method;
use ::hyper::StatusCode;
use ::serde::de::DeserializeOwned;
use ::std::net::SocketAddr;
use ::std::time::Duration;
use ::tower::ServiceBuilder;
use ::tower_http::catch_panic::CatchPanicLayer;
use ::tower_http::compression::CompressionLayer;
use ::tower_http::cors::{AllowMethods, AllowOrigin, CorsLayer};

#[macro_export]
macro_rules! server {
    (::<$Q:ty, $Ctx:ty, $Id:ty>($socket_addr:expr, $clients:expr, $config:expr $(,)?)) => {
        #[authzen::tokio::main]
        async fn main() -> Result<(), anyhow::Error> {
            authzen::dotenv::dotenv().ok();

            authzen::service_util::install_tracing(true)?;

            $crate::policy_information_point::server::<$Q, $Ctx, $Id, _>($socket_addr, $clients, $config).await
        }
    };
}

#[derive(Clone, Debug, TypedBuilder)]
#[builder(field_defaults(default, setter(into, strip_option)))]
pub struct ServerConfig {
    pub allow_credentials: Option<bool>,
    pub allow_origin: Option<AllowOrigin>,
    pub timeout_duration: Option<Duration>,
}

pub async fn server<Q, Ctx, Id, Clients>(
    socket_addr: impl Into<SocketAddr>,
    clients: Clients,
    config: ServerConfig,
) -> Result<(), anyhow::Error>
where
    Id: DeserializeOwned + Send + Serialize + 'static,
    Clients: Clone + Send + Sync + 'static,
    Ctx: Send + Sync,
    Q: DeserializeOwned + Query<Ctx, Error = service_util::Error> + Send,
    (Clients, Option<Id>): Into<Ctx>,
{
    // note: ordering of middleware layers is important, see https://docs.rs/axum/latest/axum/middleware/index.html#ordering
    let app_middleware = ServiceBuilder::new()
        // compress responses
        .layer(CompressionLayer::new())
        .layer({
            let layer = CorsLayer::new()
                .allow_methods(AllowMethods::list([Method::GET, Method::OPTIONS, Method::POST]))
                .allow_headers([header::CONTENT_TYPE, X_TRANSACTION_ID.clone()])
                .allow_credentials(config.allow_credentials.unwrap_or_default());
            match config.allow_origin {
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
            ::tower_http::trace::TraceLayer::new_for_http()
                .on_response(::tower_http::trace::DefaultOnRequest::new().level(::tracing::Level::INFO))
                .on_response(
                    ::tower_http::trace::DefaultOnResponse::new()
                        .level(::tracing::Level::INFO)
                        .latency_unit(::tower_http::LatencyUnit::Micros),
                )
                .on_failure(
                    ::tower_http::trace::DefaultOnFailure::new()
                        .level(::tracing::Level::ERROR)
                        .latency_unit(::tower_http::LatencyUnit::Micros),
                ),
        );

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

    let app = router.layer(Extension(clients));

    let service = match config.timeout_duration {
        Some(timeout_duration) => {
            app.layer(
                app_middleware
                    // handle errors produced by fallible middleware layers (e.g. timeout)
                    .layer(HandleErrorLayer::new(service_util::handle_middleware_error))
                    .timeout(timeout_duration)
                    .into_inner(),
            )
            .into_make_service()
        }
        None => app.layer(app_middleware.into_inner()).into_make_service(),
    };

    let socket_addr = socket_addr.into();
    log::info!("running policy information point server on {socket_addr}");

    axum::Server::bind(&socket_addr)
        .serve(service)
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

impl axum::response::IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        (self.headers, axum::Json(self.values)).into_response()
    }
}

impl<Id> axum::headers::Header for TransactionId<Id>
where
    Id: DeserializeOwned + Serialize,
{
    fn name() -> &'static HeaderName {
        &X_TRANSACTION_ID
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        I: Iterator<Item = &'i HeaderValue>,
    {
        let value = values.next().ok_or_else(axum::headers::Error::invalid)?;

        let value = value.to_str().map_err(|_| axum::headers::Error::invalid())?;
        match serde_plain::from_str(value) {
            Ok(transaction_id) => Ok(Self(transaction_id)),
            Err(_) => Err(axum::headers::Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<HeaderValue>,
    {
        let value = HeaderValue::from_str(&serde_plain::to_string(&self.0).unwrap()).unwrap();
        values.extend(std::iter::once(value));
    }
}

impl<E> axum::response::IntoResponse for QueryError<E>
where
    E: axum::response::IntoResponse,
{
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::Deserialization(err) => axum::response::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::boxed(axum::body::Full::from(err.to_string())))
                .unwrap(),
            Self::Query(err) => err.into_response(),
            Self::Serialization(err) => axum::response::Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(axum::body::boxed(axum::body::Full::from(err.to_string())))
                .unwrap(),
        }
    }
}
