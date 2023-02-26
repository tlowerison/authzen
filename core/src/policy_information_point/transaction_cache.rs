use crate::policy_information_point::*;
use crate::*;
use ::futures::future::{BoxFuture, FutureExt};
use ::serde::de::DeserializeOwned;

pub trait GetTransactionValues<SC, TC, Ctx>: Identifiable + Sized
where
    SC: StorageClient,
    TC: TransactionCache,
{
    fn get_transaction_values<'life0, 'async_trait>(
        ctx: &'life0 Ctx,
    ) -> BoxFuture<'async_trait, Result<HashMap<Self::Id, TxCacheEntity<Self, Self::Id>>, TC::Error>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
        SC: 'async_trait + 'life0,
        TC: 'async_trait,
        Ctx: 'async_trait;
}

impl<T, SC, TC, Ctx> GetTransactionValues<SC, TC, Ctx> for T
where
    T: DeserializeOwned + ObjectType + Identifiable + Send,
    SC: StorageClient + Send + Sync,
    TC: TransactionCache + Sync,
    Ctx: AsRef<SC> + AsRef<TC> + Sync,
{
    fn get_transaction_values<'life0, 'async_trait>(
        ctx: &'life0 Ctx,
    ) -> BoxFuture<'async_trait, Result<HashMap<Self::Id, TxCacheEntity<Self, Self::Id>>, TC::Error>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
        SC: 'async_trait + 'life0,
        TC: 'async_trait,
        Ctx: 'async_trait,
    {
        if let Some(transaction_id) = AsRef::<SC>::as_ref(ctx).transaction_id() {
            <TC as TransactionCache>::get_entities::<T, T, SC::TransactionId<'life0>>(ctx.as_ref(), transaction_id)
                .boxed()
        } else {
            async move { Ok(Default::default()) }.boxed()
        }
    }
}
