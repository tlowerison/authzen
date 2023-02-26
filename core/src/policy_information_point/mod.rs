#[cfg(feature = "policy-information-point-server")]
mod server;
mod transaction_cache;

pub use server::*;
pub use transaction_cache::*;

use crate::*;
use ::futures::future::TryFutureExt;
use ::http::header::{HeaderMap, HeaderName, HeaderValue};
use ::hyper::StatusCode;
use ::serde::{de::DeserializeOwned, Serialize};
use ::service_util::try_join_safe;
use ::std::collections::HashMap;
use ::std::fmt::Debug;

pub static X_TRANSACTION_ID: HeaderName = HeaderName::from_static("x-transaction-id");

#[derive(Clone, Copy, Debug, From)]
pub struct TransactionId<Id>(pub Id);

#[derive(Debug, Error)]
pub enum QueryError<E> {
    Deserialization(service_util::Error),
    Query(E),
    Serialization(serde_json::Error),
}

#[async_trait]
pub trait Query<Ctx>: Sized {
    type Error: Debug;
    async fn fetch(self, ctx: &Ctx) -> Result<Response, QueryError<Self::Error>>;
}

#[async_trait]
pub trait ObjectQuery<Ctx, SC, TC>: Sized
where
    SC: StorageClient + Send + Sync,
    TC: TransactionCache + Sync,
    Ctx: AsRef<SC> + AsRef<TC> + Sync,
    TC::Error: Into<Self::Error>,
    <Self::Object as AsStorage<<SC as StorageClient>::Backend>>::StorageObject: Into<Self::Object>,
{
    type Object: AsStorage<<SC as StorageClient>::Backend>
        + GetTransactionValues<SC, TC, Ctx>
        + Identifiable
        + Send
        + Serialize;
    type Error: Debug + Send;

    async fn fetch(
        self,
        ctx: &Ctx,
    ) -> Result<Vec<<Self::Object as AsStorage<SC::Backend>>::StorageObject>, Self::Error>;

    async fn fetch_with_tx_data(self, ctx: &Ctx) -> Result<Response, QueryError<Self::Error>> {
        let (storage_values, transaction_values) = try_join_safe!(
            self.fetch(ctx).map_err(Into::<Self::Error>::into),
            Self::Object::get_transaction_values(ctx).map_err(Into::<Self::Error>::into)
        )
        .map_err(QueryError::Query)?;

        let values = storage_values
            .into_iter()
            .map(Into::into)
            .collect::<Vec<Self::Object>>();

        let mut values: HashMap<_, _> = values.iter().map(|value| (value.id(), value)).collect();

        for (id, value) in &transaction_values {
            if value.exists {
                values.insert(id, &value.value);
            } else {
                values.remove(&id);
            }
        }

        let headers = Self::headers(&values);
        let values = serde_json::to_vec(&values).map_err(QueryError::Serialization)?;
        Ok(Response { headers, values })
    }

    #[allow(unused_variables)]
    fn headers(values: &HashMap<&<Self::Object as Identifiable>::Id, &Self::Object>) -> HeaderMap {
        HeaderMap::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Response {
    pub values: Vec<u8>,
    pub headers: HeaderMap,
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
