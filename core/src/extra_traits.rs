use crate::ActionError;

impl<DecisionMakerError, StorageError, TransactionCacheError>
    From<ActionError<DecisionMakerError, StorageError, TransactionCacheError>> for authzen_service_util::Error
where
    DecisionMakerError: std::fmt::Display,
    StorageError: Into<authzen_service_util::Error>,
    TransactionCacheError: Into<authzen_service_util::Error>,
{
    fn from(value: ActionError<DecisionMakerError, StorageError, TransactionCacheError>) -> Self {
        match value {
            ActionError::Authz(err) => Self::bad_request_details(err),
            ActionError::Storage(err) => err.into(),
            ActionError::TransactionCache(err) => err.into(),
        }
    }
}
