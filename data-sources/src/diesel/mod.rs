pub mod audit;
pub mod connection;
pub mod deletable;
pub mod is_deleted;
pub mod paginate;

#[doc(hidden)]
pub mod macros;
#[doc(hidden)]
pub mod schema;

mod _operations;

#[cfg(any(feature = "diesel-bb8", feature = "diesel-deadpool", feature = "diesel-mobc"))]
pub mod pool;

pub use _operations::{operations, DbEntity, DbEntityError};

pub mod prelude {
    pub use crate::diesel::_operations::operations::*;
    pub use crate::diesel::_operations::{DbEntity, DbEntityError};
    pub use crate::diesel::audit::*;
    pub use crate::diesel::deletable::*;
    pub use crate::diesel::macros::*;
    pub use crate::diesel::paginate::*;
    pub use crate::diesel::schema::*;

    #[cfg(any(feature = "diesel-bb8", feature = "diesel-deadpool", feature = "diesel-mobc"))]
    pub use crate::diesel::pool::*;
}
