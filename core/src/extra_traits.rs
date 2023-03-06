use crate::ActionError;

impl<AuthzEngineError, StorageError, TransactionCacheError>
    From<ActionError<AuthzEngineError, StorageError, TransactionCacheError>> for authzen_service_util::Error
where
    AuthzEngineError: std::fmt::Display,
    StorageError: Into<authzen_service_util::Error>,
    TransactionCacheError: Into<authzen_service_util::Error>,
{
    fn from(value: ActionError<AuthzEngineError, StorageError, TransactionCacheError>) -> Self {
        match value {
            ActionError::Authz(err) => Self::bad_request_details(err),
            ActionError::DataSource(err) => err.into(),
            ActionError::TransactionCache(err) => err.into(),
        }
    }
}
