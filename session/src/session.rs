use chrono::{Duration, NaiveDateTime};
use std::ops::Deref;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session<T> {
    #[serde(skip)]
    pub session_id: Uuid,
    pub created_at: NaiveDateTime,
    pub value: T,
    #[serde(skip)]
    pub max_age: Option<Duration>,
    #[serde(skip)]
    pub expires: Option<NaiveDateTime>,
}

impl<T> Session<T> {
    pub fn map<U>(self, map_fn: impl FnOnce(T) -> U) -> Session<U> {
        Session {
            session_id: self.session_id,
            created_at: self.created_at,
            value: map_fn(self.value),
            max_age: self.max_age,
            expires: self.expires,
        }
    }

    pub fn try_map<U, E>(self, try_map_fn: impl FnOnce(T) -> Result<U, E>) -> Result<Session<U>, E> {
        Ok(Session {
            session_id: self.session_id,
            created_at: self.created_at,
            value: try_map_fn(self.value)?,
            max_age: self.max_age,
            expires: self.expires,
        })
    }
}

impl<T> Deref for Session<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

pub trait RawSession<ParsedSession>: Sized {
    type Key;
    type Validation;
    fn try_decode(self, key: &Self::Key, validation: &Self::Validation) -> Result<ParsedSession, anyhow::Error>;
    fn add_extensions(
        session: Result<Option<Self>, anyhow::Error>,
        key: &Self::Key,
        validation: &Self::Validation,
        extensions: &mut http::Extensions,
    );
}

impl<T: Send + Sync + 'static> RawSession<T> for T {
    type Key = ();
    type Validation = ();
    fn try_decode(self, _: &Self::Key, _: &Self::Validation) -> Result<T, anyhow::Error> {
        Ok(self)
    }
    fn add_extensions(
        session: Result<Option<Self>, anyhow::Error>,
        _: &Self::Key,
        _: &Self::Validation,
        extensions: &mut http::Extensions,
    ) {
        match session {
            Ok(Some(session)) => extensions.insert(Some(session)),
            _ => extensions.insert(None::<T>),
        };
    }
}

#[derive(Clone, Debug)]
pub enum RequestSession<T> {
    None,
    SessionId(Uuid),
    Session(Session<T>),
}

#[cfg(feature = "axum-core-02")]
#[async_trait]
impl<B, T> axum_core_02::extract::FromRequest<B> for Session<T>
where
    B: Send,
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    type Rejection = http::StatusCode;

    async fn from_request(req: &mut axum_core_02::extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        req.extensions_mut()
            .remove::<Option<Session<T>>>()
            .flatten()
            .ok_or_else(|| {
                log::error!("tried to extract session::Session from request when there was none, use axum::extract::Extract<Option<session::Session<T>>> instead of axum::extract::Extract<session::Session<T>> for correct extraction session from requests");
                http::StatusCode::INTERNAL_SERVER_ERROR
            })
    }
}

#[cfg(feature = "axum-core-03")]
#[async_trait]
impl<S, B, T> axum_core_03::extract::FromRequest<S, B> for Session<T>
where
    S: Send + Sync,
    B: Send + 'static,
    T: serde::de::DeserializeOwned + Send + Sync + 'static,
{
    type Rejection = http::StatusCode;

    async fn from_request(mut req: http::Request<B>, _: &S) -> Result<Self, Self::Rejection> {
        req.extensions_mut()
            .remove::<Option<Session<T>>>()
            .flatten()
            .ok_or_else(|| {
                log::error!("tried to extract session::Session from request when there was none, use axum::extract::Extract<Option<session::Session<T>>> instead of axum::extract::Extract<session::Session<T>> for correct extraction session from requests");
                http::StatusCode::INTERNAL_SERVER_ERROR
            })
    }
}
