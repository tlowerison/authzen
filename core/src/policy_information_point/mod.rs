#[cfg(feature = "policy-information-point-server")]
mod server;
mod transaction_cache;

#[cfg(feature = "policy-information-point-server")]
pub use server::*;
pub use transaction_cache::*;

use crate::*;
use ::authzen_service_util::try_join_safe;
use ::futures::future::TryFutureExt;
use ::http::header::{HeaderMap, HeaderName};
use ::serde::Serialize;
use ::std::collections::HashMap;
use ::std::fmt::Debug;

pub static X_TRANSACTION_ID: HeaderName = HeaderName::from_static("x-transaction-id");

#[derive(Clone, Copy, Debug, From)]
pub struct TransactionId<Id>(pub Id);

#[derive(Debug, Error)]
pub enum QueryError<E> {
    Deserialization(authzen_service_util::Error),
    Query(E),
    Serialization(serde_json::Error),
}

#[async_trait]
pub trait Query<Ctx>: Sized {
    type Error: Debug;
    async fn fetch(self, ctx: &Ctx) -> Result<Response, QueryError<Self::Error>>;
}

#[async_trait]
pub trait ObjectQuery<Ctx, DS, TC>: Sized
where
    DS: DataSource + Send + Sync,
    TC: TransactionCache + Sync,
    Ctx: AsRef<DS> + AsRef<TC> + Sync,
    TC::Error: Into<Self::Error>,
    <Self::Object as AsStorage<<DS as DataSource>::Backend>>::StorageObject: Into<Self::Object>,
{
    type Object: AsStorage<<DS as DataSource>::Backend>
        + GetTransactionValues<DS, TC, Ctx>
        + Identifiable
        + Send
        + Serialize;
    type Error: Debug + Send;

    async fn fetch(
        self,
        ctx: &Ctx,
    ) -> Result<Vec<<Self::Object as AsStorage<DS::Backend>>::StorageObject>, Self::Error>;

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
