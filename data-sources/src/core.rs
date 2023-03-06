use ::async_backtrace::{backtrace, framed, Location};
use ::async_trait::async_trait;
use ::derivative::Derivative;
use ::derive_more::*;
use ::futures::future::BoxFuture;
use ::scoped_futures::ScopedBoxFuture;
use ::serde::Serialize;
use ::std::borrow::Cow;
use ::std::fmt::{Debug, Display};
use ::std::hash::Hash;
use ::std::ops::Deref;
use ::std::sync::Arc;
use ::tokio::sync::{Mutex, RwLock};

#[derive(AsRef, AsMut, Deref, DerefMut, Derivative)]
#[derivative(Debug)]
pub struct DataSourceConnection<AC, C, TransactionId> {
    #[deref]
    #[deref_mut]
    #[derivative(Debug = "ignore")]
    pub(crate) connection: Arc<RwLock<C>>,
    #[derivative(Debug = "ignore")]
    pub(crate) tx_cleanup: TxCleanup<AC, TransactionId>,
    pub(crate) tx_id: Option<TransactionId>,
}

impl<AC, C, TransactionId: Clone> Clone for DataSourceConnection<AC, C, TransactionId> {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            tx_cleanup: self.tx_cleanup.clone(),
            tx_id: self.tx_id.clone(),
        }
    }
}

pub type DataSourceConnRef<'a, C, TransactionId> = DataSourceConnection<C, &'a mut C, TransactionId>;

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

pub trait TxCleanupFn<'r, AC: 'r, E, TransactionId: 'r> =
    FnOnce(&'r DataSourceConnRef<'r, AC, TransactionId>) -> BoxFuture<'r, Result<(), E>> + Send + Sync + 'static;

pub type TxCleanup<AC, TransactionId> =
    Arc<Mutex<Vec<Box<dyn for<'r> TxCleanupFn<'r, AC, TxCleanupError, TransactionId>>>>>;

impl<AC, C, TransactionId: Clone> From<DataSourceConnection<AC, C, TransactionId>>
    for Cow<'_, DataSourceConnection<AC, C, TransactionId>>
{
    fn from(value: DataSourceConnection<AC, C, TransactionId>) -> Self {
        Cow::Owned(value)
    }
}

impl<'a, AC, C, TransactionId: Clone> From<&'a DataSourceConnection<AC, C, TransactionId>>
    for Cow<'a, DataSourceConnection<AC, C, TransactionId>>
{
    fn from(value: &'a DataSourceConnection<AC, C, TransactionId>) -> Self {
        Cow::Borrowed(value)
    }
}

/// A client for communicating with a data source. Typically this should be implemented for
/// connection or client implementations for that backend, e.g. [`diesel_async::AsyncPgConnection`](https://docs.rs/diesel-async/latest/diesel_async/pg/struct.AsyncPgConnection.html).
pub trait DataSource: Clone + Debug + Send + Sync {
    /// The backend this client will act upon.
    type Backend;
    type Error: Debug;
    /// The type for ids associated with transactions used by this client.
    /// If this client does not support transactions just set this value to `()`.
    type TransactionId: Clone + Debug + Eq + Hash + Send + Serialize + Sync = ();

    /// Returns the current transaction id if there is one available
    /// for this client. Must return Some for clients which expect
    /// to use a transaction cache to assist an authorization engine. If
    /// this client does not support transactions just return `None`.
    fn transaction_id(&self) -> Option<Self::TransactionId>;
}

#[async_trait]
pub trait TransactionalDataSource: DataSource + Sized {
    type AsyncConnection: Send + Sync + 'static;
    type Connection<'r>: Deref<Target = Self::AsyncConnection> + Send + Sync
    where
        Self: 'r;
    type TxConnection<'r>: TransactionalDataSource<Backend = Self::Backend, AsyncConnection = Self::AsyncConnection>;

    async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Self::Error> + Send + 'a,
        T: Send + 'a;

    async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Self::Error> + Send + 'a,
        T: Send + 'a;

    async fn tx_cleanup<F, E>(&self, f: F)
    where
        F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E, Self::TransactionId>,
        E: Into<TxCleanupError> + 'static;

    async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
    where
        F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
        E: Debug + From<Self::Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
        'life0: 'a;

    async fn raw_tx<'a, T, E, F>(&self, callback: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Self::Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
    {
        self.with_tx_connection(callback).await
    }
}

impl<'d, D: DataSource + Clone> DataSource for &'d D {
    type Backend = D::Backend;
    type Error = D::Error;
    type TransactionId = D::TransactionId;

    fn transaction_id(&self) -> Option<Self::TransactionId> {
        (**self).transaction_id()
    }
}

impl<'d, D: DataSource + Clone> DataSource for Cow<'d, D> {
    type Backend = D::Backend;
    type Error = D::Error;
    type TransactionId = D::TransactionId;

    fn transaction_id(&self) -> Option<Self::TransactionId> {
        (**self).transaction_id()
    }
}

#[async_trait]
impl<'d, D: TransactionalDataSource + Clone> TransactionalDataSource for &'d D {
    type AsyncConnection = D::AsyncConnection;
    type Connection<'r> = D::Connection<'r> where Self: 'r;
    type TxConnection<'r> = D::TxConnection<'r>;

    async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Self::Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).query(f).await
    }

    async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Self::Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).with_tx_connection(f).await
    }

    #[framed]
    async fn tx_cleanup<F, E>(&self, f: F)
    where
        F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E, Self::TransactionId>,
        E: Into<TxCleanupError> + 'static,
    {
        (**self).tx_cleanup(f).await
    }

    async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
    where
        F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
        E: Debug + From<Self::Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
        'life0: 'a,
    {
        (**self).tx(callback).await
    }
}

#[async_trait]
impl<'d, D: TransactionalDataSource + Clone> TransactionalDataSource for Cow<'d, D> {
    type AsyncConnection = D::AsyncConnection;
    type Connection<'r> = D::Connection<'r> where Self: 'r;
    type TxConnection<'r> = D::TxConnection<'r>;

    async fn query<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Self::Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).query(f).await
    }

    async fn with_tx_connection<'a, F, T, E>(&self, f: F) -> Result<T, E>
    where
        F: for<'r> FnOnce(&'r mut Self::AsyncConnection) -> ScopedBoxFuture<'a, 'r, Result<T, E>> + Send + 'a,
        E: Debug + From<Self::Error> + Send + 'a,
        T: Send + 'a,
    {
        (**self).with_tx_connection(f).await
    }

    #[framed]
    async fn tx_cleanup<F, E>(&self, f: F)
    where
        F: for<'r> TxCleanupFn<'r, Self::AsyncConnection, E, Self::TransactionId>,
        E: Into<TxCleanupError> + 'static,
    {
        (**self).tx_cleanup(f).await
    }

    async fn tx<'life0, 'a, T, E, F>(&'life0 self, callback: F) -> Result<T, E>
    where
        F: for<'r> TxFn<'a, Self::TxConnection<'r>, ScopedBoxFuture<'a, 'r, Result<T, E>>>,
        E: Debug + From<Self::Error> + From<TxCleanupError> + Send + 'a,
        T: Send + 'a,
        'life0: 'a,
    {
        (**self).tx(callback).await
    }
}
