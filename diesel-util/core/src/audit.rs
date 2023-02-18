use diesel::query_builder::{InsertStatement, QueryId, UndecoratedInsertRecord};
use diesel::query_source::QuerySource;
use diesel::result::Error;
use diesel::{associations::HasTable, Insertable, Table};
use diesel_async::{methods::ExecuteDsl, AsyncConnection, RunQueryDsl};
use futures::future::{ready, BoxFuture};

pub trait Audit: Into<Self::Raw> {
    type Raw;
    type Table: HasTable;
}

pub trait MaybeAudit<'query, C>: Sized + Sync {
    fn maybe_insert_audit_records<'life0, 'life1, 'async_trait>(
        conn: &'life0 mut C,
        records: &'life1 [Self],
    ) -> BoxFuture<'async_trait, Result<(), Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait;
}

impl<'query, C, T: Sync> MaybeAudit<'query, C> for T {
    default fn maybe_insert_audit_records<'life0, 'life1, 'async_trait>(
        _: &'life0 mut C,
        _: &'life1 [Self],
    ) -> BoxFuture<'async_trait, Result<(), Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        Box::pin(ready(Ok(())))
    }
}

#[async_trait]
impl<'query, C, T: Audit + Clone + Send + Sync> MaybeAudit<'query, C> for T
where
    C: AsyncConnection + 'static,

    Self: Audit + Clone + Send,
    <Self as Audit>::Raw: Send,
    Vec<<Self as Audit>::Raw>: Insertable<<<Self as Audit>::Table as HasTable>::Table>
        + UndecoratedInsertRecord<<<Self as Audit>::Table as HasTable>::Table>,
    <<Self as Audit>::Table as HasTable>::Table: Table + QueryId + Send,
    <<<Self as Audit>::Table as HasTable>::Table as QuerySource>::FromClause: Send,

    InsertStatement<
        <<Self as Audit>::Table as HasTable>::Table,
        <Vec<<Self as Audit>::Raw> as Insertable<<<Self as Audit>::Table as HasTable>::Table>>::Values,
    >: ExecuteDsl<C>,

    <Vec<<Self as Audit>::Raw> as Insertable<<<Self as Audit>::Table as HasTable>::Table>>::Values: Send,
{
    async fn maybe_insert_audit_records(conn: &mut C, records: &[Self]) -> Result<(), diesel::result::Error>
where {
        let raw_records = records.iter().map(Clone::clone).map(Into::into).collect::<Vec<_>>();
        diesel::insert_into(<Self as Audit>::Table::table())
            .values(raw_records)
            .execute(conn)
            .await?;
        Ok(())
    }
}
