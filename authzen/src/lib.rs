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

/// Implementations of common authorization engine clients.
pub mod authz_engines {
    #[cfg(feature = "opa-authz-engine")]
    #[doc(alias = "authzen_opa")]
    pub use authzen_opa as opa;
}

/// Implementations of common data source clients.
#[doc(alias = "authzen_data_sources")]
pub use authzen_data_sources as data_sources;

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

pub mod prelude {
    pub use crate::*;
    pub use data_sources::prelude::*;
}
