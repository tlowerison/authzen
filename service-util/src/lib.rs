#![cfg_attr(feature = "client", allow(incomplete_features))]
#![cfg_attr(
    feature = "client",
    feature(associated_type_defaults, const_caller_location, specialization,)
)]

#[macro_use]
extern crate async_backtrace;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate derivative;

mod env;
mod error;

pub use env::*;
pub use error::*;

#[doc(hidden)]
pub use lazy_static::lazy_static as service_util_lazy_static;
#[doc(hidden)]
pub use paste::paste as service_util_paste;

cfg_if! {
    if #[cfg(feature = "client")] {
        mod client;
        pub use client::*;
    }
}
cfg_if! {
    if #[cfg(feature = "db")] {
        mod paginate;
        pub use paginate::*;
    } else if #[cfg(feature = "graphql")] {
        mod paginate;
        pub use paginate::*;
    }
}
cfg_if! {
    if #[cfg(feature = "server")] {
        mod server;
        pub use server::*;

        #[doc(hidden)]
        pub use tokio as service_util_tokio;

        #[macro_export]
        macro_rules! try_join_safe {
            ($($expr:expr),+ $(,)?) => { $crate::try_join_safe!($($expr,)+ ~ @ _a ~ $($expr,)+) };
            ($($orig:expr,)* ~ $($final_ident:ident,)* @ $ident:ident ~ $expr:expr, $($remaining:expr,)*) => {
                $crate::service_util_paste! {
                    $crate::try_join_safe! { $($orig,)* ~ $($final_ident,)* $ident, @ [<_ $ident>] ~ $($remaining,)* }
                }
            };
            ( $($expr:expr,)* ~ $($final_ident:ident,)* @ $ident:ident ~) => {
                {
                    let ($($final_ident),*) = $crate::service_util_tokio::join!($($expr),+);
                    $crate::try_join_safe!( @ @ $($final_ident)* )
                }
            };
            (@ $($used:ident)* @ $ident:ident $($remaining:ident)*) => {
                $ident.and_then(move |$ident| $crate::try_join_safe!( @ $($used)* $ident @ $($remaining)* ))
            };
            (@ $($used:ident)* @) => {
                Ok(($($used),*))
            };
        }

        #[allow(clippy::manual_async_fn)]
        pub fn try_join_all_safe<I, T: Send, E: Send>(iter: I) -> impl futures::Future<Output = Result<Vec<T>, E>> + Send
        where
            I: IntoIterator + Send,
            <I as IntoIterator>::Item: futures::Future<Output = Result<T, E>> + Send,
        {
            async move {
                futures::future::join_all(iter).await.into_iter().collect()
            }
        }
    }
}
cfg_if! {
    if #[cfg(feature = "trace")] {
        mod trace;
        pub use trace::*;
    }
}
