use crate::prelude::*;
use crate::Ctx;
use cart_app::Account;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ExamplesCartRequest {
    Account(AccountRequest),
    Cart(CartRequest),
    CartItem(CartItemRequest),
    Item(ItemRequest),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountRequest {
    Id(Uuid),
    Ids(Vec<Uuid>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CartRequest {
    Id(Uuid),
    Ids(Vec<Uuid>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CartItemRequest {
    Id(Uuid),
    Ids(Vec<Uuid>),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemRequest {
    Id(Uuid),
    Ids(Vec<Uuid>),
}

#[async_trait]
impl ObjectQuery<Ctx, DbPool, MongodbTxCollection> for AccountRequest {
    type Object = Account<'static>;
    type Error = Error;
    #[instrument(name = "AccountRequest::handle", err(Debug), skip(ctx))]
    async fn fetch(
        self,
        ctx: &Ctx,
    ) -> Result<Vec<<Self::Object as AsStorage<<DbPool as StorageClient>::Backend>>::StorageObject>, Self::Error> {
        Ok(match self {
            Self::Id(id) => DbAccount::get(&ctx.db, [id]).await?,
            Self::Ids(ids) => DbAccount::get(&ctx.db, ids).await?,
        })
    }
}

#[async_trait]
impl ObjectQuery<Ctx, DbPool, MongodbTxCollection> for CartRequest {
    type Object = Cart<'static>;
    type Error = Error;
    #[instrument(name = "CartRequest::handle", err(Debug), skip(ctx))]
    async fn fetch(
        self,
        ctx: &Ctx,
    ) -> Result<Vec<<Self::Object as AsStorage<<DbPool as StorageClient>::Backend>>::StorageObject>, Self::Error> {
        Ok(match self {
            Self::Id(id) => DbCart::get(&ctx.db, [id]).await?,
            Self::Ids(ids) => DbCart::get(&ctx.db, ids).await?,
        })
    }
}

#[async_trait]
impl ObjectQuery<Ctx, DbPool, MongodbTxCollection> for CartItemRequest {
    type Object = CartItem<'static>;
    type Error = Error;
    #[instrument(name = "CartItemRequest::handle", err(Debug), skip(ctx))]
    async fn fetch(
        self,
        ctx: &Ctx,
    ) -> Result<Vec<<Self::Object as AsStorage<<DbPool as StorageClient>::Backend>>::StorageObject>, Self::Error> {
        Ok(match self {
            Self::Id(id) => DbCartItem::get(&ctx.db, [id]).await?,
            Self::Ids(ids) => DbCartItem::get(&ctx.db, ids).await?,
        })
    }
}

#[async_trait]
impl ObjectQuery<Ctx, DbPool, MongodbTxCollection> for ItemRequest {
    type Object = Item<'static>;
    type Error = Error;
    #[instrument(name = "ItemRequest::handle", err(Debug), skip(ctx))]
    async fn fetch(
        self,
        ctx: &Ctx,
    ) -> Result<Vec<<Self::Object as AsStorage<<DbPool as StorageClient>::Backend>>::StorageObject>, Self::Error> {
        Ok(match self {
            Self::Id(id) => DbItem::get(&ctx.db, [id]).await?,
            Self::Ids(ids) => DbItem::get(&ctx.db, ids).await?,
        })
    }
}
