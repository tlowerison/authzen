use crate::diesel::connection::{DbConnOwned, DieselPooledConnection};
use ::derivative::Derivative;
use ::derive_more::*;
use ::diesel_async::pooled_connection::{self as pc, PoolableConnection};
use ::diesel_async::AsyncConnection;
use ::std::sync::Arc;
use ::uuid::Uuid;

pub trait AsyncPoolableConnection: AsyncConnection + PoolableConnection + 'static {}

/// Pool wraps an inner connection pool, allowing it to be
/// cloned and ignored during Debug
#[derive(Derivative, From, IsVariant)]
#[derivative(Clone(bound = ""), Debug)]
pub enum Pool<C: AsyncPoolableConnection> {
    #[cfg(feature = "diesel-bb8")]
    Bb8(#[derivative(Debug = "ignore")] Arc<pc::bb8::Pool<C>>),
    #[cfg(feature = "diesel-deadpool")]
    Deadpool(#[derivative(Debug = "ignore")] Arc<pc::deadpool::Pool<C>>),
    #[cfg(feature = "diesel-mobc")]
    Mobc(#[derivative(Debug = "ignore")] Arc<pc::mobc::Pool<C>>),
}

#[cfg(feature = "diesel-bb8")]
impl<C: AsyncPoolableConnection> Pool<C> {
    pub fn bb8(inner_pool: pc::bb8::Pool<C>) -> Self {
        Self::Bb8(Arc::new(inner_pool))
    }
}

#[cfg(feature = "diesel-deadpool")]
impl<C: AsyncPoolableConnection> Pool<C> {
    pub fn deadpool(inner_pool: pc::deadpool::Pool<C>) -> Self {
        Self::Deadpool(Arc::new(inner_pool))
    }
}

#[cfg(feature = "diesel-mobc")]
impl<C: AsyncPoolableConnection> Pool<C> {
    pub fn mobc(inner_pool: pc::mobc::Pool<C>) -> Self {
        Self::Mobc(Arc::new(inner_pool))
    }
}

#[cfg(feature = "diesel-mysql")]
impl AsyncPoolableConnection for diesel_async::AsyncMysqlConnection {}

#[cfg(feature = "diesel-postgres")]
impl AsyncPoolableConnection for diesel_async::AsyncPgConnection {}

impl<C: AsyncPoolableConnection> Pool<C> {
    pub(crate) async fn get_connection(&self) -> Result<DbConnOwned<C, Uuid>, diesel::result::Error> {
        let connection = match self {
            #[cfg(feature = "diesel-bb8")]
            Self::Bb8(pool) => DieselPooledConnection::Bb8(pool.get().await.map_err(|err| {
                diesel::result::Error::QueryBuilderError(
                    format!("could not get pooled connection to database: {err}").into(),
                )
            })?),
            #[cfg(feature = "diesel-deadpool")]
            Self::Deadpool(pool) => DieselPooledConnection::Deadpool(
                pool.get().await.map_err(|err| {
                    diesel::result::Error::QueryBuilderError(
                        format!("could not get pooled connection to database: {err}").into(),
                    )
                })?,
                Default::default(),
            ),
            #[cfg(feature = "diesel-mobc")]
            Self::Mobc(pool) => DieselPooledConnection::Mobc(
                pool.get().await.map_err(|err| {
                    diesel::result::Error::QueryBuilderError(
                        format!("could not get pooled connection to database: {err}").into(),
                    )
                })?,
                Default::default(),
            ),
        };
        Ok(DbConnOwned::from(connection))
    }
}
