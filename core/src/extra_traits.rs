use crate::ActionError;

impl<DecisionMakerError, StorageError> From<ActionError<DecisionMakerError, StorageError>> for service_util::Error
where
    DecisionMakerError: std::fmt::Display,
    StorageError: Into<service_util::Error>,
{
    fn from(value: ActionError<DecisionMakerError, StorageError>) -> Self {
        match value {
            ActionError::Authz(err) => Self::bad_request_details(err),
            ActionError::Storage(err) => err.into(),
        }
    }
}
