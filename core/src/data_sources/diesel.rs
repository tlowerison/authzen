use crate::*;
use ::authzen_data_sources::diesel::{connection::Db, prelude::*};
use ::diesel::associations::HasTable;
use ::diesel::backend::Backend;
use ::diesel::expression::Expression;
use ::diesel::expression_methods::ExpressionMethods;
use ::diesel::helper_types as ht;
use ::diesel::query_dsl::methods::{FilterDsl, FindDsl};
use ::diesel::query_source::QuerySource;
use ::diesel::sql_types::SqlType;
use ::diesel::{query_builder::*, Identifiable};
use ::diesel::{Insertable, Table};
use ::diesel_async::methods::*;
use ::diesel_async::AsyncConnection;
use ::serde::{de::DeserializeOwned, Serialize};
use ::std::fmt::Debug;
use ::std::hash::Hash;

impl<E, B> StorageObject<B> for E
where
    E: DbEntity,
    B: Backend,
{
}

impl StorageError for ::diesel::result::Error {
    fn not_found() -> Self {
        Self::NotFound
    }
}

impl<E> StorageError for DbEntityError<E> {
    fn not_found() -> Self {
        Self::Db(::diesel::result::Error::NotFound)
    }
}

impl<T, Id> crate::Identifiable for T
where
    for<'a> &'a T: Identifiable<Id = &'a Id>,
    Id: Clone + DeserializeOwned + Eq + Hash + Send + Serialize + Sync + 'static,
{
    type Id = Id;

    fn id(&self) -> &Self::Id {
        <&Self as Identifiable>::id(self)
    }
}

#[async_trait]
impl<'query, 'v, E, B, I, C, O> StorageAction<C, I> for actions::Create<O>
where
    O: ?Sized + AsStorage<B, StorageObject = E>,
    E: DbInsert + Sync,
    B: Backend,
    I: IntoIterator<Item = E::PostHelper<'v>> + Send,
    C: Db<Backend = B>,
    <C as TransactionalDataSource>::AsyncConnection: AsyncConnection<Backend = B>,

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
impl<'query, 'v, E, B, I, C, O> StorageAction<C, I> for actions::Delete<O>
where
    O: ?Sized + AsStorage<B, StorageObject = E>,
    E: DbDelete + Sync,
    B: Backend,
    I: IntoIterator<Item = E::Id> + Send,
    C: Db<Backend = B> + 'query,
    <C as TransactionalDataSource>::AsyncConnection: AsyncConnection<Backend = B>,

    // DbEntity bounds
    <<E::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    DbEntityError<<E::Raw as TryInto<E>>::Error>: Debug + Send,

    E::Id: Debug + Send,
    for<'a> &'a E::Raw: Identifiable<Id = &'a E::Id>,
    <E::Table as Table>::PrimaryKey: Expression + ExpressionMethods,

    E::Raw: Deletable<'query, C::AsyncConnection, E::Table, I, E::Id, E::DeletedAt, E::DeletePatch<'v>>,
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
impl<E, B, I, C, F, O> StorageAction<C, I> for actions::Read<O>
where
    O: ?Sized + AsStorage<B, StorageObject = E>,
    E: DbEntity + Sync,
    B: Backend,
    I: IntoIterator<Item = E::Id> + Send,
    C: Db<Backend = B>,
    <C as TransactionalDataSource>::AsyncConnection: AsyncConnection<Backend = B>,

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
impl<'query, 'v, E, B, I, C, F, O> StorageAction<C, I> for actions::Update<O>
where
    O: ?Sized + AsStorage<B, StorageObject = E>,
    E: DbUpdate + Sync,
    B: Backend,
    I: IntoIterator<Item = E::PatchHelper<'v>> + Send + 'v,
    C: Db<Backend = B> + 'query,
    <C as TransactionalDataSource>::AsyncConnection: AsyncConnection<Backend = B>,

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
