#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

use cfg_if::cfg_if;

pub use core::*;
pub use proc_macros::*;

pub use proc_macros;

#[doc(alias = "authzen_service_util")]
pub use service_util;

/// Implementations of common decision maker clients.
pub mod decision_makers {
    #[cfg(feature = "opa-decision-maker")]
    #[doc(alias = "authzen_opa")]
    pub use authzen_opa as opa;
}

/// Implementations of common storage backend clients.
pub mod storage_backends {
    #[cfg(feature = "diesel-storage-backend")]
    #[doc(alias = "authzen_diesel")]
    pub use authzen_diesel as diesel;
}

#[doc(hidden)]
pub use derivative;

#[doc(hidden)]
pub use futures;

cfg_if! {
    if #[cfg(feature = "policy-information-point-server")] {
        #[doc(hidden)]
        pub use dotenv;

        #[doc(hidden)]
        pub use tokio;
    }
}
