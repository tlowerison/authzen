use crate::db::schema::*;
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel_util::*;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Identifiable, Insertable, Queryable, Serialize, TypedBuilder)]
#[diesel(table_name = cart_item)]
pub struct DbCartItem {
    #[builder(default = Uuid::new_v4())]
    pub id: Uuid,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub created_at: NaiveDateTime,
    pub cart_id: Uuid,
    pub item_id: Uuid,
}

impl DbInsert for DbCartItem {
    type Post<'a> = Self;
}
