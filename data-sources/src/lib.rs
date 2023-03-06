#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]
#![allow(incomplete_features)]
#![feature(associated_type_defaults, trait_alias)]
#![cfg_attr(feature = "diesel", feature(specialization))]

pub mod core;
pub mod prelude;

#[cfg(feature = "diesel")]
pub mod diesel;

pub use crate::core::*;

#[doc(hidden)]
pub use authzen_data_sources_proc_macros as proc_macros;

#[doc(hidden)]
pub use authzen_data_sources_proc_macros_core as proc_macros_core;
