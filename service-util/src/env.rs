use std::str::FromStr;
use thiserror::Error;

// env produces for each input:
// - a pub const with the same name as the provided identifier
// - a fn which attempts to extract that environment variable and parse it into the specified type
#[macro_export]
macro_rules! env {
    () => {};
    ($var:ident: $($tt:tt)*) => { $crate::service_util_paste! {
        pub const $var: &str = stringify!($var);
        fn [<has_set_ $var:lower>]() -> bool { [<$var _VALUE>].read().unwrap().is_some() }

        $crate::env! { @ $var $($tt)* }
    } };
    (@ $var:ident Option<$ty:ty> $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<Option<$ty>, service_util::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<Option<$ty>, service_util::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_opt_env($var);
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident $ty:ty $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<$ty, service_util::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<$ty, service_util::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_env($var);
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident $ty:ty = $expr:expr $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<$ty, service_util::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<$ty, service_util::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_opt_env($var).map(|x| x.unwrap_or_else(|| { $expr }.into()));
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident Option<$ty:ty> | $map_fn:path $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<Option<$ty>, service_util::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<Option<$ty>, service_util::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_opt_env($var).map(|x| x.map($map_fn));
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $($crate::env! { $($tt)* })?
    } };
    (@ $var:ident $ty:ty | $map_fn:path $(, $($tt:tt)*)?) => { $crate::service_util_paste! {
        $crate::service_util_lazy_static! {
            static ref [<$var _VALUE>]: std::sync::Arc<std::sync::RwLock<Option<Result<$ty, service_util::EnvError>>>> = std::sync::Arc::new(std::sync::RwLock::new(None));
        }

        pub fn [<$var:lower>]() -> Result<$ty, service_util::EnvError> {
            if [<has_set_ $var:lower>]() {
                [<$var _VALUE>].read().unwrap().as_ref().unwrap().clone()
            } else {
                let res = $crate::service_util_env($var).map($map_fn);
                let mut write_lock = [<$var _VALUE>].write().unwrap();
                *write_lock = Some(res.clone());
                res
            }
        }

        $($crate::env! { $($tt)* })?
    } };
}

#[derive(Clone, Debug, Error)]
pub enum EnvError {
    #[error("invalid value for environment variable `{0}`")]
    InvalidValue(&'static str),
    #[error("missing required environment variable `{0}`")]
    Missing(&'static str),
}

pub fn service_util_env<T: FromStr>(var_name: &'static str) -> Result<T, EnvError> {
    service_util_opt_env(var_name)?.ok_or(EnvError::Missing(var_name))
}

pub fn service_util_opt_env<T: FromStr>(var_name: &'static str) -> Result<Option<T>, EnvError> {
    if let Ok(value) = std::env::var(var_name) {
        value.parse().map(Some).or(Err(EnvError::InvalidValue(var_name)))
    } else {
        Ok(None)
    }
}

pub fn parse_allowed_origins(allowed_origins: String) -> Vec<hyper::http::HeaderValue> {
    allowed_origins
        .split(',')
        .map(TryFrom::try_from)
        .collect::<Result<_, _>>()
        .unwrap()
}
