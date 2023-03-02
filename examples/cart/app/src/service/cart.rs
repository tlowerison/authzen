use crate::*;
use authzen::actions::*;
use authzen::decision_makers::opa::OPAClient;
use authzen::storage_backends::diesel::operations::{DbGet, DbInsert};
use authzen::transaction_caches::mongodb::MongodbTxCollection;
use authzen::*;
use service_util::Error;
use uuid::Uuid;

#[instrument]
pub async fn add_item_to_cart<D: Db>(ctx: &Ctx<'_, D>, item_id: Uuid) -> Result<DbCartItem, Error> {
    let account_id = *ctx.session.account_id();

    let existing_cart = DbCart::get_unused(ctx, account_id).await?;

    let current_cart = match existing_cart {
        Some(cart) => cart,
        None => DbCart::insert_one(ctx, DbCart::builder().account_id(account_id).build()).await?,
    };

    Ok(CartItem::try_create_one(
        ctx,
        DbCartItem::builder().cart_id(current_cart.id).item_id(item_id).build(),
    )
    .await?)
}

#[instrument]
pub async fn my_cart<D: Db>(ctx: &Ctx<'_, D>) -> Result<Vec<DbItem>, Error> {
    let account_id = *ctx.session.account_id();

    let existing_cart = DbCart::get_unused(ctx, account_id).await?;

    let current_cart = match existing_cart {
        Some(cart) => cart,
        None => DbCart::insert_one(ctx, DbCart::builder().account_id(account_id).build()).await?,
    };

    let db_cart_items =
        DbCartItem::get_by_column(ctx, crate::db::schema::cart_item::cart_id, [current_cart.id]).await?;

    Ok(DbItem::get(ctx, db_cart_items.into_iter().map(|x| x.item_id)).await?)
}

// try_create should typically handle all your needs for handling
// both authorization queries and performing the actual action
// that is being authorized, however, in the case where you have
// some context which implements `CanContext<DecisionMaker>` for
// for multiple `DecisionMaker`s or implements `TryContext<DecisionMaker, StorageClient>`
// for multiple `StorageClient`s, you'll be able to explicitly specify those parameters
// as function generics
#[instrument]
pub async fn add_item_to_cart_explicitly<D: Db>(
    ctx: &Ctx<'_, D>,
    cart_id: Uuid,
    item_id: Uuid,
) -> Result<DbCartItem, Error> {
    let db_cart_item = DbCartItem::builder().cart_id(cart_id).item_id(item_id).build();
    CartItem::can_create::<&OPAClient, D, &MongodbTxCollection, _>(ctx, &[&db_cart_item]).await?;
    CartItem::try_create::<&OPAClient, D, &MongodbTxCollection, _>(ctx, [db_cart_item]).await?;
    todo!()
}
