use crate::db::schema::*;
use crate::db::Db;
use ::chrono::{NaiveDateTime, Utc};
use ::diesel::result::Error;
use ::diesel::{ExpressionMethods, OptionalExtension, QueryDsl};
use ::diesel_async::RunQueryDsl;
use ::scoped_futures::ScopedFutureExt;
use ::uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Identifiable, Insertable, Queryable, Serialize, TypedBuilder)]
#[diesel(table_name = cart)]
pub struct DbCart {
    #[builder(default = Uuid::new_v4())]
    pub id: Uuid,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub created_at: NaiveDateTime,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub updated_at: NaiveDateTime,
    pub account_id: Uuid,
    #[builder(default)]
    pub used_at: Option<NaiveDateTime>,
}

impl DbCart {
    pub async fn get_unused<D: Db>(db: &D, account_id: Uuid) -> Result<Option<DbCart>, Error> {
        db.query(|conn| {
            async move {
                cart::table
                    .filter(cart::account_id.eq(account_id))
                    .filter(cart::used_at.is_null())
                    .get_result(conn)
                    .await
                    .optional()
            }
            .scope_boxed()
        })
        .await
    }
}
