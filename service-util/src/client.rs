use crate::{traceparent, TRACEPARENT};
use async_trait::async_trait;
use concat_string::concat_string;
use hyper::body::to_bytes;
use hyper::http::header::{HeaderMap, HeaderValue};
use hyper::http::uri::{InvalidUri, PathAndQuery};
use hyper::{client::connect::Connect, Body, Method, Request, Response, StatusCode, Uri};
use lazy_static::lazy_static;
use serde::{
    de::{DeserializeOwned, Deserializer},
    Deserialize, Serialize,
};
use std::fmt::{Debug, Display};
use std::{borrow::Cow, ops::Deref};
use thiserror::Error;
use tracing::instrument;

lazy_static! {
    static ref EMPTY_HEADER_MAP: HeaderMap = HeaderMap::default();
}

pub trait ClientError: Debug + Display + From<BaseClientError> + From<Response<Vec<u8>>> + Send + Sync {}

impl<E> ClientError for E where E: Debug + Display + From<BaseClientError> + From<Response<Vec<u8>>> + Send + Sync {}

#[derive(Debug, Error)]
pub enum BaseClientError {
    #[error("could not process body, too large")]
    BodyTooLarge,
    #[error("invalid uri: {0}")]
    InvalidUri(#[from] InvalidUri),
    #[error("could not send request / receive response")]
    NetworkError(#[from] hyper::Error),
    #[error("could not build body{}", if .0.is_empty() { .0.into() } else { format!(": {}", .0)})]
    RequestBodyBuild(String),
    #[error("could not serialize request body: {0}")]
    RequestBodySerialization(String),
    #[error("could not serialize request query params: {0}")]
    RequestParamsSerialization(#[from] serde_qs::Error),
    #[error("status: {status}; message: {message}")]
    Response { status: StatusCode, message: String },
    #[error("could not deserialize response body: {0}")]
    ResponseBodyDeserialization(serde_json::error::Error),
    #[error("{0}")]
    ResponseBodyInvalidCharacter(hyper::Error),
}

impl From<Response<Vec<u8>>> for BaseClientError {
    fn from(response: Response<Vec<u8>>) -> Self {
        Self::Response {
            status: response.status(),
            message: String::from_utf8_lossy(response.into_body().as_ref()).into(),
        }
    }
}

impl ClientError for BaseClientError {}

#[derive(Clone, Debug)]
pub struct Path(Cow<'static, str>);

impl std::fmt::Display for Path {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Cow::Borrowed(str) => write!(f, "{str}"),
            Cow::Owned(str) => write!(f, "{str}"),
        }
    }
}

impl From<String> for Path {
    fn from(str: String) -> Self {
        Self(Cow::Owned(str))
    }
}

impl From<&'static str> for Path {
    fn from(str: &'static str) -> Self {
        Self(Cow::Borrowed(str))
    }
}

impl AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        match &self.0 {
            Cow::Borrowed(str) => str,
            Cow::Owned(str) => str,
        }
    }
}

impl AsRef<[u8]> for Path {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            Cow::Borrowed(str) => str.as_ref(),
            Cow::Owned(str) => str.as_ref(),
        }
    }
}

impl Path {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

struct PathAndQueryWrapper(PathAndQuery);

impl AsRef<[u8]> for PathAndQueryWrapper {
    fn as_ref(&self) -> &[u8] {
        self.0.as_str().as_ref()
    }
}

pub trait Endpoint {
    const METHOD: Method;

    type Params<'a>: Debug + Send + Serialize = () where Self: 'a;
    type Response<T> = DefaultResponse<T>;

    fn path(&self) -> Path;
    fn params(&self) -> Self::Params<'_>;
    fn headers(&self) -> HeaderMap {
        HeaderMap::default()
    }

    /// body returns the opaque Body type because the actual body serialization may vary from endpoint
    /// to endpoint; e.g. one endpoint may want to use serde_json to serialize body data in json format
    /// while another may simply want to use the raw contents of a string as the bytes to be sent
    fn body(&self) -> Body {
        Body::empty()
    }

    fn optional(self) -> Optional<Self>
    where
        Self: Sized,
    {
        Optional(self)
    }
    fn ignore(self) -> Ignore<Self>
    where
        Self: Sized,
    {
        Ignore(self)
    }
    fn raw(self) -> Raw<Self>
    where
        Self: Sized,
    {
        Raw(self)
    }
    fn paginated(self, pagination: impl Into<Option<Pagination>>) -> Paged<Self>
    where
        Self: Sized,
    {
        Paged {
            endpoint: self,
            pagination: pagination.into().unwrap_or_default(),
        }
    }
}

#[async_trait]
pub trait Client: Debug {
    type Error: ClientError;

    fn headers(&self) -> &HeaderMap;
    async fn rest(&self, request: Request<Body>) -> Result<Response<Body>, Self::Error>;
}

pub trait ClientBaseUri {
    fn base_uri(&self) -> &str;
}

#[async_trait]
impl<Connector: 'static + Clone + Connect + Send + Sync> Client for hyper::Client<Connector, Body> {
    type Error = BaseClientError;

    fn headers(&self) -> &HeaderMap {
        &EMPTY_HEADER_MAP
    }
    async fn rest(&self, request: Request<Body>) -> Result<Response<Body>, Self::Error> {
        Ok(self.request(request).await?)
    }
}

trait EndpointUri<C: Client>: Endpoint {
    fn uri(&self, client: &C) -> Result<Uri, C::Error>;
}

#[derive(Clone, Debug)]
enum SerializedParams<T> {
    Owned(String),
    Ref(T),
}

impl<T> AsRef<str> for SerializedParams<T> {
    default fn as_ref(&self) -> &str {
        match self {
            Self::Owned(string) => string.as_ref(),
            Self::Ref(_) => unreachable!(),
        }
    }
}

impl<T: SealedAsRef<str>> AsRef<str> for SerializedParams<T> {
    fn as_ref(&self) -> &str {
        match self {
            Self::Owned(string) => string.as_ref(),
            Self::Ref(t) => t.sealed_as_ref(),
        }
    }
}

trait SealedAsRef<T: ?Sized> {
    fn sealed_as_ref(&self) -> &T;
}

impl<T, U: AsRef<T>> SealedAsRef<T> for U {
    default fn sealed_as_ref(&self) -> &T {
        self.as_ref()
    }
}

impl SealedAsRef<str> for () {
    fn sealed_as_ref(&self) -> &str {
        ""
    }
}

trait EndpointSerializedParams<C: Client>: Endpoint {
    fn serialized_params(&self) -> Result<SerializedParams<Self::Params<'_>>, C::Error>;
}

impl<C: Client, E: Endpoint> EndpointSerializedParams<C> for E {
    default fn serialized_params(&self) -> Result<SerializedParams<Self::Params<'_>>, C::Error> {
        let params = self.params();
        serde_qs::to_string(&params)
            .map(SerializedParams::Owned)
            .map_err(|err| C::Error::from(BaseClientError::from(err)))
    }
}

impl<C: Client, E: Endpoint> EndpointSerializedParams<C> for E
where
    for<'a> <E as Endpoint>::Params<'a>: SealedAsRef<str>,
{
    fn serialized_params(&self) -> Result<SerializedParams<Self::Params<'_>>, C::Error> {
        Ok(SerializedParams::Ref(self.params()))
    }
}

impl<C: Client, E: Endpoint + EndpointSerializedParams<C>> EndpointUri<C> for E {
    default fn uri(&self, _client: &C) -> Result<Uri, C::Error> {
        let path_uri = self.path();
        let uri = Uri::try_from(path_uri.as_str()).map_err(BaseClientError::from)?;
        let serialized_params = self.serialized_params()?;
        if serialized_params.as_ref() == "" {
            return Ok(uri);
        }

        let path_and_query: Option<PathAndQuery> = if let Some(path_and_query) = uri.path_and_query() {
            if let Some(query) = path_and_query.query() {
                Some(
                    concat_string!(path_and_query.path(), "?", query, "&", serialized_params)
                        .parse()
                        .map_err(BaseClientError::from)?,
                )
            } else {
                Some(
                    concat_string!(path_and_query.path(), "?", serialized_params)
                        .parse()
                        .map_err(BaseClientError::from)?,
                )
            }
        } else {
            Some(
                concat_string!("?", serialized_params)
                    .parse()
                    .map_err(BaseClientError::from)?,
            )
        };

        if let Some(path_and_query) = path_and_query {
            let mut uri_parts = uri.into_parts();
            uri_parts.path_and_query = Some(path_and_query);
            Ok(Uri::from_parts(uri_parts).unwrap())
        } else {
            Ok(uri)
        }
    }
}

impl<C: Client + ClientBaseUri, E: Endpoint + EndpointSerializedParams<C>> EndpointUri<C> for E {
    default fn uri(&self, client: &C) -> Result<Uri, C::Error> {
        let uri = Uri::try_from(concat_string!(client.base_uri(), self.path())).map_err(BaseClientError::from)?;
        let serialized_params = self.serialized_params()?;
        if serialized_params.as_ref() == "" {
            return Ok(uri);
        }

        let path_and_query: Option<PathAndQuery> = if let Some(path_and_query) = uri.path_and_query() {
            if let Some(query) = path_and_query.query() {
                Some(
                    concat_string!(path_and_query.path(), "?", query, "&", serialized_params)
                        .parse()
                        .map_err(BaseClientError::from)?,
                )
            } else {
                Some(
                    concat_string!(path_and_query.path(), "?", serialized_params)
                        .parse()
                        .map_err(BaseClientError::from)?,
                )
            }
        } else {
            Some(
                concat_string!("?", serialized_params)
                    .parse()
                    .map_err(BaseClientError::from)?,
            )
        };

        if let Some(path_and_query) = path_and_query {
            let mut uri_parts = uri.into_parts();
            uri_parts.path_and_query = Some(path_and_query);
            Ok(Uri::from_parts(uri_parts).unwrap())
        } else {
            Ok(uri)
        }
    }
}

trait EndpointRequest: Endpoint {
    fn request<C: Client>(
        &self,
        client: &C,
        endpoint_uri: &Uri,
        endpoint_headers: &HeaderMap,
        next_page: Option<NextPage>,
    ) -> Result<Request<Body>, C::Error>;
}

impl<E: Endpoint> EndpointRequest for E {
    fn request<C: Client>(
        &self,
        client: &C,
        endpoint_uri: &Uri,
        endpoint_headers: &HeaderMap,
        next_page: Option<NextPage>,
    ) -> Result<Request<Body>, C::Error> {
        let uri = match next_page.as_ref() {
            Some(NextPage::FullUri(uri)) => uri,
            None => endpoint_uri,
        };

        let request = Request::builder().uri(uri).method(E::METHOD);

        let mut request = request
            .body(self.body())
            .map_err(|e| BaseClientError::RequestBodyBuild(format!("{e}")))?;

        let headers = request.headers_mut();
        for (header_name, header_value) in client.headers().iter() {
            headers.append(header_name, header_value.clone());
        }
        for (header_name, header_value) in endpoint_headers.iter() {
            headers.append(header_name, header_value.clone());
        }

        if let Some(traceparent) = traceparent() {
            headers.append(&TRACEPARENT, HeaderValue::from_str(&traceparent).unwrap());
        }

        Ok(request)
    }
}

#[async_trait]
pub trait Query<C, T = ()>: Debug
where
    C: Client,
{
    async fn query(&self, client: &C) -> Result<T, C::Error>;
}

#[async_trait]
impl<E, C, T> Query<C, T> for E
where
    E: Debug + Endpoint + Send + Sync,
    C: Client + Debug + Sync,
    T: Debug + DeserializeOwned,
    E::Response<T>: DeserializeOwned + UnwrapResponse<T>,
{
    #[framed]
    #[instrument(err(Debug))]
    async fn query(&self, client: &C) -> Result<T, C::Error> {
        let uri = self.uri(client)?;
        let headers = self.headers();

        let request = self.request(client, &uri, &headers, None)?;

        let response = client.rest(request).await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            return Err(C::Error::from(raw_response::<C>(response).await?));
        }

        Ok(
            serde_json::from_slice::<E::Response<T>>(raw_response::<C>(response).await?.into_body().as_slice())
                .map_err(BaseClientError::ResponseBodyDeserialization)
                .map_err(C::Error::from)?
                .unwrap_response(),
        )
    }
}

#[derive(Debug)]
pub struct Optional<E>(pub E);

impl<E> Deref for Optional<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E: Endpoint> Optional<E> {
    pub fn ignore(self) -> Ignore<Self> {
        Ignore(self)
    }
}

#[async_trait]
impl<E, C, T> Query<C, Option<T>> for Optional<E>
where
    E: Debug + Endpoint + Send + Sync,
    C: Client + Debug + Sync,
    T: Debug + DeserializeOwned,
    E::Response<T>: DeserializeOwned + UnwrapResponse<T>,
{
    #[framed]
    #[instrument(err(Debug))]
    async fn query(&self, client: &C) -> Result<Option<T>, C::Error> {
        let uri = self.uri(client)?;
        let headers = self.headers();

        let request = self.request(client, &uri, &headers, None)?;

        let response = client.rest(request).await?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if status.is_client_error() || status.is_server_error() {
            return Err(C::Error::from(raw_response::<C>(response).await?));
        }

        Ok(Some(
            serde_json::from_slice::<E::Response<T>>(raw_response::<C>(response).await?.into_body().as_slice())
                .map_err(BaseClientError::ResponseBodyDeserialization)
                .map_err(C::Error::from)?
                .unwrap_response(),
        ))
    }
}

#[derive(Debug)]
pub struct Ignore<E>(pub E);

impl<E> Deref for Ignore<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<E: Endpoint> Ignore<E> {
    pub fn optional(self) -> Optional<Self> {
        Optional(self)
    }
}

#[async_trait]
impl<E, C> Query<C> for Ignore<E>
where
    E: Debug + Endpoint + Send + Sync,
    C: Client + Debug + Sync,
{
    #[framed]
    #[instrument(err(Debug))]
    async fn query(&self, client: &C) -> Result<(), C::Error> {
        let uri = self.uri(client)?;
        let headers = self.headers();

        let request = self.request(client, &uri, &headers, None)?;

        let response = client.rest(request).await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            return Err(C::Error::from(raw_response::<C>(response).await?));
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Raw<E>(pub E);

impl<E> Deref for Raw<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<E, C> Query<C, Response<Vec<u8>>> for Raw<E>
where
    E: Debug + Endpoint + Send + Sync,
    C: Client + Debug + Sync,
{
    #[framed]
    #[instrument(err(Debug))]
    async fn query(&self, client: &C) -> Result<Response<Vec<u8>>, C::Error> {
        let uri = self.uri(client)?;
        let headers = self.headers();

        let request = self.request(client, &uri, &headers, None)?;

        let response = client.rest(request).await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            return Err(C::Error::from(raw_response::<C>(response).await?));
        }

        raw_response::<C>(response).await
    }
}

#[allow(unused)]
pub trait Pageable: Endpoint {
    fn next_page(prev_response: &Response<Vec<u8>>) -> Option<NextPage>;
}

#[derive(Clone, Debug)]
pub enum NextPage {
    FullUri(Uri),
}

#[derive(Clone, Debug, Default)]
pub enum Pagination {
    #[default]
    All,
    Limit(usize),
}

#[derive(Debug)]
pub struct Paged<E> {
    endpoint: E,
    pagination: Pagination,
}

impl<E> Paged<E> {
    pub fn new(pagination: Pagination, endpoint: E) -> Self {
        Paged { endpoint, pagination }
    }

    pub fn all(endpoint: E) -> Self {
        Paged {
            pagination: Pagination::All,
            endpoint,
        }
    }

    pub fn limit(limit: usize, endpoint: E) -> Self {
        Paged {
            pagination: Pagination::Limit(limit),
            endpoint,
        }
    }
}

#[async_trait]
impl<E, T, C> Query<C, Vec<T>> for Paged<E>
where
    E: Debug + Endpoint + Pageable + Send + Sync,
    T: Debug + DeserializeOwned + Send,
    C: Client + Debug + Sync,
    E::Response<Vec<T>>: DeserializeOwned + UnwrapResponse<Vec<T>>,
{
    #[framed]
    #[instrument(err(Debug))]
    async fn query(&self, client: &C) -> Result<Vec<T>, C::Error> {
        let (limit, mut all_results): (usize, Vec<T>) = match &self.pagination {
            Pagination::All => (usize::MAX, vec![]),
            Pagination::Limit(limit) => (*limit, Vec::with_capacity(*limit)),
        };

        let uri = self.endpoint.uri(client)?;
        let headers = self.endpoint.headers();

        let request = self.endpoint.request(client, &uri, &headers, None)?;

        let response = raw_response::<C>(client.rest(request).await?).await?;

        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            return Err(C::Error::from(response));
        }

        let mut results = serde_json::from_slice::<E::Response<Vec<T>>>(response.body().as_slice())
            .map_err(BaseClientError::ResponseBodyDeserialization)
            .map_err(C::Error::from)?
            .unwrap_response();

        all_results.append(&mut results);

        let mut prev_response = response;

        while all_results.len() < limit {
            let next_page = match E::next_page(&prev_response) {
                Some(next_page) => next_page,
                None => break,
            };

            let request = self.endpoint.request(client, &uri, &headers, Some(next_page))?;

            let response = raw_response::<C>(client.rest(request).await?).await?;

            let status = response.status();
            if status.is_client_error() || status.is_server_error() {
                return Err(C::Error::from(response));
            }

            let mut results = serde_json::from_slice::<E::Response<Vec<T>>>(response.body().as_slice())
                .map_err(BaseClientError::ResponseBodyDeserialization)
                .map_err(C::Error::from)?
                .unwrap_response();

            all_results.append(&mut results);

            prev_response = response;
        }

        Ok(all_results)
    }
}

impl<E> Deref for Paged<E> {
    type Target = E;
    fn deref(&self) -> &Self::Target {
        &self.endpoint
    }
}

async fn raw_response<C>(response: Response<Body>) -> Result<Response<Vec<u8>>, C::Error>
where
    C: Client,
{
    let (parts, body) = response.into_parts();
    let bytes = to_bytes(body)
        .await
        .map(|bytes| bytes.to_vec())
        .map_err(BaseClientError::ResponseBodyInvalidCharacter)?;
    Ok(Response::from_parts(parts, bytes))
}

pub trait UnwrapResponse<T> {
    fn unwrap_response(self) -> T;
}

#[derive(Clone, Debug)]
pub struct DefaultResponse<T>(T);

impl<'de, T> Deserialize<'de> for DefaultResponse<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(DefaultResponse(<T as Deserialize>::deserialize(deserializer)?))
    }
}

impl<T> UnwrapResponse<T> for DefaultResponse<T> {
    fn unwrap_response(self) -> T {
        self.0
    }
}

#[async_trait]
impl<E, C> Query<C> for Optional<Ignore<E>>
where
    E: Debug + Endpoint + Send + Sync,
    C: Client + Debug + Sync,
{
    #[framed]
    #[instrument(err(Debug))]
    async fn query(&self, client: &C) -> Result<(), C::Error> {
        let uri = self.uri(client)?;
        let headers = self.headers();

        let request = self.request(client, &uri, &headers, None)?;

        let response = client.rest(request).await?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(());
        }
        if status.is_client_error() || status.is_server_error() {
            return Err(C::Error::from(raw_response::<C>(response).await?));
        }

        Ok(())
    }
}

#[async_trait]
impl<E, C> Query<C> for Ignore<Optional<E>>
where
    E: Debug + Endpoint + Send + Sync,
    C: Client + Debug + Sync,
{
    #[framed]
    #[instrument(err(Debug))]
    async fn query(&self, client: &C) -> Result<(), C::Error> {
        let uri = self.uri(client)?;
        let headers = self.headers();

        let request = self.request(client, &uri, &headers, None)?;

        let response = client.rest(request).await?;

        let status = response.status();
        if status == StatusCode::NOT_FOUND {
            return Ok(());
        }
        if status.is_client_error() || status.is_server_error() {
            return Err(C::Error::from(raw_response::<C>(response).await?));
        }

        Ok(())
    }
}
