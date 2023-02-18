use crate::set_trace_parent;
use derive_more::*;
use hyper::header::{HeaderName, FORWARDED};
use hyper::http::Request;
use hyper::Body;
use serde::{Deserialize, Serialize};
use session_util::AccountSessionSubject;
use std::fmt::{Debug, Display};
use tokio::signal;
use tower_http::request_id::MakeRequestId;
use tracing::{info, Span};
use uuid::Uuid;

#[cfg(feature = "axum-05")]
use axum_05::{extract::RawBody, BoxError};

#[cfg(feature = "axum-06")]
use axum_06::{extract::RawBody, BoxError};

#[cfg(feature = "max-allowed-request-body-size-small")]
#[allow(dead_code)]
const MAX_ALLOWED_REQUEST_BODY_SIZE: u64 = 102_400; // 100 KB

#[cfg(feature = "max-allowed-request-body-size-medium")]
#[allow(dead_code)]
const MAX_ALLOWED_REQUEST_BODY_SIZE: u64 = 1_048_576; // 1 MB

#[cfg(feature = "max-allowed-request-body-size-large")]
#[allow(dead_code)]
const MAX_ALLOWED_REQUEST_BODY_SIZE: u64 = 10_485_760; // 10 MB

pub static X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
pub static X_REAL_IP: HeaderName = HeaderName::from_static("x-real-ip");
pub static X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

static _X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// NOTE: this struct cannot be extracted with an Extension, it can only be extracted with a TypedHeader
/// suggested usage: if using an Axum ServiceBuilder, add a call
/// ```rust
/// .set_request_id(service_util::X_REQUEST_ID, service_util::RequestId::default())
/// ```
#[derive(Clone, Copy, Default, Deref, Deserialize, Eq, From, Into, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RequestId(pub Uuid);

impl Debug for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[cfg(any(feature = "axum-05", feature = "axum-06"))]
#[framed]
pub async fn handle_middleware_error(err: BoxError) -> crate::Error {
    if err.is::<tower::timeout::error::Elapsed>() {
        crate::Error::msg(hyper::http::StatusCode::REQUEST_TIMEOUT, "Request took too long")
    } else {
        tracing::error!("Unhandled internal error: {err}");
        crate::Error::default()
    }
}

#[framed]
pub async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("failed to install Ctrl+C handler") };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("signal received, starting graceful shutdown");
}

#[cfg(any(feature = "axum-05", feature = "axum-06"))]
pub async fn from_body<T: serde::de::DeserializeOwned>(RawBody(body): RawBody) -> Result<T, crate::Error> {
    use hyper::body::HttpBody;

    let content_length = body.size_hint().upper().unwrap_or(MAX_ALLOWED_REQUEST_BODY_SIZE + 1);
    if content_length < MAX_ALLOWED_REQUEST_BODY_SIZE {
        hyper::body::to_bytes(body)
            .await
            .map_err(|_| crate::Error::bad_request_msg("invalid request body"))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|err| crate::Error::bad_request_msg(format!("could not deserialize body: {err}")))
            })
    } else {
        Err(crate::Error::bad_request_msg(format!(
            "request body is too large, maximum allowed size is {MAX_ALLOWED_REQUEST_BODY_SIZE}"
        )))
    }
}

#[cfg(any(feature = "axum-05", feature = "axum-06"))]
pub async fn body_bytes(RawBody(body): RawBody) -> Result<Vec<u8>, crate::Error> {
    use hyper::body::HttpBody;

    let content_length = body.size_hint().upper().unwrap_or(MAX_ALLOWED_REQUEST_BODY_SIZE + 1);
    if content_length < MAX_ALLOWED_REQUEST_BODY_SIZE {
        hyper::body::to_bytes(body)
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|_| crate::Error::bad_request_msg("invalid request body"))
    } else {
        Err(crate::Error::bad_request_msg(format!(
            "request body is too large, maximum allowed size is {MAX_ALLOWED_REQUEST_BODY_SIZE}"
        )))
    }
}

#[cfg(feature = "graphql")]
pub fn missing_session<E>(_: E) -> async_graphql::Error {
    use async_graphql::ErrorExtensions;
    async_graphql::Error::new("no active session").extend_with(|_, extensions| extensions.set("status", 400))
}

#[cfg(feature = "graphql")]
pub fn missing_data<E>(_: E) -> async_graphql::Error {
    use async_graphql::ErrorExtensions;
    async_graphql::Error::new("Internal Server Error").extend_with(|_, extensions| extensions.set("status", 500))
}

pub mod make_account_span {
    use super::*;

    pub fn debug<AccountId: Display + Send + Sync + 'static>(req: &Request<Body>) -> Span {
        set_trace_parent(
            req,
            tracing::debug_span!(
                target: "",
                "request",
                "http.method" = %req.method(),
                "http.target" = %req.uri(),
                "http.client_ip" = get_client_ip(req).map(display),
                "account_id" = get_account_id::<AccountId, Body>(req).map(display),
            ),
        )
    }
    pub fn error<AccountId: Display + Send + Sync + 'static>(req: &Request<Body>) -> Span {
        set_trace_parent(
            req,
            tracing::error_span!(
                target: "",
                "request",
                "http.method" = %req.method(),
                "http.target" = %req.uri(),
                "http.client_ip" = get_client_ip(req).map(display),
                "account_id" = get_account_id::<AccountId, Body>(req).map(display),
            ),
        )
    }
    pub fn info<AccountId: Display + Send + Sync + 'static>(req: &Request<Body>) -> Span {
        set_trace_parent(
            req,
            tracing::info_span!(
                target: "",
                "request",
                 "http.method" = %req.method(),
                "http.target" = %req.uri(),
                "http.client_ip" = get_client_ip(req).map(display),
                "account_id" = get_account_id::<AccountId, Body>(req).map(display),
            ),
        )
    }
    pub fn trace<AccountId: Display + Send + Sync + 'static>(req: &Request<Body>) -> Span {
        set_trace_parent(
            req,
            tracing::trace_span!(
                target: "",
                "request",
                "http.method" = %req.method(),
                "http.target" = %req.uri(),
                "http.client_ip" = get_client_ip(req).map(display),
                "account_id" = get_account_id::<AccountId, Body>(req).map(display),
            ),
        )
    }
    pub fn warn<AccountId: Display + Send + Sync + 'static>(req: &Request<Body>) -> Span {
        set_trace_parent(
            req,
            tracing::warn_span!(
                target: "",
                "request",
                "http.method" = %req.method(),
                "http.target" = %req.uri(),
                "http.client_ip" = get_client_ip(req).map(display),
                "account_id" = get_account_id::<AccountId, Body>(req).map(display),
            ),
        )
    }
}

pub fn get_client_ip(req: &Request<Body>) -> Option<&str> {
    let headers = req.headers();
    headers
        .get(&X_FORWARDED_FOR)
        .or_else(|| headers.get(&X_REAL_IP))
        .or_else(|| headers.get(FORWARDED))
        .and_then(|header_value| header_value.to_str().ok())
}

pub fn get_account_id<AccountId: Send + Sync + 'static, B>(req: &Request<B>) -> Option<&AccountId> {
    match req.extensions().get::<Option<AccountSessionSubject<AccountId>>>() {
        Some(Some(session)) => Some(&session.0),
        _ => None,
    }
}

impl MakeRequestId for RequestId {
    fn make_request_id<B>(&mut self, _: &Request<B>) -> Option<tower_http::request_id::RequestId> {
        let request_id = Uuid::new_v4().to_string().parse().unwrap();
        Some(tower_http::request_id::RequestId::new(request_id))
    }
}

#[cfg(feature = "axum-05")]
impl axum_05::headers::Header for RequestId {
    fn name() -> &'static HeaderName {
        &_X_REQUEST_ID
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_05::headers::Error>
    where
        I: Iterator<Item = &'i hyper::header::HeaderValue>,
    {
        let value = values.next().ok_or_else(axum_05::headers::Error::invalid)?;

        let value = value.to_str().map_err(|_| axum_05::headers::Error::invalid())?;
        match Uuid::parse_str(value) {
            Ok(request_id) => Ok(Self(request_id)),
            Err(_) => Err(axum_05::headers::Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<hyper::header::HeaderValue>,
    {
        let value =
            hyper::header::HeaderValue::from_str(self.0.as_simple().encode_lower(&mut Uuid::encode_buffer())).unwrap();
        values.extend(std::iter::once(value));
    }
}

#[cfg(feature = "axum-06")]
impl axum_06::headers::Header for RequestId {
    fn name() -> &'static HeaderName {
        &_X_REQUEST_ID
    }

    fn decode<'i, I>(values: &mut I) -> Result<Self, axum_06::headers::Error>
    where
        I: Iterator<Item = &'i hyper::header::HeaderValue>,
    {
        let value = values.next().ok_or_else(axum_06::headers::Error::invalid)?;

        let value = value.to_str().map_err(|_| axum_06::headers::Error::invalid())?;
        match Uuid::parse_str(value) {
            Ok(request_id) => Ok(Self(request_id)),
            Err(_) => Err(axum_06::headers::Error::invalid()),
        }
    }

    fn encode<E>(&self, values: &mut E)
    where
        E: Extend<hyper::header::HeaderValue>,
    {
        let value =
            hyper::header::HeaderValue::from_str(self.0.as_simple().encode_lower(&mut Uuid::encode_buffer())).unwrap();
        values.extend(std::iter::once(value));
    }
}
