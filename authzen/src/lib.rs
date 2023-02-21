#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

pub use core::*;
pub use proc_macros::*;

pub use proc_macros;

#[doc(hidden)]
pub use derivative;

#[doc(alias = "authzen_service_util")]
pub use service_util;

/// Decision maker client implementations provided for convienence.
pub mod decision_makers {
    #[cfg(feature = "opa-decision-maker")]
    #[doc(alias = "authzen_opa")]
    pub use authzen_opa as opa;
}

/// Storage backend client implementations provided for convienence.
pub mod storage_backends {
    #[cfg(feature = "diesel-storage-backend")]
    #[doc(alias = "authzen_diesel")]
    pub use authzen_diesel as diesel;
}
