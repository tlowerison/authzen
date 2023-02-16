use crate::db::schema::*;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel_util::*;
use uuid::Uuid;

#[derive(Audit, Clone, Debug, Identifiable, Insertable, Queryable)]
#[audit(foreign_key = item_id_arbitrary_foreign_key_name)]
#[diesel(table_name = item)]
pub struct DbItem {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub description: Option<String>,
}

#[derive(AsChangeset, Clone, Debug, Identifiable, IncludesChanges)]
#[diesel(table_name = item)]
pub struct DbItemPatch {
    pub id: Uuid,
    pub updated_at: NaiveDateTime,
    pub name: Option<String>,
    pub description: Option<Option<String>>,
}
