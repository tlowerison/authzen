use crate::db::DbPool;
use crate::service::{AccountSession, Ctx, CtxOptSession};
use authzen::service_util::{from_body, Error};
use axum::extract::{Extension, RawBody};
use axum::routing::method_routing::post;
use axum::Router;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router {
    Router::new().nest("/api", Router::new().route("/cart-item", post(add_item_to_cart)))
}

#[derive(Clone, Debug)]
pub struct Clients {
    pub account_session_store: session_util::DynAccountSessionStore,
    pub db: crate::DbPool,
    pub opa_client: authzen::decision_makers::opa::OPAClient,
    pub tx_cache_client: ApiTxCacheClient,
}

#[derive(Clone, Debug)]
pub struct ApiTxCacheClient {
    pub db: mongodb::Database,
    pub collection: authzen::transaction_caches::mongodb::MongodbTxCollection,
}

impl Clients {
    fn ctx<'a>(&'a self, session: &'a AccountSession) -> Ctx<'a, &'a DbPool> {
        Ctx {
            session,
            db: &self.db,
            opa_client: &self.opa_client,
            mongodb_client: &self.tx_cache_client.collection,
        }
    }
    fn ctx_opt_session<'a>(&'a self, session: &'a Option<AccountSession>) -> CtxOptSession<'a, &'a DbPool> {
        CtxOptSession {
            session: session.as_ref(),
            db: &self.db,
            opa_client: &self.opa_client,
            mongodb_client: &self.tx_cache_client.collection,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CartItemPost {
    pub cart_id: Uuid,
    pub item_id: Uuid,
}

async fn add_item_to_cart(
    Extension(session): Extension<Option<AccountSession>>,
    Extension(clients): Extension<Clients>,
    raw_body: RawBody,
) -> Result<(), Error> {
    let CartItemPost { cart_id, item_id } = from_body(raw_body).await?;
    crate::service::add_item_to_cart(clients.ctx_opt_session(&session), cart_id, item_id).await?;
    Ok(())
}
