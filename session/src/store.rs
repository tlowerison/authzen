use crate::*;
use anyhow::Error;
use chrono::{NaiveDateTime, Utc};
use http::header::{HeaderValue, SET_COOKIE};
use http::{Request, Response};
use hyper::Body;
use ring::hmac::Key;
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, ops::Deref, sync::Arc};
use uuid::Uuid;

pub type DynSessionStore<T> = Arc<dyn SessionStore<Value = T>>;

#[async_trait]
pub trait SessionStore: 'static + Send + Sync + Debug {
    type Value: Clone + DeserializeOwned + Serialize + Send + Sync;

    fn into_dyn(self) -> DynSessionStore<Self::Value>
    where
        Self: Sized,
    {
        Arc::new(self) as DynSessionStore<Self::Value>
    }
    fn key(&self) -> &Key;
    fn key_name(&self) -> &str;

    async fn set(&self, prefix: Option<String>, session_id: &Uuid, session: &Session<Self::Value>)
        -> Result<(), Error>;
    async fn get(&self, session_id: &Uuid) -> Result<Session<Self::Value>, Error>;
    async fn delete(&self, session_id: &Uuid) -> Result<(), Error>;

    async fn store_session_and_set_cookie(
        &self,
        res: &mut Response<Body>,
        cookie_config: CookieConfig<'_, Self::Value>,
        prefix: Option<String>,
    ) -> Result<(), Error> {
        let cookie_value = CookieValue::new(self.key())?;
        let session = Session {
            session_id: cookie_value.id,
            created_at: Utc::now().naive_utc(),
            value: cookie_config.value.clone(),
            max_age: cookie_config.max_age,
            expires: cookie_config.expires,
        };

        // for cookie formatting standards, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie
        let mut cookie = format!(
            "{}={}; SameSite={}",
            self.key_name(),
            cookie_value.encode(),
            cookie_config.same_site
        );
        if cookie_config.http_only {
            cookie = format!("{cookie}; HttpOnly");
        }
        if cookie_config.secure {
            cookie = format!("{cookie}; Secure");
        }
        if let Some(domain) = cookie_config.domain {
            cookie = format!("{cookie}; Domain={domain}");
        }
        if let Some(path) = cookie_config.path {
            cookie = format!("{cookie}; Path={path}");
        }
        if let Some(max_age) = cookie_config.max_age {
            cookie = format!("{cookie}; Max-Age={}", max_age.num_seconds());
        }
        if let Some(expires) = cookie_config.expires {
            // for chrono formatting escape sequences, see https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html
            // for date formatting standards in http headers, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Date
            cookie = format!("{cookie}; Expires={}", expires.format("%a, %d %b %Y %H:%M:%S GMT"));
        }

        let header_value = HeaderValue::from_str(&cookie).map_err(Error::msg)?;

        self.set(prefix, &session.session_id, &session).await?;
        res.headers_mut().append(SET_COOKIE, header_value);
        Ok(())
    }

    async fn delete_session(
        &self,
        res: &mut Response<Body>,
        cookie_config: CookieConfig<'_, ()>,
        session_id: Option<&Uuid>,
    ) -> Result<(), Error> {
        // for cookie formatting standards, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie
        let mut cookie = format!("{}=; SameSite={}", self.key_name(), cookie_config.same_site);
        if cookie_config.http_only {
            cookie = format!("{cookie}; HttpOnly");
        }
        if cookie_config.secure {
            cookie = format!("{cookie}; Secure");
        }
        if let Some(domain) = cookie_config.domain {
            cookie = format!("{cookie}; Domain={domain}");
        }
        if let Some(path) = cookie_config.path {
            cookie = format!("{cookie}; Path={path}");
        }

        // for chrono formatting escape sequences, see https://docs.rs/chrono/0.4.19/chrono/format/strftime/index.html
        // for date formatting standards in http headers, see https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Date
        cookie = format!(
            "{cookie}; Expires={}",
            NaiveDateTime::from_timestamp_opt(0, 0)
                .unwrap()
                .format("%a, %d %b %Y %H:%M:%S GMT")
        );

        let header_value = HeaderValue::from_str(&cookie).map_err(Error::msg)?;

        if let Some(session_id) = session_id {
            self.delete(session_id).await?;
        }
        res.headers_mut().append(SET_COOKIE, header_value);
        Ok(())
    }
}

#[async_trait]
impl<S: SessionStore + ?Sized, Wrapper: 'static + Debug + Deref<Target = S> + Send + Sync> SessionStore for Wrapper {
    type Value = S::Value;

    fn key(&self) -> &Key {
        self.deref().key()
    }
    fn key_name(&self) -> &str {
        self.deref().key_name()
    }
    async fn set(
        &self,
        prefix: Option<String>,
        session_id: &Uuid,
        session: &Session<Self::Value>,
    ) -> Result<(), Error> {
        self.deref().set(prefix, session_id, session).await
    }
    async fn get(&self, session_id: &Uuid) -> Result<Session<Self::Value>, Error> {
        self.deref().get(session_id).await
    }
    async fn delete(&self, session_id: &Uuid) -> Result<(), Error> {
        self.deref().delete(session_id).await
    }
    async fn store_session_and_set_cookie(
        &self,
        res: &mut Response<Body>,
        cookie_config: CookieConfig<'_, Self::Value>,
        prefix: Option<String>,
    ) -> Result<(), Error> {
        self.deref()
            .store_session_and_set_cookie(res, cookie_config, prefix)
            .await
    }
    async fn delete_session(
        &self,
        res: &mut Response<Body>,
        cookie_config: CookieConfig<'_, ()>,
        session_id: Option<&Uuid>,
    ) -> Result<(), Error> {
        self.deref().delete_session(res, cookie_config, session_id).await
    }
}

pub trait SessionValue<ReqBody: Sync, S: SessionStore> {
    fn get_unparsed_request_session(store: &S, req: &Request<ReqBody>) -> Result<RequestSession<S::Value>, Error> {
        match get_session_id_from_request(store, req) {
            Some(session_id) => Ok(RequestSession::SessionId(session_id)),
            None => Ok(RequestSession::None),
        }
    }
}
