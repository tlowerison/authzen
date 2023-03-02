#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

pub use core::*;
pub use proc_macros::*;

#[doc(hidden)]
pub use async_trait::async_trait as authzen_opa_async_trait;
#[doc(hidden)]
pub use authzen_session;
#[doc(hidden)]
pub use serde as authzen_opa_serde;
