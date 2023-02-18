use crate::SoftDeletable;
use diesel::dsl::{IsNotNull, IsNull};
use diesel::{
    backend::Backend, expression::ValidGrouping, expression_methods::ExpressionMethods, helper_types::*,
    query_builder::*, query_dsl::methods::*, AppearsOnTable, Expression, QueryDsl, QueryResult, QuerySource,
    SelectableExpression,
};
use diesel_async::{methods::LoadQuery, AsyncConnection};
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait IsDeleted<'query, C, S, T>:
    LoadQuery<'query, C, T> + Send + QueryFragment<<C as AsyncConnection>::Backend> + QueryId + Sized + 'query
where
    C: AsyncConnection,
{
    type IsDeletedFilter: LoadQuery<'query, C, T>
        + Send
        + QueryId
        + QueryFragment<<C as AsyncConnection>::Backend>
        + 'query
        + From<Self> = Self;

    #[allow(clippy::wrong_self_convention)]
    fn is_deleted(self) -> Self::IsDeletedFilter {
        self.into()
    }
}

pub trait IsNotDeleted<'query, C, S, T>:
    LoadQuery<'query, C, T> + QueryFragment<<C as AsyncConnection>::Backend> + QueryId + Send + Sized + 'query
where
    C: AsyncConnection,
{
    type IsNotDeletedFilter: LoadQuery<'query, C, T>
        + Send
        + QueryId
        + QueryFragment<<C as AsyncConnection>::Backend>
        + 'query
        + From<Self> = Self;

    #[allow(clippy::wrong_self_convention)]
    fn is_not_deleted(self) -> Self::IsNotDeletedFilter {
        self.into()
    }
}

impl<'query, Q, C, S, T> IsDeleted<'query, C, S, T> for Q
where
    Q: LoadQuery<'query, C, T> + QueryFragment<<C as AsyncConnection>::Backend> + QueryId + Send + 'query,
    C: AsyncConnection,
{
    default type IsDeletedFilter = Self;
    default fn is_deleted(self) -> Self::IsDeletedFilter {
        self.into()
    }
}

impl<'query, Q, C, S, T> IsNotDeleted<'query, C, S, T> for Q
where
    Q: LoadQuery<'query, C, T> + Send + QueryFragment<<C as AsyncConnection>::Backend> + QueryId + 'query,
    C: AsyncConnection,
{
    default type IsNotDeletedFilter = Self;
    default fn is_not_deleted(self) -> Self::IsNotDeletedFilter {
        self.into()
    }
}

impl<'query, Q, C, S, T> IsDeleted<'query, C, S, T> for Q
where
    Q: LoadQuery<'query, C, T>
        + Send
        + QueryId
        + QueryFragment<<C as AsyncConnection>::Backend>
        + 'query
        + FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>>,
    C: AsyncConnection,
    S: SoftDeletable,
    IsDeletedFilter<'query, Q, S>:
        LoadQuery<'query, C, T> + Send + QueryFragment<<C as AsyncConnection>::Backend> + QueryId + 'query,
{
    type IsDeletedFilter = IsDeletedFilter<'query, Q, S>;
}

impl<'query, Q, C, S, T> IsNotDeleted<'query, C, S, T> for Q
where
    Q: LoadQuery<'query, C, T>
        + Send
        + QueryId
        + QueryFragment<<C as AsyncConnection>::Backend>
        + 'query
        + FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>>,
    C: AsyncConnection,
    S: SoftDeletable,
    IsNotDeletedFilter<'query, Q, S>:
        LoadQuery<'query, C, T> + Send + QueryFragment<<C as AsyncConnection>::Backend> + QueryId + 'query,
{
    type IsNotDeletedFilter = IsNotDeletedFilter<'query, Q, S>;
}

pub struct IsDeletedFilter<'query, Q, S>(
    Filter<Q, IsNotNull<<S as SoftDeletable>::DeletedAt>>,
    PhantomData<S>,
    PhantomData<&'query ()>,
)
where
    S: SoftDeletable,
    Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>>,
    Filter<Q, IsNotNull<<S as SoftDeletable>::DeletedAt>>: 'query,
    Self: 'query;

pub struct IsNotDeletedFilter<'query, Q, S>(
    Filter<Q, IsNull<<S as SoftDeletable>::DeletedAt>>,
    PhantomData<S>,
    PhantomData<&'query ()>,
)
where
    S: SoftDeletable,
    Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>>,
    Filter<Q, IsNull<<S as SoftDeletable>::DeletedAt>>: 'query,
    Self: 'query;

impl<Q, S> From<Q> for IsDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>>,
{
    fn from(q: Q) -> Self {
        Self(
            q.filter(<<S as SoftDeletable>::DeletedAt as Default>::default().is_not_null()),
            PhantomData,
            PhantomData,
        )
    }
}

impl<Q, S> From<Q> for IsNotDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>>,
{
    fn from(q: Q) -> Self {
        Self(
            q.filter(<<S as SoftDeletable>::DeletedAt as Default>::default().is_null()),
            PhantomData,
            PhantomData,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Wrapper<T>(pub T);

impl<T> From<T> for Wrapper<T> {
    fn from(t: T) -> Self {
        Self(t)
    }
}

impl<S, Q, F, QS> SelectableExpression<IsDeletedFilter<'_, Q, S>> for Wrapper<QS>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    QS: SelectableExpression<F>,
{
}
impl<S, Q, F, QS> SelectableExpression<IsNotDeletedFilter<'_, Q, S>> for Wrapper<QS>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    QS: SelectableExpression<F>,
{
}

impl<S, Q, F, QS> AppearsOnTable<IsDeletedFilter<'_, Q, S>> for Wrapper<QS>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    QS: AppearsOnTable<F>,
{
}
impl<S, Q, F, QS> AppearsOnTable<IsNotDeletedFilter<'_, Q, S>> for Wrapper<QS>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    QS: AppearsOnTable<F>,
{
}

impl<T: Expression> Expression for Wrapper<T> {
    type SqlType = T::SqlType;
}

impl<GroupByClause, T: ValidGrouping<GroupByClause>> ValidGrouping<GroupByClause> for Wrapper<T> {
    type IsAggregate = T::IsAggregate;
}

impl<S, Q, F> QuerySource for IsDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    F: QuerySource,
{
    type FromClause = F::FromClause;
    type DefaultSelection = Wrapper<<F as QuerySource>::DefaultSelection>;

    fn from_clause(&self) -> Self::FromClause {
        self.0.from_clause()
    }
    fn default_selection(&self) -> Self::DefaultSelection {
        Wrapper(self.0.default_selection())
    }
}
impl<S, Q, F> QuerySource for IsNotDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    F: QuerySource,
{
    type FromClause = F::FromClause;
    type DefaultSelection = Wrapper<<F as QuerySource>::DefaultSelection>;

    fn from_clause(&self) -> Self::FromClause {
        self.0.from_clause()
    }
    fn default_selection(&self) -> Self::DefaultSelection {
        Wrapper(self.0.default_selection())
    }
}

impl<S, Q, F> Query for IsDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    F: Query,
{
    type SqlType = <F as Query>::SqlType;
}
impl<S, Q, F> Query for IsNotDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    F: Query,
{
    type SqlType = <F as Query>::SqlType;
}

impl<S, Q, F, DB: Backend, ST> QueryFragment<DB, ST> for IsDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    F: QueryFragment<DB, ST>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass.reborrow())
    }
}
impl<S, Q, F, DB: Backend, ST> QueryFragment<DB, ST> for IsNotDeletedFilter<'_, Q, S>
where
    S: SoftDeletable,
    Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    F: QueryFragment<DB, ST>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        self.0.walk_ast(pass.reborrow())
    }
}

impl<F: QueryId, S: SoftDeletable, Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>> QueryId
    for IsDeletedFilter<'_, Q, S>
{
    type QueryId = <F as QueryId>::QueryId;
    const HAS_STATIC_QUERY_ID: bool = <F as QueryId>::HAS_STATIC_QUERY_ID;
    fn query_id() -> Option<std::any::TypeId> {
        <F as QueryId>::query_id()
    }
}
impl<F: QueryId, S: SoftDeletable, Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>> QueryId
    for IsNotDeletedFilter<'_, Q, S>
{
    type QueryId = <F as QueryId>::QueryId;
    const HAS_STATIC_QUERY_ID: bool = <F as QueryId>::HAS_STATIC_QUERY_ID;
    fn query_id() -> Option<std::any::TypeId> {
        <F as QueryId>::query_id()
    }
}

impl<
        'query,
        DB,
        F: BoxedDsl<'query, DB>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > BoxedDsl<'query, DB> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as BoxedDsl<'query, DB>>::Output;
    fn internal_into_boxed(self) -> IntoBoxed<'query, Self, DB> {
        self.0.internal_into_boxed()
    }
}
impl<
        'query,
        DB,
        F: BoxedDsl<'query, DB>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > BoxedDsl<'query, DB> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as BoxedDsl<'query, DB>>::Output;
    fn internal_into_boxed(self) -> IntoBoxed<'query, Self, DB> {
        self.0.internal_into_boxed()
    }
}

impl<F: DistinctDsl, S: SoftDeletable, Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>> DistinctDsl
    for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as DistinctDsl>::Output;
    fn distinct(self) -> Distinct<Self> {
        self.0.distinct()
    }
}
impl<F: DistinctDsl, S: SoftDeletable, Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>> DistinctDsl
    for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as DistinctDsl>::Output;
    fn distinct(self) -> Distinct<Self> {
        self.0.distinct()
    }
}

#[cfg(feature = "postgres")]
impl<
        Selection,
        F: DistinctOnDsl<Selection>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > DistinctOnDsl<Selection> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as DistinctOnDsl<Selection>>::Output;
    fn distinct_on(self, selection: Selection) -> DistinctOn<Self, Selection> {
        self.0.distinct_on(selection)
    }
}
#[cfg(feature = "postgres")]
impl<
        Selection,
        F: DistinctOnDsl<Selection>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > DistinctOnDsl<Selection> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as DistinctOnDsl<Selection>>::Output;
    fn distinct_on(self, selection: Selection) -> DistinctOn<Self, Selection> {
        self.0.distinct_on(selection)
    }
}

impl<
        Predicate,
        F: FilterDsl<Predicate>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > FilterDsl<Predicate> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as FilterDsl<Predicate>>::Output;
    fn filter(self, predicate: Predicate) -> Self::Output {
        self.0.filter(predicate)
    }
}
impl<
        Predicate,
        F: FilterDsl<Predicate>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > FilterDsl<Predicate> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as FilterDsl<Predicate>>::Output;
    fn filter(self, predicate: Predicate) -> Self::Output {
        self.0.filter(predicate)
    }
}

impl<PK, F: FindDsl<PK>, S: SoftDeletable, Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>>
    FindDsl<PK> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as FindDsl<PK>>::Output;
    fn find(self, id: PK) -> Self::Output {
        self.0.find(id)
    }
}
impl<PK, F: FindDsl<PK>, S: SoftDeletable, Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>>
    FindDsl<PK> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as FindDsl<PK>>::Output;
    fn find(self, id: PK) -> Self::Output {
        self.0.find(id)
    }
}

impl<
        Expr: Expression,
        F: GroupByDsl<Expr>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > GroupByDsl<Expr> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as GroupByDsl<Expr>>::Output;
    fn group_by(self, expr: Expr) -> GroupBy<Self, Expr> {
        self.0.group_by(expr)
    }
}
impl<
        Expr: Expression,
        F: GroupByDsl<Expr>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > GroupByDsl<Expr> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as GroupByDsl<Expr>>::Output;
    fn group_by(self, expr: Expr) -> GroupBy<Self, Expr> {
        self.0.group_by(expr)
    }
}

impl<
        Predicate,
        F: HavingDsl<Predicate>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > HavingDsl<Predicate> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as HavingDsl<Predicate>>::Output;
    fn having(self, predicate: Predicate) -> Having<Self, Predicate> {
        self.0.having(predicate)
    }
}
impl<
        Predicate,
        F: HavingDsl<Predicate>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > HavingDsl<Predicate> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as HavingDsl<Predicate>>::Output;
    fn having(self, predicate: Predicate) -> Having<Self, Predicate> {
        self.0.having(predicate)
    }
}

impl<F: LimitDsl, S: SoftDeletable, Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>> LimitDsl
    for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as LimitDsl>::Output;
    fn limit(self, limit: i64) -> Self::Output {
        self.0.limit(limit)
    }
}
impl<F: LimitDsl, S: SoftDeletable, Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>> LimitDsl
    for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as LimitDsl>::Output;
    fn limit(self, limit: i64) -> Self::Output {
        self.0.limit(limit)
    }
}

impl<
        Conn,
        F: diesel::RunQueryDsl<Conn>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > diesel::RunQueryDsl<Conn> for IsDeletedFilter<'_, Q, S>
{
}
impl<
        Conn,
        F: diesel::RunQueryDsl<Conn>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > diesel::RunQueryDsl<Conn> for IsNotDeletedFilter<'_, Q, S>
{
}
impl<
        Conn,
        F: diesel_async::RunQueryDsl<Conn>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > diesel_async::RunQueryDsl<Conn> for IsDeletedFilter<'_, Q, S>
{
}
impl<
        Conn,
        F: diesel_async::RunQueryDsl<Conn>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > diesel_async::RunQueryDsl<Conn> for IsNotDeletedFilter<'_, Q, S>
{
}

impl<
        Lock,
        F: LockingDsl<Lock>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > LockingDsl<Lock> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as LockingDsl<Lock>>::Output;
    fn with_lock(self, lock: Lock) -> Self::Output {
        self.0.with_lock(lock)
    }
}
impl<
        Lock,
        F: LockingDsl<Lock>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > LockingDsl<Lock> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as LockingDsl<Lock>>::Output;
    fn with_lock(self, lock: Lock) -> Self::Output {
        self.0.with_lock(lock)
    }
}

impl<
        Modifier,
        F: ModifyLockDsl<Modifier>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > ModifyLockDsl<Modifier> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as ModifyLockDsl<Modifier>>::Output;
    fn modify_lock(self, modifier: Modifier) -> Self::Output {
        self.0.modify_lock(modifier)
    }
}
impl<
        Modifier,
        F: ModifyLockDsl<Modifier>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > ModifyLockDsl<Modifier> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as ModifyLockDsl<Modifier>>::Output;
    fn modify_lock(self, modifier: Modifier) -> Self::Output {
        self.0.modify_lock(modifier)
    }
}

impl<F: OffsetDsl, S: SoftDeletable, Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>> OffsetDsl
    for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as OffsetDsl>::Output;
    fn offset(self, offset: i64) -> Self::Output {
        self.0.offset(offset)
    }
}
impl<F: OffsetDsl, S: SoftDeletable, Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>> OffsetDsl
    for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as OffsetDsl>::Output;
    fn offset(self, offset: i64) -> Self::Output {
        self.0.offset(offset)
    }
}

impl<
        Predicate,
        F: OrFilterDsl<Predicate>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > OrFilterDsl<Predicate> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as OrFilterDsl<Predicate>>::Output;
    fn or_filter(self, predicate: Predicate) -> Self::Output {
        self.0.or_filter(predicate)
    }
}
impl<
        Predicate,
        F: OrFilterDsl<Predicate>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > OrFilterDsl<Predicate> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as OrFilterDsl<Predicate>>::Output;
    fn or_filter(self, predicate: Predicate) -> Self::Output {
        self.0.or_filter(predicate)
    }
}

impl<
        Expr: Expression,
        F: OrderDsl<Expr>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > OrderDsl<Expr> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as OrderDsl<Expr>>::Output;
    fn order(self, expr: Expr) -> Self::Output {
        self.0.order(expr)
    }
}
impl<
        Expr: Expression,
        F: OrderDsl<Expr>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > OrderDsl<Expr> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as OrderDsl<Expr>>::Output;
    fn order(self, expr: Expr) -> Self::Output {
        self.0.order(expr)
    }
}

impl<
        Selection: Expression,
        F: SelectDsl<Selection>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > SelectDsl<Selection> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as SelectDsl<Selection>>::Output;
    fn select(self, selection: Selection) -> Self::Output {
        self.0.select(selection)
    }
}
impl<
        Selection: Expression,
        F: SelectDsl<Selection>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > SelectDsl<Selection> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as SelectDsl<Selection>>::Output;
    fn select(self, selection: Selection) -> Self::Output {
        self.0.select(selection)
    }
}

impl<F: SelectNullableDsl, S: SoftDeletable, Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>>
    SelectNullableDsl for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as SelectNullableDsl>::Output;
    fn nullable(self) -> Self::Output {
        self.0.nullable()
    }
}
impl<F: SelectNullableDsl, S: SoftDeletable, Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>>
    SelectNullableDsl for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as SelectNullableDsl>::Output;
    fn nullable(self) -> Self::Output {
        self.0.nullable()
    }
}

impl<
        Expr,
        F: ThenOrderDsl<Expr>,
        S: SoftDeletable,
        Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > ThenOrderDsl<Expr> for IsDeletedFilter<'_, Q, S>
{
    type Output = <F as ThenOrderDsl<Expr>>::Output;
    fn then_order_by(self, expr: Expr) -> Self::Output {
        self.0.then_order_by(expr)
    }
}
impl<
        Expr,
        F: ThenOrderDsl<Expr>,
        S: SoftDeletable,
        Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>,
    > ThenOrderDsl<Expr> for IsNotDeletedFilter<'_, Q, S>
{
    type Output = <F as ThenOrderDsl<Expr>>::Output;
    fn then_order_by(self, expr: Expr) -> Self::Output {
        self.0.then_order_by(expr)
    }
}

impl<F: QueryDsl, S: SoftDeletable, Q: FilterDsl<IsNotNull<<S as SoftDeletable>::DeletedAt>, Output = F>> QueryDsl
    for IsDeletedFilter<'_, Q, S>
{
}
impl<F: QueryDsl, S: SoftDeletable, Q: FilterDsl<IsNull<<S as SoftDeletable>::DeletedAt>, Output = F>> QueryDsl
    for IsNotDeletedFilter<'_, Q, S>
{
}
