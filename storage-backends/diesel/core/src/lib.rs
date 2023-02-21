#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]
#![allow(incomplete_features)]
#![feature(associated_type_defaults, specialization, trait_alias)]

#[macro_use]
extern crate async_backtrace;
#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate derive_more;
#[doc(hidden)]
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde;
#[macro_use]
extern crate tracing;

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

#[cfg(any(feature = "bb8", feature = "deadpool", feature = "mobc"))]
mod pool;

pub use _operations::operations;

pub mod prelude {
    pub use crate::_operations::operations::*;
    pub use crate::_operations::{DbEntity, DbEntityError};
    pub use crate::audit::*;
    pub use crate::connection::{TxCleanup, TxCleanupError, TxCleanupFn, TxFn};
    pub use crate::deletable::*;
    pub use crate::macros::*;
    pub use crate::paginate::*;
    pub use crate::schema::*;

    #[cfg(any(feature = "bb8", feature = "deadpool", feature = "mobc"))]
    pub use crate::pool::*;
}
