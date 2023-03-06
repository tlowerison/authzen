use crate::prelude::*;
use async_backtrace::{backtrace, Location};
use diesel::associations::HasTable;
use diesel::backend::Backend;
use diesel::dsl::SqlTypeOf;
use diesel::expression::{AsExpression, Expression};
use diesel::expression_methods::ExpressionMethods;
use diesel::helper_types as ht;
use diesel::query_builder::*;
use diesel::query_dsl::methods::{FilterDsl, FindDsl};
use diesel::query_source::QuerySource;
use diesel::result::Error;
use diesel::sql_types::SqlType;
use diesel::{Identifiable, Insertable, Table};
use diesel_async::{methods::*, AsyncConnection, RunQueryDsl};
use futures::future::{ready, BoxFuture, FutureExt, TryFutureExt};
use scoped_futures::{ScopedBoxFuture, ScopedFutureExt};
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fmt::Display;
use std::ops::Deref;
use std::{fmt::Debug, hash::Hash, sync::Arc};
use tokio::sync::{Mutex, RwLock};
use tracing::Instrument;
use uuid::Uuid;

#[derive(AsRef, AsMut, Deref, DerefMut, Derivative)]
#[derivative(Debug)]
pub struct DbConnection<AC, C> {
    #[deref]
    #[deref_mut]
    #[derivative(Debug = "ignore")]
    pub(crate) connection: Arc<RwLock<C>>,
    #[derivative(Debug = "ignore")]
    pub(crate) tx_cleanup: TxCleanup<AC>,
    pub(crate) tx_id: Option<Uuid>,
}

impl<AC, C> Clone for DbConnection<AC, C> {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            tx_cleanup: self.tx_cleanup.clone(),
            tx_id: self.tx_id,
        }
    }
}

pub(crate) type DbConnRef<'a, C> = DbConnection<C, &'a mut C>;

#[derive(Derivative, thiserror::Error)]
#[derivative(Debug)]
#[error("{source}")]
pub struct TxCleanupError {
    pub source: anyhow::Error,
    #[derivative(Debug = "ignore")]
    pub backtrace: Option<Box<[Location]>>,
}

impl TxCleanupError {
    #[framed]
    pub fn new<S: Display + Debug + Send + Sync + 'static>(msg: S) -> Self {
        Self {
            source: anyhow::Error::msg(msg),
            backtrace: backtrace(),
        }
    }
}

impl From<anyhow::Error> for TxCleanupError {
    #[framed]
    fn from(source: anyhow::Error) -> Self {
        Self {
            source,
            backtrace: backtrace(),
        }
    }
}

impl From<TxCleanupError> for Error {
    fn from(value: TxCleanupError) -> Self {
        error!("transaction cleanup error occurred within a transaction whose error type is diesel::result::Error, converting error to diesel::result::Error::RollbackTransaction");
        error!("original error: {value}");
        Self::RollbackTransaction
    }
}

cfg_if! {
    if #[cfg(any(feature = "bb8", feature = "deadpool", feature = "mobc"))] {
        use diesel_async::pooled_connection::{self as pc, PoolableConnection};
        use std::ops::DerefMut;

        pub(crate) type DbConnOwned<'a, C> = DbConnection<C, PooledConnection<'a, C>>;

        #[derive(Derivative)]
        #[derivative(Debug)]
        pub enum PooledConnection<'a, C: PoolableConnection + 'static> {
            #[cfg(feature = "bb8")]
            Bb8(
                #[derivative(Debug = "ignore")]
                pc::bb8::PooledConnection<'a, C>,
            ),
            #[cfg(feature = "deadpool")]
            Deadpool(
                #[derivative(Debug = "ignore")]
                pc::deadpool::Object<C>,
                #[derivative(Debug = "ignore")]
                std::marker::PhantomData<&'a ()>,
            ),
            #[cfg(feature = "mobc")]
            Mobc(
                #[derivative(Debug = "ignore")]
                mobc::Connection<pc::AsyncDieselConnectionManager<C>>,
                #[derivative(Debug = "ignore")]
                std::marker::PhantomData<&'a ()>,
            ),
        }

        impl<'a, C: AsyncConnection + PoolableConnection + 'static> Deref for PooledConnection<'a, C> {
            type Target = C;
            fn deref(&self) -> &Self::Target {
                match self {
                    #[cfg(feature = "bb8")]
                    Self::Bb8(conn) => conn.deref(),
                    #[cfg(feature = "deadpool")]
                    Self::Deadpool(conn, _) => conn.deref(),
                    #[cfg(feature = "mobc")]
                    Self::Mobc(conn, _) => conn.deref(),
                }
            }
        }

        impl<'a, C: AsyncConnection + PoolableConnection + 'static> DerefMut for PooledConnection<'a, C> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                match self {
                    #[cfg(feature = "bb8")]
                    Self::Bb8(conn) => conn.deref_mut(),
                    #[cfg(feature = "deadpool")]
                    Self::Deadpool(conn, _) => conn.deref_mut(),
                    #[cfg(feature = "mobc")]
                    Self::Mobc(conn, _) => conn.deref_mut(),
                }
            }
        }

        impl<'a, C: PoolableConnection + 'static> PoolableConnection for PooledConnection<'a, C> {}

        impl<'a, C: PoolableConnection + 'static> From<PooledConnection<'a, C>> for DbConnOwned<'a, C> {
            fn from(connection: PooledConnection<'a, C>) -> Self {
                DbConnection {
                    tx_id: None,
                    connection: Arc::new(RwLock::new(connection)),
                    tx_cleanup: Arc::new(Mutex::new(vec![])),
                }
            }
        }
    }
}

pub trait TxFn<'a, C, Fut>: FnOnce(C) -> Fut + Send + 'a {
    fn call_tx_fn(self, connection: C) -> Fut;
}

impl<'a, C, F, Fut> TxFn<'a, C, Fut> for F
where
    C: Send,
    F: FnOnce(C) -> Fut + Send + 'a,
{
    fn call_tx_fn(self, connection: C) -> Fut {
        (self)(connection)
    }
}

pub trait TxCleanupFn<'r, AC: 'r, E> =
    FnOnce(&'r DbConnRef<'r, AC>) -> BoxFuture<'r, Result<(), E>> + Send + Sync + 'static;

pub type TxCleanup<AC> = Arc<Mutex<Vec<Box<dyn for<'r> TxCleanupFn<'r, AC, TxCleanupError>>>>>;

impl<AC, C> From<DbConnection<AC, C>> for Cow<'_, DbConnection<AC, C>> {
    fn from(value: DbConnection<AC, C>) -> Self {
        Cow::Owned(value)
    }
}

impl<'a, AC, C> From<&'a DbConnection<AC, C>> for Cow<'a, DbConnection<AC, C>> {
    fn from(value: &'a DbConnection<AC, C>) -> Self {
        Cow::Borrowed(value)
    }
}

macro_rules! instrument_err {
    ($fut:expr) => {
        $fut.map_err(|err| {
            tracing::Span::current().record("error", &&*format!("{err:?}"));
            err
        })
        .instrument(tracing::Span::current())
    };
}

macro_rules! execute_query {
    ($self:expr, $query:expr $(,)?) => {{
        let query = $query;
        instrument_err!($self.query(move |connection| Box::pin(query.get_results(connection))))
    }};
}

/// Db represents a shared reference to a mutable async db connection
/// It abstracts over the case where the connection is owned vs. a mutable reference.
/// The main goal of this abstraction is to defer locking the access to the
/// connection until the latest point possible, allowing other code paths (excepting
/// for connections in transactions) to access the connection as well.
/// At the moment, Db is passed in by value instead of reference into the transaction
/// provided transaction wrapper so you'll need to use `&conn` instead of just `conn`.
///
/// Applications using this trait will likely want to export their own trait alias (or
/// equivalent workaround) for it with the appropriate backend specified. Note that
/// this trait is not included in this crate's prelude to help prevent potential collisions
/// if the desired trait alias would also be named Db.
#[async_trait]
pub trait Db: Clone + Debug + Send + Sync + Sized {
    type Backend: Backend;
    type AsyncConnection: AsyncConnection<Backend = Self::Backend> + 'static;
    type Connection<'r>: Deref<Target = Self::AsyncConnection> + Send + Sync
    where
        Self: 'r;
    type TxConnection<'r>: Db<Backend = Self::Backend, AsyncConnection = Self::AsyncConnection>;

    async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Error> + Send + 'a,
        T: Send + 'a;

    async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Error> + Send + 'a,
        T: Send + 'a;

    fn tx_id(&self) -> Option<Uuid>;

    async fn tx_cleanup<F, E>(&self, f: F)
    where
        F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E>,
        E: Into<TxCleanupError> + 'static;

    async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
    where
        F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
        E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
        'life0: 'a;

    async fn raw_tx<'a, T, E, F>(&self, callback: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
    {
        self.with_tx_connection(callback).await
    }

    #[framed]
    #[instrument(skip_all)]
    fn get<'life0, 'async_trait, 'query, R, T, Pk, I, F>(
        &'life0 self,
        ids: I,
    ) -> BoxFuture<'async_trait, Result<Vec<R>, Error>>
    where
        Pk: AsExpression<SqlTypeOf<T::PrimaryKey>>,
        I: Debug + IntoIterator<Item = Pk> + Send,
        <I as IntoIterator>::IntoIter: Send + ExactSizeIterator,
        R: Send + HasTable<Table = T>,
        T: Table,
        T::PrimaryKey: Expression + ExpressionMethods,
        <T::PrimaryKey as Expression>::SqlType: SqlType,
        T: FilterDsl<ht::EqAny<<T as Table>::PrimaryKey, I>, Output = F>,
        F: IsNotDeleted<'query, Self::AsyncConnection, R, R>,

        'life0: 'async_trait,
        'query: 'async_trait,
        R: 'async_trait,
        T: 'async_trait,
        Pk: 'async_trait,
        F: 'async_trait,
        I: 'async_trait,
        Self: 'life0,
    {
        let ids = ids.into_iter();
        if ids.len() == 0 {
            return Box::pin(ready(Ok(vec![])));
        }
        execute_query!(
            self,
            R::table().filter(R::table().primary_key().eq_any(ids)).is_not_deleted(),
        )
        .boxed()
    }

    #[framed]
    #[instrument(skip_all)]
    fn get_by_column<'life0, 'async_trait, 'query, R, T, U, C, F>(
        &'life0 self,
        c: C,
        values: impl IntoIterator<Item = U> + Debug + Send,
    ) -> BoxFuture<'async_trait, Result<Vec<R>, Error>>
    where
        C: Debug + Expression + ExpressionMethods + Send,
        SqlTypeOf<C>: SqlType,
        U: AsExpression<SqlTypeOf<C>>,
        R: Send + HasTable<Table = T>,
        T: Table,
        T: FilterDsl<ht::EqAny<C, Vec<U>>, Output = F>,
        F: IsNotDeleted<'query, Self::AsyncConnection, R, R>,

        'life0: 'async_trait,
        'query: 'async_trait,
        R: 'async_trait,
        U: 'async_trait,
        C: 'async_trait,
        F: 'async_trait,
        Self: 'life0,
    {
        execute_query!(
            self,
            R::table()
                .filter(c.eq_any(values.into_iter().collect::<Vec<_>>()))
                .is_not_deleted(),
        )
        .boxed()
    }

    #[framed]
    #[instrument(skip_all)]
    fn get_page<'life0, 'async_trait, 'query, R, P, F>(
        &'life0 self,
        page: P,
    ) -> BoxFuture<'async_trait, Result<Vec<R>, Error>>
    where
        P: Borrow<Page> + Debug + Send,
        R: Send + HasTable,
        <R as HasTable>::Table: Table + IsNotDeleted<'query, Self::AsyncConnection, R, R, IsNotDeletedFilter = F>,
        F: Paginate + Send,
        <F as AsQuery>::Query: 'query,
        Paginated<<F as AsQuery>::Query>: LoadQuery<'query, Self::AsyncConnection, R> + Send,

        'life0: 'async_trait,
        'query: 'async_trait,
        R: 'async_trait,
        P: 'async_trait,
        F: 'async_trait,
        Self: 'life0,
    {
        if page.borrow().is_empty() {
            return Box::pin(ready(Ok(vec![])));
        }
        execute_query!(self, R::table().is_not_deleted().paginate(page)).boxed()
    }

    #[framed]
    #[instrument(skip_all)]
    fn get_pages<'life0, 'async_trait, 'query, R, P, I, F>(
        &'life0 self,
        pages: I,
    ) -> BoxFuture<'async_trait, Result<Vec<R>, Error>>
    where
        P: Debug + for<'a> PageRef<'a> + Send,
        I: Debug + IntoIterator<Item = P> + Send,
        <I as IntoIterator>::IntoIter: Send,
        R: Send + HasTable,
        <R as HasTable>::Table: Table + IsNotDeleted<'query, Self::AsyncConnection, R, R, IsNotDeletedFilter = F>,
        F: Paginate + Send,
        <F as AsQuery>::Query: 'query,
        Paginated<<F as AsQuery>::Query>: LoadQuery<'query, Self::AsyncConnection, R> + Send,

        'life0: 'async_trait,
        'query: 'async_trait,
        R: 'async_trait,
        P: 'async_trait,
        I: 'async_trait,
        F: 'async_trait,
        Self: 'life0,
    {
        execute_query!(self, R::table().is_not_deleted().multipaginate(pages.into_iter()),).boxed()
    }

    #[framed]
    #[instrument(skip_all)]
    fn insert<'life0, 'async_trait, 'query, 'v, R, V, I>(
        &'life0 self,
        values: I,
    ) -> BoxFuture<'async_trait, Result<Vec<R>, Error>>
    where
        V: HasTable + Send,
        <V as HasTable>::Table: Table + QueryId + Send + 'query,
        <<V as HasTable>::Table as QuerySource>::FromClause: Send,

        Vec<V>: Insertable<<V as HasTable>::Table>,
        <Vec<V> as Insertable<<V as HasTable>::Table>>::Values: Send + 'query,
        R: Send,
        InsertStatement<<V as HasTable>::Table, <Vec<V> as Insertable<<V as HasTable>::Table>>::Values>:
            LoadQuery<'query, Self::AsyncConnection, R>,

        I: IntoIterator<Item = V> + Send,

        R: MaybeAudit<'query, Self::AsyncConnection>,

        'v: 'async_trait + 'life0,
        'life0: 'async_trait,
        Self: 'life0,
        R: 'async_trait,
        V: 'async_trait,
        I: 'async_trait + 'v,
    {
        instrument_err!(self.raw_tx(move |conn| {
            let values = values.into_iter().collect::<Vec<_>>();
            if values.is_empty() {
                return Box::pin(ready(Ok(vec![])));
            }

            async move {
                let all_inserted = diesel::insert_into(V::table())
                    .values(values)
                    .get_results::<R>(conn)
                    .await?;

                R::maybe_insert_audit_records(conn, &all_inserted).await?;

                Ok(all_inserted)
            }
            .scope_boxed()
        }))
        .boxed()
    }

    #[framed]
    #[instrument(skip_all)]
    fn insert_one<'life0, 'async_trait, 'query, 'v, R, V>(
        &'life0 self,
        value: V,
    ) -> BoxFuture<'async_trait, Result<R, Error>>
    where
        V: HasTable + Send,
        <V as HasTable>::Table: Table + QueryId + Send + 'query,
        <<V as HasTable>::Table as QuerySource>::FromClause: Send,

        [V; 1]: Insertable<<V as HasTable>::Table>,
        <[V; 1] as Insertable<<V as HasTable>::Table>>::Values: Send + 'query,
        R: Send,
        InsertStatement<<V as HasTable>::Table, <[V; 1] as Insertable<<V as HasTable>::Table>>::Values>:
            LoadQuery<'query, Self::AsyncConnection, R>,

        R: MaybeAudit<'query, Self::AsyncConnection>,

        'v: 'async_trait + 'life0,
        'life0: 'async_trait,
        Self: 'life0,
        R: 'async_trait,
        V: 'async_trait,
    {
        instrument_err!(self.raw_tx(move |conn| {
            async move {
                let inserted = diesel::insert_into(V::table())
                    .values([value])
                    .get_result::<R>(conn)
                    .await?;

                let inserted = [inserted];
                R::maybe_insert_audit_records(conn, &inserted).await?;
                let [inserted] = inserted;

                Ok(inserted)
            }
            .scope_boxed()
        }))
        .boxed()
    }

    #[framed]
    #[instrument(skip_all)]
    fn update<'life0, 'async_trait, 'query, 'v, R, V, I, T, Pk, F>(
        &'life0 self,
        patches: I,
    ) -> BoxFuture<'async_trait, Result<Vec<R>, Error>>
    where
        V: AsChangeset<Target = T> + HasTable<Table = T> + IncludesChanges + Send + Sync,
        for<'a> &'a V: HasTable<Table = T> + Identifiable<Id = &'a Pk> + IntoUpdateTarget,
        <V as AsChangeset>::Changeset: Send + 'query,
        for<'a> <&'a V as IntoUpdateTarget>::WhereClause: Send,

        ht::Find<T, Pk>: IntoUpdateTarget,
        <ht::Find<T, Pk> as IntoUpdateTarget>::WhereClause: Send + 'query,
        ht::Update<ht::Find<T, Pk>, V>: AsQuery + LoadQuery<'query, Self::AsyncConnection, R> + Send,

        Pk: AsExpression<SqlTypeOf<T::PrimaryKey>> + Clone + Hash + Eq + Send + Sync,

        T: FindDsl<Pk> + Table + Send + 'query,
        ht::Find<T, Pk>: HasTable<Table = T> + Send,
        <T as QuerySource>::FromClause: Send,

        I: IntoIterator<Item = V> + Send,
        R: Send,
        for<'a> &'a R: Identifiable<Id = &'a Pk>,

        R: MaybeAudit<'query, Self::AsyncConnection>,

        T::PrimaryKey: Expression + ExpressionMethods,
        <T::PrimaryKey as Expression>::SqlType: SqlType,
        T: FilterDsl<ht::EqAny<<T as Table>::PrimaryKey, Vec<Pk>>, Output = F>,
        F: IsNotDeleted<'query, Self::AsyncConnection, R, R>,

        'life0: 'async_trait,
        'query: 'async_trait,
        'v: 'async_trait + 'life0,
        R: 'async_trait,
        V: 'async_trait,
        I: 'async_trait + 'v,
        T: 'async_trait,
        Pk: 'async_trait,
        F: 'async_trait,
    {
        let patches = patches.into_iter().collect::<Vec<V>>();
        let ids = patches.iter().map(|patch| patch.id().clone()).collect::<Vec<_>>();

        instrument_err!(self.raw_tx(move |conn| {
            async move {
                let no_change_patch_ids = patches
                    .iter()
                    .filter_map(
                        |patch| {
                            if !patch.includes_changes() {
                                Some(patch.id().to_owned())
                            } else {
                                None
                            }
                        },
                    )
                    .collect::<Vec<_>>();

                let num_changed_patches = ids.len() - no_change_patch_ids.len();
                if num_changed_patches == 0 {
                    return Ok(vec![]);
                }
                let mut all_updated = Vec::with_capacity(num_changed_patches);
                for patch in patches.into_iter().filter(|patch| patch.includes_changes()) {
                    let record = diesel::update(V::table().find(patch.id().to_owned()))
                        .set(patch)
                        .get_result::<R>(conn)
                        .await?;
                    all_updated.push(record);
                }

                R::maybe_insert_audit_records(conn, &all_updated).await?;

                let filter = FilterDsl::filter(V::table(), V::table().primary_key().eq_any(no_change_patch_ids))
                    .is_not_deleted();
                let unchanged_records = filter.get_results::<R>(&mut *conn).await?;

                let mut all_records = unchanged_records
                    .into_iter()
                    .chain(all_updated.into_iter())
                    .map(|record| (record.id().clone(), record))
                    .collect::<HashMap<_, _>>();

                Ok(ids.iter().map(|id| all_records.remove(id).unwrap()).collect::<Vec<_>>())
            }
            .scope_boxed()
        }))
        .boxed()
    }
}

#[async_trait]
impl<'d, D: Db + Clone> Db for Cow<'d, D> {
    type Backend = D::Backend;
    type AsyncConnection = D::AsyncConnection;
    type Connection<'r> = D::Connection<'r> where Self: 'r;
    type TxConnection<'r> = D::TxConnection<'r>;

    async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).query(f).await
    }

    async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).with_tx_connection(f).await
    }

    fn tx_id(&self) -> Option<Uuid> {
        (**self).tx_id()
    }

    #[framed]
    async fn tx_cleanup<F, E>(&self, f: F)
    where
        F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E>,
        E: Into<TxCleanupError> + 'static,
    {
        (**self).tx_cleanup(f).await
    }

    async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
    where
        F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
        E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
        'life0: 'a,
    {
        (**self).tx(callback).await
    }
}

#[async_trait]
impl<'d, D: Db + Clone> Db for &'d D {
    type Backend = D::Backend;
    type AsyncConnection = D::AsyncConnection;
    type Connection<'r> = D::Connection<'r> where Self: 'r;
    type TxConnection<'r> = D::TxConnection<'r>;

    async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).query(f).await
    }

    async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).with_tx_connection(f).await
    }

    fn tx_id(&self) -> Option<Uuid> {
        (**self).tx_id()
    }

    #[framed]
    async fn tx_cleanup<F, E>(&self, f: F)
    where
        F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E>,
        E: Into<TxCleanupError> + 'static,
    {
        (**self).tx_cleanup(f).await
    }

    async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
    where
        F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
        E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
        'life0: 'a,
    {
        (**self).tx(callback).await
    }
}

cfg_if! {
    if #[cfg(any(feature = "bb8", feature = "deadpool", feature = "mobc"))] {
        #[async_trait]
        impl<'d, C> Db for DbConnOwned<'d, C>
        where
            C: AsyncConnection + PoolableConnection + Send + Sync + 'static,
        {
            type Backend = <C as AsyncConnection>::Backend;
            type AsyncConnection = C;
            type Connection<'r> = PooledConnection<'r, C> where Self: 'r;
            type TxConnection<'r> = Cow<'r, DbConnRef<'r, Self::AsyncConnection>>;

            async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + Send + 'a,
                T: Send + 'a
            {
                let mut connection = self.connection.write().await;
                f(connection.deref_mut().deref_mut()).await
            }

            async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + Send + 'a,
                T: Send + 'a
            {
                let mut connection = self.connection.write().await;
                let connection = connection.deref_mut().deref_mut();
                connection.transaction(f).await
            }

            fn tx_id(&self) -> Option<Uuid> {
                self.tx_id
            }

            #[framed]
            async fn tx_cleanup<F, E>(&self, f: F)
            where
                F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E>,
                E: Into<TxCleanupError> + 'static,
            {
                let mut tx_cleanup = self.tx_cleanup.lock().await;
                tx_cleanup.push(Box::new(|x| f(x).map_err(Into::into).boxed()));
            }

            #[framed]
            async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
            where
                F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
                E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
                T: Send + 'a,
                'life0: 'a,
            {
                let tx_cleanup = <Self as AsRef<TxCleanup<Self::AsyncConnection>>>::as_ref(self).clone();
                self.with_tx_connection(move |mut conn| async move {
                    let db_connection = DbConnection {
                        tx_id: Some(Uuid::new_v4()),
                        connection: Arc::new(RwLock::new(conn.deref_mut())),
                        tx_cleanup: tx_cleanup.clone(),
                    };
                    let value = callback.call_tx_fn(Cow::Borrowed(&db_connection)).await?;
                    let mut tx_cleanup = tx_cleanup.lock().await;
                    for tx_cleanup_fn in tx_cleanup.drain(..) {
                        tx_cleanup_fn(&db_connection).await?;
                    }
                    Ok::<T, E>(value)
                }.scope_boxed()).await
            }

            #[framed]
            async fn raw_tx<'a, T, E, F>(&self, callback: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
                T: Send + 'a,
            {
                let tx_cleanup = <Self as AsRef<TxCleanup<Self::AsyncConnection>>>::as_ref(self).clone();
                #[allow(unused_mut)]
                self.with_tx_connection(move |mut conn| async move {
                    let value = callback(conn).await?;

                    let db_connection = DbConnection {
                        tx_id: Some(Uuid::new_v4()),
                        tx_cleanup: tx_cleanup.clone(),
                        connection: Arc::new(RwLock::new(conn.deref_mut())),
                    };
                    let mut tx_cleanup = tx_cleanup.lock().await;
                    for tx_cleanup_fn in tx_cleanup.drain(..) {
                        tx_cleanup_fn(&db_connection).await?;
                    }
                    Ok(value)
                }.scope_boxed()).await
            }
        }

        #[async_trait]
        impl<'d, C> Db for DbConnection<C, &'d mut C>
        where
            C: AsyncConnection + PoolableConnection + Sync + 'static,
        {
            type Backend = <C as AsyncConnection>::Backend;
            type AsyncConnection = C;
            type Connection<'r> = &'d mut C where Self: 'r;
            type TxConnection<'r> = Cow<'r, DbConnRef<'r, Self::AsyncConnection>>;

            async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + Send + 'a,
                T: Send + 'a
            {
                let mut connection = self.connection.write().await;
                f(connection.deref_mut().deref_mut()).await
            }

            async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + Send + 'a,
                T: Send + 'a
            {
                let mut connection = self.connection.write().await;
                let connection = connection.deref_mut().deref_mut();
                connection.transaction(f).await
            }

            fn tx_id(&self) -> Option<Uuid> {
                self.tx_id
            }

            #[framed]
            async fn tx_cleanup<F, E>(&self, f: F)
            where
                F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E>,
                E: Into<TxCleanupError> + 'static,
            {
                let mut tx_cleanup = self.tx_cleanup.lock().await;
                tx_cleanup.push(Box::new(|x| f(x).map_err(Into::into).boxed()));
            }

            async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
            where
                F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
                E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
                T: Send + 'a,
                'life0: 'a,
            {
                let tx_id = self.tx_id;
                let tx_cleanup = <Self as AsRef<TxCleanup<Self::AsyncConnection>>>::as_ref(self).clone();
                self.with_tx_connection(move |conn| {
                    let db_connection = DbConnection {
                        connection: Arc::new(RwLock::new(conn)),
                        tx_cleanup: tx_cleanup.clone(),
                        tx_id,
                    };
                    callback.call_tx_fn(Cow::Owned(db_connection)).scope_boxed()
                })
                .await
            }

            #[framed]
            async fn raw_tx<'a, T, E, F>(&self, callback: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
                T: Send + 'a,
            {
                self.with_tx_connection(move |conn| callback(conn).scope_boxed()).await
            }
        }

        #[async_trait]
        impl<C> Db for crate::pool::Pool<C>
        where
            C: AsyncPoolableConnection + Sync + 'static,
        {
            type Backend = <C as AsyncConnection>::Backend;
            type AsyncConnection = C;
            type Connection<'r> = PooledConnection<'r, C>;
            type TxConnection<'r> = Cow<'r, DbConnRef<'r, Self::AsyncConnection>>;

            async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + Send + 'a,
                T: Send + 'a
            {
                let connection = self.get_connection().await?;
                let mut connection = connection.write().await;
                f(connection.deref_mut().deref_mut()).await
            }

            async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + Send + 'a,
                T: Send + 'a
            {
                let connection = self.get_connection().await?;
                let mut connection = connection.write().await;
                let connection = connection.deref_mut().deref_mut();
                connection.transaction(f).await
            }

            fn tx_id(&self) -> Option<Uuid> {
                None
            }

            async fn tx_cleanup<F, E>(&self, _: F)
            where
                F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E>,
                E: Into<TxCleanupError> + 'static,
            {
            }

            #[framed]
            async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
            where
                F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
                E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
                T: Send + 'a,
                'life0: 'a,
            {
                self.raw_tx(|async_connection| {
                    async move {
                        let tx_cleanup = TxCleanup::default();
                        let db_connection = DbConnection {
                            tx_id: Some(Uuid::new_v4()),
                            connection: Arc::new(RwLock::new(async_connection)),
                            tx_cleanup: tx_cleanup.clone(),
                        };
                        let value = callback.call_tx_fn(Cow::Borrowed(&db_connection)).await?;
                        let mut tx_cleanup = tx_cleanup.lock().await;
                        for tx_cleanup_fn in tx_cleanup.drain(..) {
                            tx_cleanup_fn(&db_connection).await?;
                        }
                        Ok::<T, E>(value)
                    }
                    .scope_boxed()
                })
                .await
            }

            #[framed]
            async fn raw_tx<'a, T, E, F>(&self, callback: F) -> Result<T, E>
            where
                F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>>
                    + Send
                    + 'a,
                E: Debug + From<Error> + From<TxCleanupError> + Send + 'a,
                T: Send + 'a,
            {
                #[allow(unused_mut)]
                self.with_tx_connection(move |mut conn| async move {
                    let value = callback(conn).await?;

                    let tx_cleanup = TxCleanup::default();

                    let db_connection = DbConnection {
                        tx_id: Some(Uuid::new_v4()),
                        tx_cleanup: tx_cleanup.clone(),
                        connection: Arc::new(RwLock::new(conn.deref_mut())),
                    };
                    let mut tx_cleanup = tx_cleanup.lock().await;
                    for tx_cleanup_fn in tx_cleanup.drain(..) {
                        tx_cleanup_fn(&db_connection).await?;
                    }
                    Ok(value)
                }.scope_boxed()).await
            }
        }
    }
}
