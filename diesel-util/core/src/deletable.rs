use crate::*;
use diesel::associations::HasTable;
use diesel::dsl::SqlTypeOf;
use diesel::expression::{AsExpression, Expression};
use diesel::expression_methods::ExpressionMethods;
use diesel::helper_types as ht;
use diesel::query_dsl::methods::{FilterDsl, FindDsl};
use diesel::query_source::QuerySource;
use diesel::result::Error;
use diesel::sql_types::{Nullable, SqlType, Timestamp};
use diesel::{query_builder::*, Identifiable};
use diesel::{Column, Table};
use diesel_async::methods::*;
use diesel_async::{AsyncConnection, RunQueryDsl};
use either::Either;
use futures::future::{ready, BoxFuture, FutureExt};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub trait Deletable<'query, C, Tab, I, F, DeletedAt, DeletePatch>: Sized {
    fn hard_delete<'life0, 'async_trait>(
        conn: &'life0 mut C,
        ids: I,
    ) -> BoxFuture<'async_trait, Result<Vec<Self>, Error>>
    where
        'life0: 'async_trait,
        'query: 'async_trait,
        Self: 'async_trait;

    fn maybe_soft_delete<'life0, 'async_trait>(
        conn: &'life0 mut C,
        ids: I,
    ) -> BoxFuture<'async_trait, Either<I, Result<Vec<Self>, Error>>>
    where
        'life0: 'async_trait,
        'query: 'async_trait,
        Self: 'async_trait;
}

pub trait SoftDeletable {
    type DeletedAt: Default + Column<SqlType = Nullable<Timestamp>> + ExpressionMethods;
}

impl<'query, C, Tab, I, F, DeletedAt, DeletePatch, T> Deletable<'query, C, Tab, I, F, DeletedAt, DeletePatch> for T
where
    C: AsyncConnection + 'static,

    // Id bounds
    I: IntoIterator + Send + Sized + 'query,
    I::Item: Clone + Debug + Hash + Eq + Send + Sync + AsExpression<SqlTypeOf<Tab::PrimaryKey>>,
    for<'a> &'a Self: Identifiable<Id = &'a <I as IntoIterator>::Item>,

    Tab: Table + QueryId + Send,
    Tab::PrimaryKey: Expression + ExpressionMethods,
    <Tab::PrimaryKey as Expression>::SqlType: SqlType,

    Self: Send + HasTable<Table = Tab>,
    Tab: FilterDsl<ht::EqAny<Tab::PrimaryKey, Vec<I::Item>>, Output = F>,
    F: IntoUpdateTarget + Send + 'query,

    DeleteStatement<F::Table, F::WhereClause>:
        LoadQuery<'query, C, Self> + QueryFragment<C::Backend> + QueryId + Send + 'query,

    // Audit bounds
    Self: MaybeAudit<'query, C>,
{
    default fn hard_delete<'life0, 'async_trait>(
        conn: &'life0 mut C,
        ids: I,
    ) -> BoxFuture<'async_trait, Result<Vec<Self>, Error>>
    where
        'query: 'async_trait,
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        async move {
            let query = Self::table().filter(Self::table().primary_key().eq_any(ids));
            let records = diesel::delete(query).get_results(conn).await?;

            Self::maybe_insert_audit_records(conn, &records).await?;

            Ok(records)
        }
        .boxed()
    }

    #[allow(unused_variables)]
    default fn maybe_soft_delete<'life0, 'async_trait>(
        conn: &'life0 mut C,
        ids: I,
    ) -> BoxFuture<'async_trait, Either<I, Result<Vec<Self>, Error>>>
    where
        'query: 'async_trait,
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(ready(Either::Left(ids)))
    }
}

impl<'query, C, Tab, I, F, DeletedAt, DeletePatch, T> Deletable<'query, C, Tab, I, F, DeletedAt, DeletePatch> for T
where
    T: SoftDeletable,

    C: AsyncConnection + 'static,

    // Id bounds
    I: IntoIterator + Send + Sized + 'query,
    I::Item: Clone + Debug + Hash + Eq + Send + Sync + AsExpression<SqlTypeOf<Tab::PrimaryKey>>,
    for<'a> &'a Self: Identifiable<Id = &'a <I as IntoIterator>::Item>,

    Tab: Table + QueryId + Send + 'query,
    Tab::PrimaryKey: Expression + ExpressionMethods,
    <Tab::PrimaryKey as Expression>::SqlType: SqlType,

    Self: Send + HasTable<Table = Tab>,
    Tab: FilterDsl<ht::EqAny<Tab::PrimaryKey, Vec<I::Item>>, Output = F>,
    F: IntoUpdateTarget + Send + 'query,

    DeleteStatement<F::Table, F::WhereClause>:
        LoadQuery<'query, C, Self> + QueryFragment<C::Backend> + QueryId + Send + 'query,

    // Audit bounds
    Self: MaybeAudit<'query, C>,

    I::Item: Into<DeletePatch>,
    DeletePatch: AsChangeset<Target = Tab> + Debug + HasTable<Table = Tab> + IncludesChanges + Send + Sync + 'query,
    DeletePatch::Changeset: Send,
    for<'a> &'a DeletePatch: HasTable<Table = Tab> + Identifiable<Id = &'a I::Item> + IntoUpdateTarget,
    for<'a> <&'a DeletePatch as IntoUpdateTarget>::WhereClause: Send,

    <Tab as QuerySource>::FromClause: Send,

    // UpdateStatement bounds
    Tab: FindDsl<I::Item>,
    ht::Find<Tab, I::Item>: HasTable<Table = Tab> + IntoUpdateTarget + Send,
    <ht::Find<Tab, I::Item> as IntoUpdateTarget>::WhereClause: Send,
    ht::Update<ht::Find<Tab, I::Item>, DeletePatch>: AsQuery + LoadQuery<'query, C, Self> + Send,

    // Filter bounds for records whose changesets do not include any changes
    F: IsNotDeleted<'query, C, Self, Self>,
{
    default fn maybe_soft_delete<'life0, 'async_trait>(
        conn: &'life0 mut C,
        ids: I,
    ) -> BoxFuture<'async_trait, Either<I, Result<Vec<Self>, Error>>>
    where
        'query: 'async_trait,
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let patches = ids.into_iter().map(Into::into).collect::<Vec<DeletePatch>>();
        let ids = patches.iter().map(|patch| patch.id().clone()).collect::<Vec<_>>();

        Box::pin(async move {
            let result = (move || async move {
                let no_change_patch_ids = patches
                    .iter()
                    .filter_map(
                        |patch| {
                            if !patch.includes_changes() {
                                Some(patch.id().to_owned())
                            } else {
                                None
                            }
                        },
                    )
                    .collect::<Vec<_>>();

                let num_changed_patches = ids.len() - no_change_patch_ids.len();
                if num_changed_patches == 0 {
                    return Ok(vec![]);
                }
                let mut all_updated = Vec::with_capacity(num_changed_patches);
                for patch in patches.into_iter().filter(|patch| patch.includes_changes()) {
                    let record = diesel::update(Self::table().find(patch.id().to_owned()))
                        .set(patch)
                        .get_result::<Self>(conn)
                        .await?;
                    all_updated.push(record);
                }

                Self::maybe_insert_audit_records(conn, &all_updated).await?;

                let filter = FilterDsl::filter(Self::table(), Self::table().primary_key().eq_any(no_change_patch_ids))
                    .is_not_deleted();
                let unchanged_records = filter.get_results::<Self>(&mut *conn).await?;

                let mut all_records = unchanged_records
                    .into_iter()
                    .chain(all_updated.into_iter())
                    .map(|record| (record.id().clone(), record))
                    .collect::<HashMap<_, _>>();

                Ok(ids.iter().map(|id| all_records.remove(id).unwrap()).collect::<Vec<_>>())
            })()
            .await;
            Either::Right(result)
        })
    }
}
