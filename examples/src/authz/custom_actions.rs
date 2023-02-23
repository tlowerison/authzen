use authzen::storage_backends::diesel::prelude::*;
use authzen::transaction_caches::mongodb::*;
use authzen::*;
use diesel::backend::Backend;
use diesel::expression::Expression;
use diesel::expression_methods::ExpressionMethods;
use diesel::query_source::QuerySource;
use diesel::sql_types::SqlType;
use diesel::{query_builder::*, Identifiable};
use diesel::{Insertable, Table};
use diesel_async::methods::*;
use futures::future::BoxFuture;
use std::fmt::Debug;

// produces an action struct called `DoAThing`
// with ObjectType::TYPE == "do.a.thing"
action!(DoAThing = "do.a.thing");

// produces an action struct called `CreateThenDelete`
// with ObjectType::TYPE == "create_then_delete"
action!(CreateThenDelete);

/// note: the bounds on this trait look super freaky,
/// but they're all copied (mostly) directly from the authzen
/// implementation of `authzen::StorageAction<C, I>`
/// for `authzen::Create` and `authzen::Delete`
///
/// in general, if you're implementing StorageActions
/// to be used with diesel, you're going to run into
/// crazy trait bounds
#[async_trait]
impl<'query, 'v, E, B, I, C, O> StorageAction<C, I> for CreateThenDelete<O>
where
    O: AsStorage<B, StorageObject = E>,
    E: DbInsert + Sync,
    B: Backend,
    I: IntoIterator<Item = E::PostHelper<'v>> + Send,
    C: authzen::storage_backends::diesel::connection::Db<Backend = B>,

    // DbEntity bounds
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    DbEntityError<<E::Raw as TryInto<E>>::Error>: Debug + Send,

    // DbInsert bounds
    'v: 'query,
    I: 'v,
    C: 'query,
    <E::Raw as TryInto<E>>::Error: Send,
    Vec<E::Post<'v>>: Insertable<E::Table> + Send,
    <Vec<E::Post<'v>> as Insertable<E::Table>>::Values: Send,
    <E::Table as QuerySource>::FromClause: Send,
    InsertStatement<E::Table, <Vec<E::Post<'v>> as Insertable<E::Table>>::Values>:
        LoadQuery<'query, C::AsyncConnection, E::Raw>,
    E::Raw: MaybeAudit<'query, C::AsyncConnection>,

    // DbDelete bounds
    E: DbDelete,
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    DbEntityError<<E::Raw as TryInto<E>>::Error>: Debug + Send,
    E::Id: Clone + Debug + Send,
    for<'a> &'a E::Raw: Identifiable<Id = &'a E::Id>,
    <E::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
    E::Raw: Deletable<'query, C::AsyncConnection, E::Table, Vec<E::Id>, E::Id, E::DeletedAt, E::DeletePatch<'v>>,

    // additional bounds
    E: Into<E::Raw>,
    E::Id: Sync,
    for<'a> &'a E::Raw: Identifiable<Id = &'a E::Id>,
{
    type Ok = Vec<E>;
    type Error = DbEntityError<<E::Raw as TryInto<E>>::Error>;

    async fn act(client: &C, input: I) -> Result<Self::Ok, Self::Error>
    where
        C: 'async_trait,
        I: 'async_trait,
    {
        let records = E::insert(client, input).await?;

        let raw_records = records.into_iter().map(Into::into).collect::<Vec<E::Raw>>();

        let ids = raw_records.iter().map(|x| x.id().to_owned()).collect::<Vec<_>>();
        let records = E::delete(client, ids).await?;
        Ok(records)
    }
}

// because a successful execution of the CreateThenDelete operation will not
// produce any new db records (that we are interested in, i.e. excluding Audit records),
// we can simply do nothing in the transaction cache management for this action
impl<O, SC, I> TransactionCacheAction<CreateThenDelete<O>, SC, I> for MongodbTxCollection
where
    O: ?Sized + ObjectType,
    SC: ?Sized + StorageClient + Send + Sync,
    CreateThenDelete<O>: StorageAction<SC, I> + Send,
{
    fn manage_cache<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        _: SC::TransactionId<'life1>,
        _: &'life2 <CreateThenDelete<O> as StorageAction<SC, I>>::Ok,
    ) -> BoxFuture<'async_trait, Result<(), <Self as TransactionCache<SC>>::Error>>
    where
        Self: Sync,
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        Box::pin(async { Ok(()) })
    }
}
