use crate::db::schema::*;
use authzen::storage_backends::diesel::prelude::*;
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use uuid::Uuid;

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
}

impl DbInsert for DbCart {
    type Post<'a> = Self;
}
