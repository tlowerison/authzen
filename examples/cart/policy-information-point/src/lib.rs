#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate tracing;

pub mod env;
mod examples_cart;
mod prelude;

use examples_cart::*;

use crate::prelude::*;
use authzen::transaction_caches::mongodb::MongodbTxCollection;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Clients {
    #[derivative(Debug = "ignore")]
    pub db: DbPool,
    #[derivative(Debug = "ignore")]
    pub tx_cache_client: MongodbTxCollection,
}

#[derive(Clone, Derivative)]
pub struct Ctx {
    #[derivative(Debug = "ignore")]
    pub db: DbPool,
    #[derivative(Debug = "ignore")]
    pub tx_cache_client: MongodbTxCollection,
    pub transaction_id: Option<Uuid>,
}

impl AsRef<DbPool> for Ctx {
    fn as_ref(&self) -> &DbPool {
        &self.db
    }
}

impl AsRef<MongodbTxCollection> for Ctx {
    fn as_ref(&self) -> &MongodbTxCollection {
        &self.tx_cache_client
    }
}

// required for Ctx to be derived from Clients and an optional transaction id in the main endpoint
// to be passed to the query using the provided server function from authzen
impl From<(Clients, Option<Uuid>)> for Ctx {
    fn from(value: (Clients, Option<Uuid>)) -> Self {
        Self {
            db: value.0.db,
            tx_cache_client: value.0.tx_cache_client,
            transaction_id: value.1,
        }
    }
}

/// top level enum for matching the different
/// services supported by this policy information point
/// in this examples case, it's just the service "examples_cart"
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "service")]
pub enum Request {
    ExamplesCart(examples_cart::ExamplesCartRequest),
}

#[async_trait]
impl Query<Ctx> for Request {
    type Error = Error;
    async fn fetch(self, ctx: &Ctx) -> Result<Response, QueryError<Self::Error>> {
        Ok(match self {
            Self::ExamplesCart(req) => match req {
                ExamplesCartRequest::Account(req) => req.fetch_with_tx_data(ctx).await?,
                ExamplesCartRequest::Cart(req) => req.fetch_with_tx_data(ctx).await?,
                ExamplesCartRequest::CartItem(req) => req.fetch_with_tx_data(ctx).await?,
                ExamplesCartRequest::Item(req) => req.fetch_with_tx_data(ctx).await?,
            },
        })
    }
}
