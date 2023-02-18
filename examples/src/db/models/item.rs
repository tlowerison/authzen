use crate::db::schema::*;
use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use diesel_util::*;
use uuid::Uuid;

#[derive(Audit, Clone, Debug, Deserialize, Identifiable, Insertable, Queryable, Serialize)]
#[audit(foreign_key = item_id_arbitrary_foreign_key_name)]
#[diesel(table_name = item)]
#[derive(TypedBuilder)]
pub struct DbItem {
    #[builder(default = Uuid::new_v4())]
    pub id: Uuid,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub created_at: NaiveDateTime,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub updated_at: NaiveDateTime,
    pub name: String,
    #[builder(default, setter(into))]
    pub description: Option<String>,
}

#[derive(AsChangeset, Clone, Debug, Deserialize, Identifiable, IncludesChanges, Serialize, TypedBuilder)]
#[diesel(table_name = item)]
pub struct DbItemPatch {
    pub id: Uuid,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub updated_at: NaiveDateTime,
    #[builder(setter(into))]
    pub name: Option<String>,
    #[builder(setter(into))]
    pub description: Option<Option<String>>,
}

impl DbInsert for DbItem {
    type Post<'a> = Self;
}

impl DbUpdate for DbItem {
    type Patch<'a> = DbItemPatch;
}
