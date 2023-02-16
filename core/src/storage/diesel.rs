use crate::*;
use diesel::associations::HasTable;
use diesel::backend::Backend;
use diesel::expression::Expression;
use diesel::expression_methods::ExpressionMethods;
use diesel::helper_types as ht;
use diesel::query_dsl::methods::{FilterDsl, FindDsl};
use diesel::query_source::QuerySource;
use diesel::sql_types::SqlType;
use diesel::{query_builder::*, Identifiable};
use diesel::{Insertable, Table};
use diesel_async::methods::*;
use diesel_util::*;
use std::fmt::Debug;

impl<B> StorageBackend for B where B: Backend {}

impl<E, B> StorageObject<B> for E
where
    E: DbEntity,
    B: Backend,
{
}

impl<C: _Db> StorageClient for C {
    type Backend = <C as _Db>::Backend;
}

#[async_trait]
impl<'query, 'v, E, B, I, C, O> StorageAction<C, I> for action::Create<O>
where
    O: HasStorageObject<B, StorageObject = E>,
    E: DbInsert,
    B: Backend,
    I: IntoIterator<Item = E::PostHelper<'v>> + Send,
    C: _Db<Backend = B>,

    // DbEntity bounds
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    DbEntityError<<E::Raw as TryInto<E>>::Error>: Debug + Send,

    // DbInsert::insert bounds
    'v: 'query,
    I: 'v,
    C: 'query,
    <E::Raw as TryInto<E>>::Error: Send,
    Vec<E::Post<'v>>: Insertable<E::Table> + Send,
    <Vec<E::Post<'v>> as Insertable<E::Table>>::Values: Send,
    <E::Table as QuerySource>::FromClause: Send,
    InsertStatement<E::Table, <Vec<E::Post<'v>> as Insertable<E::Table>>::Values>:
        LoadQuery<'query, C::AsyncConnection, E::Raw>,

    // Audit bounds
    E::Raw: MaybeAudit<'query, C::AsyncConnection>,
{
    type Ok = Vec<E>;
    type Error = DbEntityError<<E::Raw as TryInto<E>>::Error>;

    async fn act(client: &C, input: I) -> Result<Self::Ok, Self::Error>
    where
        C: 'async_trait,
        I: 'async_trait,
    {
        Ok(E::insert(client, input).await?)
    }
}

#[async_trait]
impl<'query, 'v, E, B, I, C, F, O> StorageAction<C, I> for action::Delete<O>
where
    O: HasStorageObject<B, StorageObject = E>,
    E: DbDelete,
    B: Backend,
    I: IntoIterator<Item = E::Id> + Send,
    C: _Db<Backend = B> + 'query,

    // DbEntity bounds
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    DbEntityError<<E::Raw as TryInto<E>>::Error>: Debug + Send,

    E::Id: Debug + Send,
    for<'a> &'a E::Raw: Identifiable<Id = &'a E::Id>,
    <E::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
    E::Table: FilterDsl<ht::EqAny<<E::Table as Table>::PrimaryKey, Vec<E::Id>>, Output = F>,

    E::Raw: Deletable<'query, C::AsyncConnection, E::Table, I, F, E::DeletedAt, E::DeletePatch<'v>>,
{
    type Ok = Vec<E>;
    type Error = DbEntityError<<E::Raw as TryInto<E>>::Error>;

    async fn act(client: &C, input: I) -> Result<Self::Ok, Self::Error>
    where
        C: 'async_trait,
        I: 'async_trait,
    {
        Ok(E::delete(client, input).await?)
    }
}

#[async_trait]
impl<E, B, I, C, F, O> StorageAction<C, I> for action::Read<O>
where
    O: HasStorageObject<B, StorageObject = E>,
    E: DbEntity,
    B: Backend,
    I: IntoIterator<Item = E::Id> + Send,
    C: _Db<Backend = B>,

    // DbEntity bounds
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    DbEntityError<<E::Raw as TryInto<E>>::Error>: Debug + Send,

    // DbGet::get bounds
    E::Id: Debug + Send,
    for<'a> &'a E::Raw: Identifiable<Id = &'a E::Id>,
    <E::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
    E::Table: FilterDsl<ht::EqAny<<E::Table as Table>::PrimaryKey, Vec<E::Id>>, Output = F>,
    F: for<'query> IsNotDeleted<'query, C::AsyncConnection, E::Raw, E::Raw>,
{
    type Ok = Vec<E>;
    type Error = DbEntityError<<E::Raw as TryInto<E>>::Error>;

    async fn act(client: &C, input: I) -> Result<Self::Ok, Self::Error>
    where
        C: 'async_trait,
        I: 'async_trait,
    {
        Ok(E::get(client, input).await?)
    }
}

#[async_trait]
impl<'query, 'v, E, B, I, C, F, O> StorageAction<C, I> for action::Update<O>
where
    O: HasStorageObject<B, StorageObject = E>,
    E: DbUpdate,
    B: Backend,
    I: IntoIterator<Item = E::PatchHelper<'v>> + Send + 'v,
    C: _Db<Backend = B> + 'query,

    'v: 'query,

    // DbEntity bounds
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    DbEntityError<<E::Raw as TryInto<E>>::Error>: Debug + Send,

    // Id bounds
    E::Id: Clone + Hash + Eq + Send + Sync,
    for<'a> &'a E::Raw: Identifiable<Id = &'a E::Id>,
    <E::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

    // Changeset bounds
    <E::Patch<'v> as AsChangeset>::Changeset: Send,
    for<'a> &'a E::Patch<'v>: HasTable<Table = E::Table> + Identifiable<Id = &'a E::Id> + IntoUpdateTarget,
    for<'a> <&'a E::Patch<'v> as IntoUpdateTarget>::WhereClause: Send,
    <E::Table as QuerySource>::FromClause: Send,

    // UpdateStatement bounds
    E::Table: FindDsl<E::Id>,
    ht::Find<E::Table, E::Id>: HasTable<Table = E::Table> + IntoUpdateTarget + Send,
    <ht::Find<E::Table, E::Id> as IntoUpdateTarget>::WhereClause: Send,
    ht::Update<ht::Find<E::Table, E::Id>, E::Patch<'v>>: AsQuery + LoadQuery<'query, C::AsyncConnection, E::Raw> + Send,

    // Filter bounds for records whose changesets do not include any changes
    E::Table: FilterDsl<ht::EqAny<<E::Table as Table>::PrimaryKey, Vec<E::Id>>, Output = F>,
    F: IsNotDeleted<'query, C::AsyncConnection, E::Raw, E::Raw>,

    // Audit bounds
    E::Raw: MaybeAudit<'query, C::AsyncConnection>,
{
    type Ok = Vec<E>;
    type Error = DbEntityError<<E::Raw as TryInto<E>>::Error>;

    async fn act(client: &C, input: I) -> Result<Self::Ok, Self::Error>
    where
        C: 'async_trait,
        I: 'async_trait,
    {
        Ok(E::update(client, input).await?)
    }
}
