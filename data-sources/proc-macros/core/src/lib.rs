#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

#[cfg(feature = "diesel")]
pub mod diesel;

mod transactional_data_source;

pub use transactional_data_source::*;

#[doc(hidden)]
pub mod reexports {
    pub use async_trait;
    pub use scoped_futures;
}
