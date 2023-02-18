use crate::env::EnvError;
use async_backtrace::{backtrace, Location};
use hyper::StatusCode;
use std::fmt::{Display, Formatter};

#[cfg(feature = "client")]
use crate::BaseClientError;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Error {
    pub status_code: StatusCode,
    pub msg: Option<String>,
    pub details: Option<String>,
    #[derivative(Debug = "ignore")]
    pub backtrace: Option<Box<[Location]>>,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let Error { status_code, msg, .. } = &self;

        #[cfg(feature = "log_error")]
        {
            let Error { details, backtrace, .. } = &self;
            let mut log_err_msg = format!(" - status_code: {status_code}");
            if let Some(msg) = msg.as_ref() {
                log_err_msg = format!("{log_err_msg}\n - msg: {msg}");
            }
            if let Some(details) = details.as_ref() {
                log_err_msg = format!("{log_err_msg}\n - details: {details}");
            }
            if let Some(backtrace) = backtrace.as_ref() {
                let backtrace = backtrace.iter().map(|l| l.to_string()).collect::<Vec<_>>().join("\n");
                log_err_msg = format!("{log_err_msg}\n - backtrace:\n{backtrace}");
            }

            tracing::error!("{log_err_msg}");
        }

        if let Some(canonical_reason) = status_code.canonical_reason() {
            write!(f, "{canonical_reason}")?;
            if let Some(msg) = msg.as_ref() {
                write!(f, ": {msg}")?;
            }
        } else if let Some(msg) = msg.as_ref() {
            write!(f, "{msg}")?;
        }

        Ok(())
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn init(
        status_code: impl Into<StatusCode>,
        msg: impl Into<Option<String>>,
        details: impl Into<Option<String>>,
    ) -> Self {
        Self {
            status_code: status_code.into(),
            msg: msg.into(),
            details: details.into(),
            backtrace: backtrace(),
        }
    }

    pub fn init_with_backtrace(
        status_code: impl Into<StatusCode>,
        msg: impl Into<Option<String>>,
        details: impl Into<Option<String>>,
        backtrace: impl Into<Option<Box<[Location]>>>,
    ) -> Self {
        Self {
            status_code: status_code.into(),
            msg: msg.into(),
            details: details.into(),
            backtrace: backtrace.into(),
        }
    }

    #[framed]
    pub fn new(status_code: impl Into<StatusCode>) -> Self {
        Error::init(status_code, None, None)
    }

    #[framed]
    pub fn status_map<E: Display>(status_code: impl Into<StatusCode>) -> Box<dyn Fn(E) -> Self> {
        let status_code = status_code.into();
        Box::new(move |err: E| Error::init(status_code, None, format!("{err}")))
    }

    #[framed]
    pub fn msg(status_code: impl Into<StatusCode>, err: impl Display) -> Self {
        Error::init(status_code, format!("{err}"), None)
    }

    #[framed]
    pub fn bad_request() -> Self {
        Error::init(StatusCode::BAD_REQUEST, None, None)
    }

    #[framed]
    pub fn default_msg(err: impl Display) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, format!("{err}"), None)
    }

    #[framed]
    pub fn bad_request_msg(err: impl Display) -> Self {
        Error::init(StatusCode::BAD_REQUEST, format!("{err}"), None)
    }

    #[framed]
    pub fn details(status_code: impl Into<StatusCode>, err: impl Display) -> Self {
        Error::init(status_code, None, format!("{err}"))
    }

    #[framed]
    pub fn default_details(err: impl Display) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
    }

    #[framed]
    pub fn default_msg_and_details(msg: impl Display, details: impl Display) -> Self {
        Error::init(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("{msg}"),
            format!("{details}"),
        )
    }

    #[framed]
    pub fn bad_request_details(err: impl Display) -> Self {
        Error::init(StatusCode::BAD_REQUEST, None, format!("{err}"))
    }

    #[cfg(feature = "graphql")]
    pub fn graphql(self) -> async_graphql::Error {
        use async_graphql::ErrorExtensions;
        async_graphql::Error::new(self.msg.map(std::borrow::Cow::from).unwrap_or_else(|| {
            self.status_code
                .canonical_reason()
                .map(std::borrow::Cow::from)
                .unwrap_or_default()
        }))
        .extend_with(|_, extensions| extensions.set("status", self.status_code.as_u16()))
    }
}

impl Default for Error {
    #[framed]
    fn default() -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, None)
    }
}

impl From<EnvError> for Error {
    #[framed]
    fn from(err: EnvError) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
    }
}

impl From<anyhow::Error> for Error {
    #[framed]
    fn from(err: anyhow::Error) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    #[framed]
    fn from(err: std::sync::PoisonError<T>) -> Self {
        Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
    }
}

impl From<(StatusCode, String)> for Error {
    fn from((status_code, msg): (StatusCode, String)) -> Self {
        Self::msg(status_code, msg)
    }
}

#[cfg(all(feature = "server", feature = "axum-05"))]
impl axum_05::response::IntoResponse for Error {
    fn into_response(self) -> axum_05::response::Response {
        let body = match self.msg {
            Some(msg) => axum_05::body::boxed(axum_05::body::Full::from(msg)),
            None => axum_05::body::boxed(axum_05::body::Empty::new()),
        };

        axum_05::response::Response::builder()
            .status(self.status_code)
            .body(body)
            .unwrap()
    }
}

#[cfg(all(feature = "server", feature = "axum-06"))]
impl axum_06::response::IntoResponse for Error {
    fn into_response(self) -> axum_06::response::Response {
        let body = match self.msg {
            Some(msg) => axum_06::body::boxed(axum_06::body::Full::from(msg)),
            None => axum_06::body::boxed(axum_06::body::Empty::new()),
        };

        axum_06::response::Response::builder()
            .status(self.status_code)
            .body(body)
            .unwrap()
    }
}

#[cfg(feature = "grpc")]
impl From<Error> for tonic::Status {
    fn from(error: Error) -> Self {
        if error.status_code.is_server_error() {
            return tonic::Status::new(tonic::Code::Internal, format!("{error}"));
        }
        tonic::Status::new(tonic::Code::Unknown, format!("{error}"))
    }
}

#[cfg(feature = "client")]
impl From<BaseClientError> for Error {
    #[framed]
    fn from(base_client_error: BaseClientError) -> Self {
        match base_client_error {
            BaseClientError::BodyTooLarge => Self::default(),
            BaseClientError::InvalidUri(invalid_uri) => Self::default_details(invalid_uri),
            BaseClientError::NetworkError(err) => Self::default_details(err),
            BaseClientError::RequestBodyBuild(err) => Self::default_details(err),
            BaseClientError::RequestBodySerialization(err) => Self::default_details(err),
            BaseClientError::RequestParamsSerialization(err) => Self::default_details(err),
            BaseClientError::Response { status, message } => Self::details(status, message),
            BaseClientError::ResponseBodyDeserialization(err) => Self::default_details(err),
            BaseClientError::ResponseBodyInvalidCharacter(err) => Self::default_details(err),
        }
    }
}

#[cfg(feature = "client")]
impl<Body> From<hyper::Response<Body>> for Error {
    default fn from(response: hyper::Response<Body>) -> Self {
        let status = response.status();
        if !(status.is_client_error() || status.is_server_error()) {
            unimplemented!();
        }

        Error::init(status, None, None)
    }
}

#[cfg(feature = "client")]
impl From<hyper::Response<Vec<u8>>> for Error {
    #[framed]
    fn from(response: hyper::Response<Vec<u8>>) -> Self {
        let status = response.status();
        if !(status.is_client_error() || status.is_server_error()) {
            unimplemented!();
        }

        let details = std::str::from_utf8(response.body().as_slice()).map(String::from).ok();

        Error::init(status, None, details)
    }
}

#[cfg(feature = "db")]
pub use db::*;
#[cfg(feature = "db")]
mod db {
    use super::*;

    use async_backtrace::{backtrace, Location};
    use diesel::result::DatabaseErrorKind;
    use diesel_util::{DbEntityError, TxCleanupError};
    use std::fmt::{Debug, Display};

    /// DbError is a simplified representation of a diesel Error
    /// It largely exists to make service code handling db errors
    /// able to make business decisions without having to handle
    /// the complexity of potentially every database failure in
    /// interaction with the database.
    #[derive(Derivative, thiserror::Error)]
    #[derivative(Debug)]
    pub enum DbError {
        #[error("db error: application error: {0}")]
        Application(anyhow::Error, #[derivative(Debug = "ignore")] Option<Box<[Location]>>),
        #[error("db error: database could not process request: {0}")]
        BadRequest(
            diesel::result::Error,
            #[derivative(Debug = "ignore")] Option<Box<[Location]>>,
        ),
        #[error("db error: bad request: {0}")]
        CustomBadRequest(anyhow::Error, #[derivative(Debug = "ignore")] Option<Box<[Location]>>),
        #[error("invalid db state")]
        InvalidDbState(anyhow::Error, #[derivative(Debug = "ignore")] Option<Box<[Location]>>),
        #[error("db error: network error while communiciating with database: {0}")]
        Network(
            diesel::result::Error,
            #[derivative(Debug = "ignore")] Option<Box<[Location]>>,
        ),
        #[error("db error: {0}")]
        Other(
            diesel::result::Error,
            #[derivative(Debug = "ignore")] Option<Box<[Location]>>,
        ),
        #[error("db error: application performed an invalid db operation: {0}")]
        Server(
            diesel::result::Error,
            #[derivative(Debug = "ignore")] Option<Box<[Location]>>,
        ),
        #[error("db error: could not commit transaction, another concurrent has locked affected rows")]
        Stale(#[derivative(Debug = "ignore")] Option<Box<[Location]>>),
        #[error("transaction cleanup error: {0}")]
        TxCleanup(#[from] TxCleanupError),
    }

    impl DbError {
        #[framed]
        pub fn application<M: Debug + Display + Send + Sync + 'static>(msg: M) -> DbError {
            DbError::Application(anyhow::Error::msg(msg), backtrace())
        }
        #[framed]
        pub fn bad_request<M: Debug + Display + Send + Sync + 'static>(msg: M) -> DbError {
            DbError::CustomBadRequest(anyhow::Error::msg(msg), backtrace())
        }
        #[framed]
        pub fn invalid_db_state<M: Debug + Display + Send + Sync + 'static>(msg: M) -> DbError {
            DbError::InvalidDbState(anyhow::Error::msg(msg), backtrace())
        }
    }

    impl From<std::convert::Infallible> for DbError {
        fn from(_: std::convert::Infallible) -> Self {
            unreachable!()
        }
    }

    impl From<diesel::result::Error> for DbError {
        #[framed]
        fn from(error: diesel::result::Error) -> Self {
            match &error {
                diesel::result::Error::InvalidCString(_) => DbError::BadRequest(error, backtrace()),
                diesel::result::Error::DatabaseError(kind, _) => match kind {
                    DatabaseErrorKind::UniqueViolation => DbError::BadRequest(error, backtrace()),
                    DatabaseErrorKind::ForeignKeyViolation => DbError::BadRequest(error, backtrace()),
                    DatabaseErrorKind::UnableToSendCommand => DbError::BadRequest(error, backtrace()),
                    DatabaseErrorKind::ReadOnlyTransaction => DbError::Server(error, backtrace()),
                    DatabaseErrorKind::NotNullViolation => DbError::BadRequest(error, backtrace()),
                    DatabaseErrorKind::CheckViolation => DbError::BadRequest(error, backtrace()),
                    DatabaseErrorKind::ClosedConnection => DbError::Network(error, backtrace()),
                    // assume any other database errors are the result
                    // of a raised exception implying a bad request
                    _ => DbError::BadRequest(error, backtrace()),
                },
                diesel::result::Error::NotFound => DbError::BadRequest(error, backtrace()),
                diesel::result::Error::QueryBuilderError(_) => DbError::Server(error, backtrace()),
                diesel::result::Error::DeserializationError(_) => DbError::Server(error, backtrace()),
                diesel::result::Error::SerializationError(_) => DbError::Server(error, backtrace()),
                diesel::result::Error::AlreadyInTransaction => DbError::Server(error, backtrace()),
                diesel::result::Error::NotInTransaction => DbError::Server(error, backtrace()),
                _ => DbError::Other(error, backtrace()),
            }
        }
    }

    impl From<DbError> for Option<diesel::result::Error> {
        fn from(db_error: DbError) -> Self {
            match db_error {
                DbError::Application(_, _) => None,
                DbError::BadRequest(error, _) => Some(error),
                DbError::CustomBadRequest(_, _) => None,
                DbError::InvalidDbState(_, _) => None,
                DbError::Network(error, _) => Some(error),
                DbError::Other(error, _) => Some(error),
                DbError::Server(error, _) => Some(error),
                DbError::Stale(_) => None,
                DbError::TxCleanup(_) => None,
            }
        }
    }

    impl<E: Display> From<DbEntityError<E>> for Error {
        fn from(value: DbEntityError<E>) -> Self {
            Self::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{value}"))
        }
    }

    impl From<DbError> for Error {
        #[framed]
        fn from(err: DbError) -> Self {
            let (err, backtrace) = match err {
                DbError::BadRequest(err, backtrace) => {
                    return Error::init_with_backtrace(StatusCode::BAD_REQUEST, None, format!("{err}"), backtrace)
                }
                DbError::CustomBadRequest(err, backtrace) => {
                    return Error::init_with_backtrace(StatusCode::BAD_REQUEST, None, format!("{err}"), backtrace)
                }
                DbError::Stale(backtrace) => {
                    return Error::init_with_backtrace(StatusCode::SERVICE_UNAVAILABLE, None, None, backtrace)
                }

                DbError::Application(err, backtrace) => (format!("{err}"), backtrace),
                DbError::InvalidDbState(err, backtrace) => (format!("{err}"), backtrace),
                DbError::Network(err, backtrace) => (format!("{err}"), backtrace),
                DbError::Other(err, backtrace) => (format!("{err}"), backtrace),
                DbError::Server(err, backtrace) => (format!("{err}"), backtrace),
                DbError::TxCleanup(err) => return Self::from(err),
            };
            Self::init_with_backtrace(StatusCode::INTERNAL_SERVER_ERROR, None, err, backtrace)
        }
    }

    impl From<diesel::result::Error> for Error {
        #[framed]
        fn from(err: diesel::result::Error) -> Self {
            Error::init(StatusCode::INTERNAL_SERVER_ERROR, None, format!("{err}"))
        }
    }

    impl From<TxCleanupError> for Error {
        #[framed]
        fn from(err: TxCleanupError) -> Self {
            Self::init_with_backtrace(
                StatusCode::INTERNAL_SERVER_ERROR,
                None,
                format!("{}", err.source),
                err.backtrace,
            )
        }
    }

    impl From<Error> for TxCleanupError {
        #[framed]
        fn from(err: Error) -> Self {
            Self {
                source: anyhow::Error::msg(err),
                backtrace: backtrace(),
            }
        }
    }
}
