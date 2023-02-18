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

mod audit;
mod connection;
mod deletable;
mod is_deleted;
mod macros;
mod operations;
mod paginate;
mod schema;

pub use audit::*;
pub use connection::*;
pub use deletable::*;
pub use is_deleted::*;
pub use macros::*;
pub use operations::*;
pub use paginate::*;
pub use schema::*;

cfg_if! {
    if #[cfg(any(feature = "bb8", feature = "deadpool", feature = "mobc"))] {
        mod pool;
        pub use pool::*;
    }
}
