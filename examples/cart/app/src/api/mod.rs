use crate::db::DbPool;
use crate::service::{AccountSession, Ctx, CtxOptSession};
use authzen::service_util::{from_body, Error};
use axum::extract::{Extension, RawBody};
use axum::routing::method_routing::post;
use axum::{Json, Router};
use http::response::Parts;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/item", post(create_item))
            .route("/add-cart-item", post(add_item_to_cart))
            .route("/sign-up", post(sign_up)),
    )
}

#[derive(Clone, Debug)]
pub struct Clients {
    pub db: crate::DbPool,
    pub opa_client: authzen::decision_makers::opa::OPAClient,
    pub session_store: session_util::DynAccountSessionStore,
    pub tx_cache_client: ApiTxCacheClient,
}

#[derive(Clone, Debug)]
pub struct ApiTxCacheClient {
    pub db: mongodb::Database,
    pub collection: authzen::transaction_caches::mongodb::MongodbTxCollection,
}

impl Clients {
    #[allow(unused)]
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
#[serde(rename_all = "snake_case")]
enum SignUpPost {
    Email(String),
    Username(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Account {
    pub id: Uuid,
}

async fn sign_up(
    Extension(session): Extension<Option<AccountSession>>,
    Extension(clients): Extension<Clients>,
    raw_body: RawBody,
) -> Result<(Parts, Json<Account>), Error> {
    let sign_up_post: SignUpPost = from_body(raw_body).await?;
    let identifier = match sign_up_post {
        SignUpPost::Email(email) => crate::db::Identifier::Email(email),
        SignUpPost::Username(username) => crate::db::Identifier::Username(username),
    };
    let (parts, db_account) =
        crate::service::sign_up(clients.ctx_opt_session(&session), &clients.session_store, identifier).await?;
    Ok((parts, Json(Account { id: db_account.id })))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "camelCase")]
struct CreateItemPost {
    pub name: String,
    pub description: Option<String>,
}

async fn create_item(
    Extension(session): Extension<Option<AccountSession>>,
    Extension(clients): Extension<Clients>,
    raw_body: RawBody,
) -> Result<(), Error> {
    let CreateItemPost { name, description } = from_body(raw_body).await?;
    crate::service::create_item(clients.ctx_opt_session(&session), name, description).await?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "camelCase")]
struct CartItemPost {
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
