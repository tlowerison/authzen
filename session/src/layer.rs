use crate::*;
use data_encoding::BASE64;
use futures::future::{BoxFuture, FutureExt};
use http::{header::COOKIE, Request, Response};
use ring::hmac::verify;
use serde::{de::DeserializeOwned, Serialize};
use std::marker::PhantomData;
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;
use uuid::Uuid;

#[derive(Clone)]
pub struct SessionService<I, P, S, K, V> {
    pub inner: I,
    pub layer: SessionLayer<P, S, K, V>,
}

pub struct SessionLayer<P, S, K, V> {
    pub key: K,
    pub validation: V,
    pub store: S,
    pub _encoded: PhantomData<P>,
}

impl<S: Clone, K: Clone, V: Clone, P> Clone for SessionLayer<P, S, K, V> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            validation: self.validation.clone(),
            store: self.store.clone(),
            _encoded: PhantomData,
        }
    }
}

impl<I, P, S: Clone, K: Clone, V: Clone> Layer<I> for SessionLayer<P, S, K, V> {
    type Service = SessionService<I, P, S, K, V>;
    fn layer(&self, inner: I) -> Self::Service {
        SessionService {
            layer: self.clone(),
            inner,
        }
    }
}

impl<P, S, K, V> SessionLayer<P, S, K, V>
where
    S: SessionStore,
    Session<<S as SessionStore>::Value>: RawSession<P, Key = K, Validation = V>,
{
    pub fn encoded(store: S, key: K, validation: V) -> Self {
        SessionLayer {
            store,
            key,
            validation,
            _encoded: PhantomData,
        }
    }
}

impl<P, S> SessionLayer<P, S, (), ()> {
    pub fn plain(store: S) -> Self {
        SessionLayer {
            store,
            key: (),
            validation: (),
            _encoded: PhantomData,
        }
    }
}

// TODO: reimplement with no clone or 'static bounds once Service::Future is generic
impl<ReqBody, ResBody, I, P, S, K, V, R> Service<Request<ReqBody>> for SessionService<I, P, S, K, V>
where
    I: Clone + Service<Request<ReqBody>, Response = Response<ResBody>> + Send + 'static,
    <I as Service<Request<ReqBody>>>::Future: Send,
    S: Clone + SessionStore<Value = R>,
    K: Clone + Send + 'static,
    V: Clone + Send + 'static,
    R: DeserializeOwned + Serialize + SessionValue<ReqBody, S> + Send + Sync + 'static,
    P: Clone + Send + Sync + 'static,
    Session<R>: std::fmt::Debug + RawSession<P, Key = K, Validation = V>,
    ReqBody: Send + Sync + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = I::Error;
    type Future = BoxFuture<'static, Result<Response<ResBody>, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // Our timeout service is ready if the inner service is ready.
        // This is how backpressure can be propagated through a tree of nested services.
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let request_session = R::get_unparsed_request_session(&self.layer.store, &req);

        let Self { mut inner, layer } = self.clone();
        let SessionLayer {
            key, validation, store, ..
        } = layer;

        async move {
            match request_session {
                Ok(RequestSession::None) => Ok(None),
                Ok(RequestSession::SessionId(session_id)) => store.get(&session_id).await.map(Some),
                Ok(RequestSession::Session(session)) => Ok(Some(session)),
                Err(err) => Err(err),
            }
        }
        .map(move |session| {
            Session::<R>::add_extensions(session, &key, &validation, req.extensions_mut());
            ResponseFuture::future(inner.call(req))
        })
        .flatten()
        .boxed()
    }
}

pub(crate) fn get_session_id_from_request<S: SessionStore, ReqBody>(store: &S, req: &Request<ReqBody>) -> Option<Uuid> {
    req.headers()
        .get_all(COOKIE)
        .iter()
        .filter_map(|header| header.to_str().ok())
        .flat_map(|cookie_str| {
            cookie_str
                .split(';')
                .filter_map(|x| cookie::Cookie::parse_encoded(x.trim()).ok())
        })
        .find_map(|cookie| {
            let (name, value) = cookie.name_value();
            if name != store.key_name() {
                return None;
            }
            let cookie_value: CookieValue = serde_plain::from_str(value).ok()?;

            let signature = BASE64.decode(cookie_value.signature.as_bytes()).ok()?;
            verify(store.key(), cookie_value.id.as_bytes(), &signature).ok()?;

            Some(cookie_value.id)
        })
}
