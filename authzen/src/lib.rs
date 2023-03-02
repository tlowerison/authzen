#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

use cfg_if::cfg_if;

pub use authzen_core::*;
pub use authzen_proc_macros::*;

pub use authzen_proc_macros as proc_macros;

#[cfg(feature = "proc-macro-util")]
#[doc(alias = "authzen_proc_macro_util")]
pub use authzen_proc_macro_util as proc_macro_util;

#[cfg(feature = "service-util")]
#[doc(alias = "authzen_service_util")]
pub use authzen_service_util as service_util;

#[cfg(feature = "session")]
#[doc(alias = "authzen_session")]
pub use authzen_session as session;

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
