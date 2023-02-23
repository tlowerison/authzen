use crate::ActionError;

impl<DecisionMakerError, StorageError, TransactionCacheError>
    From<ActionError<DecisionMakerError, StorageError, TransactionCacheError>> for service_util::Error
where
    DecisionMakerError: std::fmt::Display,
    StorageError: Into<service_util::Error>,
    TransactionCacheError: Into<service_util::Error>,
{
    fn from(value: ActionError<DecisionMakerError, StorageError, TransactionCacheError>) -> Self {
        match value {
            ActionError::Authz(err) => Self::bad_request_details(err),
            ActionError::Storage(err) => err.into(),
            ActionError::TransactionCache(err) => err.into(),
        }
    }
}
