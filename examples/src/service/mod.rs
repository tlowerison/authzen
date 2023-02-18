pub mod context;

pub use context::*;

use crate::*;
use authzen::*;
use diesel_util::*;
use service_util::Error;
use uuid::Uuid;

pub async fn add_item_to_cart<D: Db>(ctx: Ctx<'_, D>, cart_id: Uuid, item_id: Uuid) -> Result<DbCartItem, Error> {
    let db_cart_item = DbCartItem::builder().cart_id(cart_id).item_id(item_id).build();
    CartItem::try_create(&ctx, [db_cart_item]).await?;
    todo!()
}

// try_create should typically handle all your needs for handling
// both authorization queries and performing the actual action
// that is being authorized, however, in the case where you have
// some context which implements `CanContext<DecisionMaker>` for
// for multiple `DecisionMaker`s or implements `TryContext<DecisionMaker, StorageClient>`
// for multiple `StorageClient`s, you'll be able to explicitly specify those parameters
// as function generics
pub async fn add_item_to_cart_explicitly<D: Db>(
    ctx: Ctx<'_, D>,
    cart_id: Uuid,
    item_id: Uuid,
) -> Result<DbCartItem, Error> {
    let db_cart_item = DbCartItem::builder().cart_id(cart_id).item_id(item_id).build();
    CartItem::can_create::<&opa_util::OPAClient>(&ctx, &[&db_cart_item]).await?;
    CartItem::try_create::<&opa_util::OPAClient, D>(&ctx, [db_cart_item]).await?;
    todo!()
}

pub async fn do_things<D: Db>(ctx: Ctx<'_, D>, cart_id: Uuid, item_id: Uuid) -> Result<DbCartItem, Error> {
    let db_cart_item = DbCartItem::builder().cart_id(cart_id).item_id(item_id).build();

    CartItem::can_do_a_thing::<&opa_util::OPAClient>(&ctx, &[&db_cart_item]).await?;
    // CartItem::try_create_then_delete(&ctx, [db_cart_item]).await?;
    DbCartItem::delete(&ctx, [Uuid::default()]).await?;
    todo!()
}

mod foo {
    use diesel::associations::HasTable;
    use diesel::dsl::SqlTypeOf;
    use diesel::expression::{AsExpression, Expression};
    use diesel::expression_methods::ExpressionMethods;
    use diesel::helper_types as ht;
    use diesel::query_dsl::methods::FilterDsl;
    use diesel::sql_types::SqlType;
    use diesel::Table;
    use diesel::{query_builder::*, Identifiable};
    use diesel_async::methods::*;
    use diesel_async::AsyncConnection;
    use diesel_util::*;
    use std::fmt::Debug;
    use std::hash::Hash;

    fn foo<'query, C, Tab, I, F, DeletedAt, DeletePatch, T>()
    where
        T: Deletable<'query, C, Tab, I, F, DeletedAt, DeletePatch>,
        C: AsyncConnection + 'static,

        // Id bounds
        I: IntoIterator + Send + Sized + 'query,
        I::Item: Clone + Debug + Hash + Eq + Send + Sync + AsExpression<SqlTypeOf<Tab::PrimaryKey>>,
        for<'a> &'a T: Identifiable<Id = &'a <I as IntoIterator>::Item>,

        Tab: Table + QueryId + Send,
        Tab::PrimaryKey: Expression + ExpressionMethods,
        <Tab::PrimaryKey as Expression>::SqlType: SqlType,

        T: Send + HasTable<Table = Tab>,
        Tab: FilterDsl<ht::EqAny<Tab::PrimaryKey, Vec<I::Item>>, Output = F>,
        F: IntoUpdateTarget + Send + 'query,

        DeleteStatement<F::Table, F::WhereClause>:
            LoadQuery<'query, C, T> + QueryFragment<C::Backend> + QueryId + Send + 'query,

        // Audit bounds
        T: MaybeAudit<'query, C>,
    {
    }

    fn bar<D: crate::Db>() {
        foo::<'_, D::AsyncConnection, crate::db::schema::cart_item::table, [uuid::Uuid; 1], _, (), (), crate::DbCartItem>(
        )
    }
}
