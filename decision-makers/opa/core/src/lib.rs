#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]
#![feature(associated_type_defaults)]

#[macro_use]
extern crate async_backtrace;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate serde_with;
#[macro_use]
extern crate tracing;
#[macro_use]
extern crate typed_builder;

mod endpoints;
mod models;

pub use endpoints::*;
pub use models::*;

use authzen_service_util::*;
use hyper::{client::HttpConnector, header::HeaderMap, Body, Request, Response, StatusCode};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use std::{ops::Deref, sync::Arc, time::Duration};
use tokio::time::timeout;

pub const DEFAULT_TIMEOUT_SECONDS: u64 = 30;

lazy_static! {
    pub(crate) static ref OPA_DEBUG: bool = std::env::var("OPA_DEBUG").map(|s| s == "true").unwrap_or_default();
    pub(crate) static ref OPA_EXPLAIN: Option<String> = std::env::var("OPA_EXPLAIN").ok();
    pub(crate) static ref OPA_PRETTY: bool = std::env::var("OPA_PRETTY").map(|s| s == "true").unwrap_or_default();
}

#[derive(Clone, Debug)]
pub struct OPAClient<Connector = HttpsConnector<HttpConnector>>(Arc<_OPAClient<Connector>>);

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct _OPAClient<Connector = HttpsConnector<HttpConnector>> {
    base_uri: String,
    client: hyper::client::Client<Connector>,
    headers: HeaderMap,
    timeout: Duration,
    pub data_path: String,
    pub query: String,
}

impl Deref for OPAClient {
    type Target = _OPAClient;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[async_trait]
impl Client for OPAClient {
    type Error = Error;

    fn headers(&self) -> &HeaderMap {
        &self.deref().headers
    }

    #[framed]
    async fn rest(&self, request: Request<Body>) -> Result<Response<Body>, Self::Error> {
        let response = timeout(self.timeout, self.client.request(request))
            .await
            .map_err(|_| Error::new(StatusCode::REQUEST_TIMEOUT))?
            .map_err(Error::default_details)?;
        cfg_if! {
            if #[cfg(feature = "debug")] {
                let (parts, body) = response.into_parts();
                let bytes = hyper::body::to_bytes(body).await.unwrap();
                println!("{}", String::from_utf8_lossy(bytes.as_ref()));
                Ok(Response::from_parts(parts, hyper::Body::from(bytes)))
            } else {
                Ok(response)
            }
        }
    }
}

impl ClientBaseUri for OPAClient {
    fn base_uri(&self) -> &str {
        &self.deref().base_uri
    }
}

impl OPAClient {
    pub fn new(
        scheme: &str,
        host: &str,
        port: &Option<u16>,
        data_path: impl ToString,
        query: impl ToString,
    ) -> Result<Self, anyhow::Error> {
        let client: hyper::Client<HttpsConnector<HttpConnector>> = hyper::Client::builder().build::<_, hyper::Body>(
            HttpsConnectorBuilder::new()
                .with_webpki_roots()
                .https_or_http()
                .enable_http1()
                .build(),
        );

        let headers = HeaderMap::new();
        let port = port.map(|x| format!(":{x}")).unwrap_or_default();
        let base_uri = format!("{scheme}://{host}{port}");

        Ok(Self(Arc::new(_OPAClient {
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
            base_uri,
            client,
            headers,
            data_path: data_path.to_string(),
            query: query.to_string(),
        })))
    }
    pub fn timeout(self, timeout: Duration) -> Self {
        let mut _opa_client = Arc::try_unwrap(self.0).unwrap();
        _opa_client.timeout = timeout;
        Self(Arc::new(_opa_client))
    }

    #[cfg(test)]
    pub(crate) fn test() -> Result<OPAClient, anyhow::Error> {
        OPAClient::new("http", "127.0.0.1", &Some(8181), "app", "authz")
    }
}
