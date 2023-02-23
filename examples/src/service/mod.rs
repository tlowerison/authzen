pub mod account;
pub mod cart;
pub mod context;
pub mod item;

pub use account::*;
pub use cart::*;
pub use context::*;
pub use item::*;

use crate::*;
use authzen::*;
use service_util::Error;
use uuid::Uuid;

pub async fn do_things<D: Db>(ctx: Ctx<'_, D>, cart_id: Uuid, item_id: Uuid) -> Result<DbCartItem, Error> {
    let db_cart_item = DbCartItem::builder().cart_id(cart_id).item_id(item_id).build();

    CartItem::can_do_a_thing(&ctx, &[&db_cart_item]).await?;

    let created_then_deleted_cart_items = CartItem::try_create_then_delete(&ctx, [db_cart_item])
        .await?
        .pop()
        .ok_or_else(|| Error::default_details("expected to have created and deleted a cart item"))?;
    Ok(created_then_deleted_cart_items)
}
