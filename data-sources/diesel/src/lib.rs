#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

pub use authzen_diesel_core::*;
pub use authzen_diesel_proc_macros::*;

pub mod prelude {
    pub use authzen_diesel_core::prelude::*;
    pub use authzen_diesel_proc_macros::*;
}

#[doc(hidden)]
pub use anyhow;
#[doc(hidden)]
pub use async_trait::async_trait as authzen_diesel_async_trait;
#[doc(hidden)]
pub use chrono;
#[doc(hidden)]
pub use derivative::Derivative as DieselUtilDerivative;
#[doc(hidden)]
pub use diesel;
#[doc(hidden)]
pub use lazy_static::lazy_static as authzen_diesel_lazy_static;
#[doc(hidden)]
pub use paste::paste;
#[doc(hidden)]
pub use scoped_futures;
#[doc(hidden)]
pub use serde_json;
#[doc(hidden)]
pub use tokio;
#[doc(hidden)]
pub use uuid;
