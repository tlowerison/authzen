use diesel::pg::Pg;
use diesel_async::AsyncPgConnection;
use diesel_util::_Db;

pub mod models;
pub mod schema;

pub use models::*;

// trait alias workaround
pub trait Db: _Db<AsyncConnection = AsyncPgConnection, Backend = Pg> {}

impl<D: _Db<AsyncConnection = AsyncPgConnection, Backend = Pg>> Db for D {}
