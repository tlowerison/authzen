use crate::diesel::{connection::Db, prelude::*};
use ::async_backtrace::framed;
use ::async_trait::async_trait;
use ::derive_more::*;
use ::diesel::associations::HasTable;
use ::diesel::dsl::SqlTypeOf;
use ::diesel::expression::{AsExpression, Expression};
use ::diesel::expression_methods::ExpressionMethods;
use ::diesel::helper_types as ht;
use ::diesel::query_dsl::methods::{FilterDsl, FindDsl};
use ::diesel::query_source::QuerySource;
use ::diesel::result::Error;
use ::diesel::sql_types::SqlType;
use ::diesel::{query_builder::*, Identifiable};
use ::diesel::{Insertable, Table};
use ::diesel_async::methods::*;
use ::either::Either;
use ::futures::future::FutureExt;
use ::scoped_futures::ScopedFutureExt;
use ::std::borrow::Borrow;
use ::std::fmt::Debug;
use ::std::hash::Hash;
use ::tracing::{error, instrument};

/// Wrapper error type returned from DbEntity methods
/// solely exists to facilitate error conversion when
/// converting between the raw db representation and
/// the entity representation.
#[derive(Debug, Display, IsVariant, PartialEq, Unwrap, thiserror::Error)]
pub enum DbEntityError<E = ::std::convert::Infallible> {
    Db(Error),
    /// In the cases where [`DbEntity::Raw`] does not implement [`Into<Self>`]
    /// and instead only implements [`TryInto<Self>`], this variant captures
    /// any errors that occurred during conversion. Conversion types are
    /// useful when you have a more useful structure to keep your db entities
    /// in which doesn't map exactly to a diesel table instance, e.g. nesting
    /// structs or having types which are coupled i.e. if one is an Some
    /// then the other must be Some. In the cases where the assumptions made
    /// by the conversion type during conversion are not held up at runtime,
    /// i.e. data has been modified in the database in a way which breaks
    /// these assumptions, then implementing `TryInto` rather than `Into`
    /// for converting between the types is obviously a better result,
    /// allowing you to avoid a panic.
    Conversion(E),
}

impl<E> From<Error> for DbEntityError<E> {
    fn from(value: Error) -> Self {
        Self::Db(value)
    }
}

impl<E> DbEntityError<E> {
    pub fn conversion(value: E) -> Self {
        Self::Conversion(value)
    }
}

impl From<DbEntityError<::std::convert::Infallible>> for Error {
    fn from(value: DbEntityError<::std::convert::Infallible>) -> Self {
        value.unwrap_db()
    }
}

/// Base trait which unites common associated types used by the common sql operations in diesel
/// (i.e. select, insert, update and delete).
pub trait DbEntity: Sized + Send + 'static {
    /// An equivalent representation of this entity which has diesel trait implementations.
    type Raw: HasTable<Table = Self::Table> + TryInto<Self> + Send + 'static;

    /// The table this entity represents.
    type Table: Table + QueryId + Send;

    /// The type of this entity's table's primary key
    /// note that this type should be equivalent diesel sql_type representation of
    /// the type of the primary key field in [`Self::Raw`].
    type Id: AsExpression<SqlTypeOf<<Self::Table as Table>::PrimaryKey>>
    where
        <<Self::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType;
}

/// DbEntity is automatically implemented for any type which implements Audit and HasTable
impl<T, Tab, Id> DbEntity for T
where
    T: Clone + HasTable<Table = Tab> + Send + 'static,
    Tab: Table + QueryId + Send,

    Id: AsExpression<SqlTypeOf<Tab::PrimaryKey>>,
    for<'a> &'a T: Identifiable<Id = &'a Id>,
    Tab::PrimaryKey: Expression + ExpressionMethods,
    <Tab::PrimaryKey as Expression>::SqlType: SqlType,
{
    type Raw = T;
    type Table = Tab;
    type Id = Id;
}

/// Common operations used across most database tables.
pub mod operations {
    use super::*;

    pub use crate::diesel::deletable::*;
    pub use crate::diesel::is_deleted::*;

    #[async_trait]
    pub trait DbGet: DbEntity {
        #[framed]
        #[instrument(skip_all)]
        async fn get<'query, D, F>(
            db: &D,
            ids: impl IntoIterator<Item = Self::Id> + Send,
        ) -> Result<Vec<Self>, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,

            // Id bounds
            Self::Id: Debug + Send,
            for<'a> &'a Self::Raw: Identifiable<Id = &'a Self::Id>,
            <Self::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<Self::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            Self::Table: FilterDsl<ht::EqAny<<Self::Table as Table>::PrimaryKey, Vec<Self::Id>>, Output = F>,
            F: IsNotDeleted<'query, D::AsyncConnection, Self::Raw, Self::Raw>,
        {
            let ids = ids.into_iter().collect::<Vec<_>>();
            tracing::Span::current().record("ids", &*format!("{ids:?}"));

            if ids.is_empty() {
                return Ok(vec![]);
            }

            let result: Result<Vec<Self::Raw>, _> = db.get(ids).await;
            match result {
                Ok(records) => Ok(records
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()
                    .map_err(DbEntityError::conversion)?),
                Err(err) => {
                    error!(target: module_path!(), error = %err);
                    Err(err.into())
                }
            }
        }

        #[framed]
        #[instrument(skip_all)]
        async fn get_one<'query, D, F>(
            db: &D,
            id: Self::Id,
        ) -> Result<Self, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,

            // Id bounds
            Self::Id: Debug + Send,
            for<'a> &'a Self::Raw: Identifiable<Id = &'a Self::Id>,
            <Self::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<Self::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            Self::Table: FilterDsl<ht::EqAny<<Self::Table as Table>::PrimaryKey, [Self::Id; 1]>, Output = F>,
            F: IsNotDeleted<'query, D::AsyncConnection, Self::Raw, Self::Raw>,
        {
            let result: Result<Vec<Self::Raw>, _> = db.get([id]).await;
            match result {
                Ok(records) => Ok(records
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(DbEntityError::conversion)?
                    .pop()
                    .ok_or(Error::NotFound)?),
                Err(err) => {
                    error!(target: module_path!(), error = %err);
                    Err(err.into())
                }
            }
        }

        #[framed]
        #[instrument(skip_all)]
        async fn get_by_column<'query, D, C, U, F>(
            db: &D,
            column: C,
            values: impl IntoIterator<Item = U> + Send,
        ) -> Result<Vec<Self>, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,

            C: Debug + Expression + ExpressionMethods + Send,
            SqlTypeOf<C>: SqlType,
            U: AsExpression<SqlTypeOf<C>> + Debug + Send,
            Self::Table: FilterDsl<ht::EqAny<C, Vec<U>>, Output = F>,
            F: IsNotDeleted<'query, D::AsyncConnection, Self::Raw, Self::Raw>,
        {
            let values = values.into_iter().collect::<Vec<_>>();
            tracing::Span::current().record("values", &*format!("{values:?}"));

            let result: Result<Vec<Self::Raw>, _> = db.get_by_column(column, values).await;
            match result {
                Ok(records) => Ok(records
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()
                    .map_err(DbEntityError::conversion)?),
                Err(err) => {
                    error!(target: module_path!(), error = %err);
                    Err(err.into())
                }
            }
        }

        #[framed]
        #[instrument(skip_all)]
        async fn get_page<'query, D, P, F>(
            db: &D,
            page: P,
        ) -> Result<Vec<Self>, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,

            // Page bounds
            P: Borrow<Page> + Debug + Send,

            // Query bounds
            Self::Table: IsNotDeleted<'query, D::AsyncConnection, Self::Raw, Self::Raw, IsNotDeletedFilter = F>,
            F: Paginate + Send,
            <F as AsQuery>::Query: 'query,
            Paginated<<F as AsQuery>::Query>: Send + LoadQuery<'query, D::AsyncConnection, Self::Raw>,
        {
            if page.borrow().is_empty() {
                return Ok(vec![]);
            }
            let result: Result<Vec<Self::Raw>, _> = db.get_page(page).await;
            match result {
                Ok(records) => Ok(records
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()
                    .map_err(DbEntityError::conversion)?),
                Err(err) => {
                    error!(target: module_path!(), error = %err);
                    Err(err.into())
                }
            }
        }

        #[framed]
        #[instrument(skip_all)]
        async fn get_pages<'query, D, P, F>(
            db: &D,
            pages: impl IntoIterator<Item = P> + Send,
        ) -> Result<Vec<Self>, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,

            // Page bounds
            P: Borrow<Page> + Debug + for<'a> PageRef<'a> + Send,

            // Query bounds
            Self::Table: IsNotDeleted<'query, D::AsyncConnection, Self::Raw, Self::Raw, IsNotDeletedFilter = F>,
            F: Paginate + Send,
            <F as AsQuery>::Query: 'query,
            Paginated<<F as AsQuery>::Query>: Send + LoadQuery<'query, D::AsyncConnection, Self::Raw>,
        {
            let pages = pages.into_iter().collect::<Vec<_>>();
            tracing::Span::current().record("pages", &*format!("{pages:?}"));

            if pages.iter().all(|page| page.borrow().is_empty()) {
                return Ok(vec![]);
            }

            let result: Result<Vec<Self::Raw>, _> = db.get_pages(pages).await;
            match result {
                Ok(records) => Ok(records
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()
                    .map_err(DbEntityError::conversion)?),
                Err(err) => {
                    error!(target: module_path!(), error = %err);
                    Err(err.into())
                }
            }
        }
    }

    #[async_trait]
    pub trait DbInsert: DbEntity {
        type PostHelper<'v>: Debug + Into<Self::Post<'v>> + Send = Self::Post<'v>;
        type Post<'v>: Debug + HasTable<Table = Self::Table> + Send;

        #[framed]
        #[instrument(skip_all)]
        async fn insert<'query, 'v, D>(
            db: &D,
            posts: impl IntoIterator<Item = Self::PostHelper<'v>> + Send + 'v,
        ) -> Result<Vec<Self>, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db + 'query,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,
            'v: 'query,

            <Self::Raw as TryInto<Self>>::Error: Send,

            // Insertable bounds
            Vec<Self::Post<'v>>: Insertable<Self::Table> + Send,
            <Vec<Self::Post<'v>> as Insertable<Self::Table>>::Values: Send,
            <Self::Table as QuerySource>::FromClause: Send,

            // Insert bounds
            InsertStatement<Self::Table, <Vec<Self::Post<'v>> as Insertable<Self::Table>>::Values>:
                LoadQuery<'query, D::AsyncConnection, Self::Raw>,

            // Audit bounds
            Self::Raw: MaybeAudit<'query, D::AsyncConnection>,
        {
            let db_post_helpers = posts.into_iter().collect::<Vec<_>>();
            tracing::Span::current().record("posts", &*format!("{db_post_helpers:?}"));

            if db_post_helpers.is_empty() {
                return Ok(vec![]);
            }

            let db_posts = db_post_helpers.into_iter().map(Self::PostHelper::into);

            db.insert(db_posts)
                .map(|result| match result {
                    Ok(records) => Ok(records),
                    Err(err) => {
                        let err = err;
                        error!(target: module_path!(), error = %err);
                        Err(err)
                    }
                })
                .await
                .map_err(DbEntityError::from)
                .and_then(|records| {
                    records
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()
                        .map_err(DbEntityError::conversion)
                })
        }

        #[framed]
        #[instrument(skip_all)]
        async fn insert_one<'query, 'v, D>(
            db: &D,
            post: Self::PostHelper<'v>,
        ) -> Result<Self, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db + 'query,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,
            'v: 'query,

            <Self::Raw as TryInto<Self>>::Error: Send,

            // Insertable bounds
            Self::Post<'v>: HasTable,
            [Self::Post<'v>; 1]: Insertable<Self::Table> + Send,
            <[Self::Post<'v>; 1] as Insertable<Self::Table>>::Values: Send + 'query,
            <Self::Table as QuerySource>::FromClause: Send,

            // Insert bounds
            InsertStatement<Self::Table, <[Self::Post<'v>; 1] as Insertable<Self::Table>>::Values>:
                LoadQuery<'query, D::AsyncConnection, Self::Raw>,

            // Audit bounds
            Self::Raw: MaybeAudit<'query, D::AsyncConnection>,
        {
            tracing::Span::current().record("post", &*format!("{post:?}"));

            db.insert_one(post.into())
                .map(|result| match result {
                    Ok(record) => Ok(record),
                    Err(err) => {
                        let err = err;
                        error!(target: module_path!(), error = %err);
                        Err(err)
                    }
                })
                .await
                .map_err(DbEntityError::from)
                .and_then(|record| record.try_into().map_err(DbEntityError::conversion))
        }
    }

    #[async_trait]
    pub trait DbUpdate: DbEntity {
        /// conversion helper type if needed, defaults to [`Self::Patch`]
        type PatchHelper<'v>: Debug + Into<Self::Patch<'v>> + Send = Self::Patch<'v>;
        type Patch<'v>: AsChangeset<Target = Self::Table>
            + Debug
            + HasTable<Table = Self::Table>
            + IncludesChanges
            + Send
            + Sync;

        #[framed]
        #[instrument(skip_all)]
        async fn update<'query, 'v, D, F>(
            db: &D,
            patches: impl IntoIterator<Item = Self::PatchHelper<'v>> + Send + 'v,
        ) -> Result<Vec<Self>, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db + 'query,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: diesel_async::AsyncConnection<Backend = D::Backend>,
            'v: 'query,

            // Id bounds
            Self::Id: Clone + Hash + Eq + Send + Sync,
            for<'a> &'a Self::Raw: Identifiable<Id = &'a Self::Id>,
            <Self::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<Self::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Changeset bounds
            <Self::Patch<'v> as AsChangeset>::Changeset: Send,
            for<'a> &'a Self::Patch<'v>:
                HasTable<Table = Self::Table> + Identifiable<Id = &'a Self::Id> + IntoUpdateTarget,
            for<'a> <&'a Self::Patch<'v> as IntoUpdateTarget>::WhereClause: Send,
            <Self::Table as QuerySource>::FromClause: Send,

            // UpdateStatement bounds
            Self::Table: FindDsl<Self::Id>,
            ht::Find<Self::Table, Self::Id>: HasTable<Table = Self::Table> + IntoUpdateTarget + Send,
            <ht::Find<Self::Table, Self::Id> as IntoUpdateTarget>::WhereClause: Send,
            ht::Update<ht::Find<Self::Table, Self::Id>, Self::Patch<'v>>:
                AsQuery + LoadQuery<'query, D::AsyncConnection, Self::Raw> + Send,

            // Filter bounds for records whose changesets do not include any changes
            Self::Table: FilterDsl<ht::EqAny<<Self::Table as Table>::PrimaryKey, Vec<Self::Id>>, Output = F>,
            F: IsNotDeleted<'query, D::AsyncConnection, Self::Raw, Self::Raw>,

            // Audit bounds
            Self::Raw: MaybeAudit<'query, D::AsyncConnection>,
        {
            let db_patch_helpers = patches.into_iter().collect::<Vec<_>>();
            tracing::Span::current().record("patches", &*format!("{db_patch_helpers:?}"));

            if db_patch_helpers.is_empty() {
                return Ok(vec![]);
            }

            db.update(db_patch_helpers.into_iter().map(Self::PatchHelper::into))
                .map(|result| match result {
                    Ok(records) => Ok(records),
                    Err(err) => {
                        let err = err;
                        error!(target: module_path!(), error = %err);
                        Err(err)
                    }
                })
                .await
                .map_err(DbEntityError::from)
                .and_then(|records| {
                    records
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()
                        .map_err(DbEntityError::conversion)
                })
        }

        #[framed]
        #[instrument(skip_all)]
        async fn update_one<'query, 'v, D, F>(
            db: &D,
            patch: Self::PatchHelper<'v>,
        ) -> Result<Self, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db + 'query,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,
            'v: 'query,

            // Id bounds
            Self::Id: Clone + Hash + Eq + Send + Sync,
            for<'a> &'a Self::Raw: Identifiable<Id = &'a Self::Id>,
            <Self::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<Self::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Changeset bounds
            <Self::Patch<'v> as AsChangeset>::Changeset: Send,
            for<'a> &'a Self::Patch<'v>:
                HasTable<Table = Self::Table> + Identifiable<Id = &'a Self::Id> + IntoUpdateTarget,
            for<'a> <&'a Self::Patch<'v> as IntoUpdateTarget>::WhereClause: Send,
            <Self::Table as QuerySource>::FromClause: Send,

            // UpdateStatement bounds
            Self::Table: FindDsl<Self::Id>,
            ht::Find<Self::Table, Self::Id>: HasTable<Table = Self::Table> + IntoUpdateTarget + Send,
            <ht::Find<Self::Table, Self::Id> as IntoUpdateTarget>::WhereClause: Send,
            ht::Update<ht::Find<Self::Table, Self::Id>, Self::Patch<'v>>:
                AsQuery + LoadQuery<'query, D::AsyncConnection, Self::Raw> + Send,

            // Filter bounds for records whose changesets do not include any changes
            Self::Table: FilterDsl<ht::EqAny<<Self::Table as Table>::PrimaryKey, Vec<Self::Id>>, Output = F>,
            F: IsNotDeleted<'query, D::AsyncConnection, Self::Raw, Self::Raw>,

            // Audit bounds
            Self::Raw: MaybeAudit<'query, D::AsyncConnection>,
        {
            tracing::Span::current().record("patch", &*format!("{patch:?}"));

            db.update([patch.into()])
                .map(|result| match result {
                    Ok(mut records) => Ok(records.pop().ok_or(Error::NotFound)?),
                    Err(err) => {
                        let err = err;
                        error!(target: module_path!(), error = %err);
                        Err(err)
                    }
                })
                .await
                .map_err(DbEntityError::from)
                .and_then(|record| record.try_into().map_err(DbEntityError::conversion))
        }
    }

    #[async_trait]
    pub trait DbDelete: DbEntity {
        type DeletedAt;
        type DeletePatch<'v>;

        #[framed]
        #[instrument(skip_all)]
        async fn delete<'query, 'v, D, I>(
            db: &D,
            ids: I,
        ) -> Result<Vec<Self>, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db + 'query,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,

            I: Send,

            // Id bounds
            Self::Id: Debug + Send,
            for<'a> &'a Self::Raw: Identifiable<Id = &'a Self::Id>,
            <Self::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<Self::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            Self::Raw:
                Deletable<'query, D::AsyncConnection, Self::Table, I, Self::Id, Self::DeletedAt, Self::DeletePatch<'v>>,
        {
            db.raw_tx(move |conn| {
                async move {
                    match Self::Raw::maybe_soft_delete(conn, ids).await {
                        Either::Left(ids) => Self::Raw::hard_delete(conn, ids).await,
                        Either::Right(result) => result,
                    }
                }
                .scope_boxed()
            })
            .map(|result| match result {
                Ok(records) => Ok(records),
                Err(err) => {
                    let err = err;
                    error!(target: module_path!(), error = %err);
                    Err(err)
                }
            })
            .await
            .map_err(DbEntityError::from)
            .and_then(|records| {
                records
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<_, _>>()
                    .map_err(DbEntityError::conversion)
            })
        }

        #[framed]
        #[instrument(skip_all)]
        async fn delete_one<'query, 'v, D, I>(
            db: &D,
            id: I,
        ) -> Result<Self, DbEntityError<<Self::Raw as TryInto<Self>>::Error>>
        where
            D: Db + 'query,
            D::Backend: ::diesel::backend::Backend,
            D::AsyncConnection: ::diesel_async::AsyncConnection<Backend = D::Backend>,

            I: Send,

            // Id bounds
            Self::Id: Debug + Send,
            for<'a> &'a Self::Raw: Identifiable<Id = &'a Self::Id>,
            <Self::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<Self::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            Self::Raw: Deletable<
                'query,
                D::AsyncConnection,
                Self::Table,
                [I; 1],
                Self::Id,
                Self::DeletedAt,
                Self::DeletePatch<'v>,
            >,
        {
            db.raw_tx(move |conn| {
                async move {
                    let ids = [id];
                    match Self::Raw::maybe_soft_delete(conn, ids).await {
                        Either::Left(ids) => Self::Raw::hard_delete(conn, ids).await,
                        Either::Right(result) => result,
                    }
                }
                .scope_boxed()
            })
            .map(|result| match result {
                Ok(records) => Ok(records),
                Err(err) => {
                    let err = err;
                    error!(target: module_path!(), error = %err);
                    Err(err)
                }
            })
            .await
            .map_err(DbEntityError::from)
            .and_then(|records| {
                let mut records = records
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(DbEntityError::conversion)?;
                if records.is_empty() {
                    return Err(DbEntityError::Db(Error::NotFound));
                }
                Ok(records.pop().unwrap())
            })
        }
    }

    impl<T: DbEntity> DbGet for T {}

    impl<T: DbEntity> DbInsert for T
    where
        T: Debug + HasTable<Table = Self::Table> + Send,
    {
        type Post<'v> = T;
    }

    impl<T: DbEntity> DbDelete for T {
        default type DeletedAt = ();
        default type DeletePatch<'v> = ();
    }

    impl<T: DbEntity> DbUpdate for T
    where
        T: AsChangeset<Target = Self::Table> + Debug + HasTable<Table = Self::Table> + IncludesChanges + Send + Sync,
    {
        type Patch<'v> = T;
    }
}
