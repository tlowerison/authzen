use crate::*;

// TODO: either use specialization or newtypes avoid conflicting implementations
// between B: diesel::Backend and B: sqlx::Database
// impl<B> StorageBackend for B where B: Database {}

impl StorageError for sqlx::Error {
    fn not_found() -> Self {
        Self::RowNotFound
    }
}

// TODO: refactor DbEntity in authzen::storage_backends::diesel
// to be generalized over diesel, sqlx, etc.
// need to do that in order be able to attach a transaction id
// to each one of their transactions
