use crate::db::schema::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_util::*;
use uuid::Uuid;

#[derive(Clone, Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = cart)]
pub struct DbCart {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub account_id: Uuid,
}
