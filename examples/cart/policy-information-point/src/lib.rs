#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate tracing;

mod examples_cart;
mod prelude;

use examples_cart::*;

use crate::prelude::*;
use authzen::transaction_caches::mongodb::MongodbTxCollection;

#[derive(Clone, Debug)]
pub struct Ctx {
    pub db: DbPool,
    pub tx_cache_db: mongodb::Database,
    pub tx_cache_client: MongodbTxCollection,
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
