use crate::{DbConnOwned, DbConnection};
use diesel_async::AsyncConnection;
use std::sync::Arc;

use diesel_async::pooled_connection::PoolableConnection;

#[cfg(feature = "bb8")]
pub use diesel_async::pooled_connection::bb8::Pool as InnerPool;

#[cfg(feature = "deadpool")]
pub use diesel_async::pooled_connection::deadpool::Pool as InnerPool;

#[cfg(feature = "mobc")]
pub use diesel_async::pooled_connection::mobc::Pool as InnerPool;

/// Pool wraps an inner connection pool, allowing it to be
/// cloned and ignored during Debug
#[derive(Derivative, Deref, DerefMut, From, Into)]
#[derivative(Debug)]
pub struct Pool<C: AsyncPoolableConnection>(#[derivative(Debug = "ignore")] pub Arc<InnerPool<C>>);

impl<C: AsyncPoolableConnection> Pool<C> {
    pub fn new(inner_pool: InnerPool<C>) -> Self {
        Self(Arc::new(inner_pool))
    }
}

impl<C: AsyncPoolableConnection> Clone for Pool<C> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

cfg_if! {
    if #[cfg(any(feature = "bb8", feature = "deadpool"))] {
        pub trait AsyncPoolableConnection: AsyncConnection + PoolableConnection + 'static {}
    }
}

cfg_if! {
    if #[cfg(feature = "mobc")] {
        pub trait AsyncPoolableConnection: AsyncConnection + PoolableConnection + 'static {}
    }
}

#[cfg(feature = "mysql")]
impl AsyncPoolableConnection for diesel_async::AsyncMysqlConnection {}

#[cfg(feature = "postgres")]
impl AsyncPoolableConnection for diesel_async::AsyncPgConnection {}

impl<C: AsyncPoolableConnection> Pool<C> {
    pub(crate) async fn get_connection(&self) -> Result<DbConnOwned<C>, diesel::result::Error> {
        let connection = self.get().await.map_err(|err| {
            diesel::result::Error::QueryBuilderError(
                format!("could not get pooled connection to database: {err}").into(),
            )
        })?;
        Ok(DbConnection::from(connection))
    }
}
