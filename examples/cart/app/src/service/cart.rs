use crate::*;
use authzen::actions::*;
use authzen::decision_makers::opa::OPAClient;
use authzen::transaction_caches::mongodb::MongodbTxCollection;
use authzen::*;
use service_util::Error;
use uuid::Uuid;

pub async fn add_item_to_cart<D: Db>(
    ctx: CtxOptSession<'_, D>,
    cart_id: Uuid,
    item_id: Uuid,
) -> Result<DbCartItem, Error> {
    let db_cart_item = DbCartItem::builder().cart_id(cart_id).item_id(item_id).build();
    Ok(CartItem::try_create_one(&ctx, db_cart_item).await?)
}

// try_create should typically handle all your needs for handling
// both authorization queries and performing the actual action
// that is being authorized, however, in the case where you have
// some context which implements `CanContext<DecisionMaker>` for
// for multiple `DecisionMaker`s or implements `TryContext<DecisionMaker, StorageClient>`
// for multiple `StorageClient`s, you'll be able to explicitly specify those parameters
// as function generics
pub async fn add_item_to_cart_explicitly<D: Db>(
    ctx: Ctx<'_, D>,
    cart_id: Uuid,
    item_id: Uuid,
) -> Result<DbCartItem, Error> {
    let db_cart_item = DbCartItem::builder().cart_id(cart_id).item_id(item_id).build();
    CartItem::can_create::<&OPAClient, D, &MongodbTxCollection, _>(&ctx, &[&db_cart_item]).await?;
    CartItem::try_create::<&OPAClient, D, &MongodbTxCollection, _>(&ctx, [db_cart_item]).await?;
    todo!()
}