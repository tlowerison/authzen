use crate::db::schema::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_util::*;
use uuid::Uuid;

#[derive(Clone, Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = cart_item)]
pub struct DbCart {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub cart_id: Uuid,
    pub item_id: Uuid,
}
