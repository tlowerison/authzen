use crate::Error;

impl<DecisionMakerError, StorageError> From<Error<DecisionMakerError, StorageError>> for service_util::Error
where
    DecisionMakerError: std::fmt::Display,
    StorageError: Into<service_util::Error>,
{
    fn from(value: Error<DecisionMakerError, StorageError>) -> Self {
        match value {
            Error::Authz(err) => Self::bad_request_details(err),
            Error::Storage(err) => err.into(),
        }
    }
}
