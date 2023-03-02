use crate::db::DbPool;
use crate::service::{AccountSession, Ctx, CtxOptSession};
use authzen::service_util::Error;
use axum::extract::Extension;
use axum::routing::method_routing::{get, post};
use axum::{Json, Router};
use http::response::Parts;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/cart", get(my_cart))
            .route("/item", post(create_item))
            .route("/add-cart-item", post(add_item_to_cart))
            .route("/sign-up", post(sign_up)),
    )
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Clients {
    #[derivative(Debug = "ignore")]
    pub db: crate::DbPool,
    #[derivative(Debug = "ignore")]
    pub opa_client: authzen::decision_makers::opa::OPAClient,
    #[derivative(Debug = "ignore")]
    pub session_store: session_util::DynAccountSessionStore,
    #[derivative(Debug = "ignore")]
    pub tx_cache_client: ApiTxCacheClient,
}

#[derive(Clone)]
pub struct ApiTxCacheClient {
    pub db: mongodb::Database,
    pub collection: authzen::transaction_caches::mongodb::MongodbTxCollection,
}

impl Clients {
    fn ctx<'a>(&'a self, session: &'a Option<AccountSession>) -> Result<Ctx<'a, &'a DbPool>, Error> {
        let session = session
            .as_ref()
            .ok_or_else(|| Error::bad_request_msg("must be signed in"))?;
        Ok(Ctx {
            session,
            db: &self.db,
            opa_client: &self.opa_client,
            mongodb_client: &self.tx_cache_client.collection,
        })
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

#[instrument]
async fn sign_up(
    Extension(session): Extension<Option<AccountSession>>,
    Extension(clients): Extension<Clients>,
    Json(sign_up_post): Json<SignUpPost>,
) -> Result<(Parts, Json<Account>), Error> {
    let identifier = match sign_up_post {
        SignUpPost::Email(email) => crate::db::Identifier::Email(email),
        SignUpPost::Username(username) => crate::db::Identifier::Username(username),
    };
    let (parts, db_account) =
        crate::service::sign_up(&clients.ctx_opt_session(&session), &clients.session_store, identifier).await?;
    Ok((parts, Json(Account { id: db_account.id })))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "camelCase")]
struct CreateItemPost {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "camelCase")]
struct Item {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

#[instrument]
async fn create_item(
    Extension(session): Extension<Option<AccountSession>>,
    Extension(clients): Extension<Clients>,
    Json(CreateItemPost { name, description }): Json<CreateItemPost>,
) -> Result<Json<Item>, Error> {
    let db_item = crate::service::create_item(&clients.ctx_opt_session(&session), name, description).await?;
    Ok(Json(Item {
        id: db_item.id,
        name: db_item.name,
        description: db_item.description,
    }))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename = "camelCase")]
struct CartItemPost {
    pub item_id: Uuid,
}

#[instrument]
async fn add_item_to_cart(
    Extension(session): Extension<Option<AccountSession>>,
    Extension(clients): Extension<Clients>,
    Json(CartItemPost { item_id }): Json<CartItemPost>,
) -> Result<(), Error> {
    crate::service::add_item_to_cart(&clients.ctx(&session)?, item_id).await?;
    Ok(())
}

#[instrument]
async fn my_cart(
    Extension(session): Extension<Option<AccountSession>>,
    Extension(clients): Extension<Clients>,
) -> Result<Json<Vec<Item>>, Error> {
    let db_cart_items = crate::service::my_cart(&clients.ctx(&session)?).await?;
    Ok(Json(
        db_cart_items
            .into_iter()
            .map(|x| Item {
                id: x.id,
                name: x.name,
                description: x.description,
            })
            .collect(),
    ))
}
