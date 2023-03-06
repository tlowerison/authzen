use crate::db::schema::*;
use ::authzen::prelude::*;
use ::chrono::{NaiveDateTime, Utc};
use ::diesel::prelude::*;
use ::uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, TypedBuilder)]
pub struct DbAccount {
    #[builder(default = Uuid::new_v4())]
    pub id: Uuid,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub created_at: NaiveDateTime,
    #[builder(default = Utc::now().naive_utc(), setter(skip))]
    pub updated_at: NaiveDateTime,
    #[builder(default, setter(skip))]
    pub deleted_at: Option<NaiveDateTime>,
    #[serde(flatten)]
    pub identifier: Identifier,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Identifier {
    Email(String),
    Username(String),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DbAccountPatch {
    pub id: Uuid,
    pub updated_at: NaiveDateTime,
    pub identifier: Option<Identifier>,
}

impl DbEntity for DbAccount {
    type Table = account::table;
    type Raw = _DbAccount;
    type Id = Uuid;
}

impl DbInsert for DbAccount {
    type PostHelper<'a> = Self;
    type Post<'a> = _DbAccount;
}

impl DbUpdate for DbAccount {
    type PatchHelper<'a> = DbAccountPatch;
    type Patch<'a> = _DbAccountPatch;
}

impl authzen::Identifiable for DbAccount {
    type Id = Uuid;
    fn id(&self) -> &Self::Id {
        &self.id
    }
}

#[derive(Audit, Clone, Debug, Identifiable, Insertable, Queryable, SoftDelete)]
#[audit(foreign_key = account_id)]
#[diesel(table_name = account)]
#[soft_delete(db_entity = DbAccount, deleted_at = deleted_at)]
pub struct _DbAccount {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(AsChangeset, Clone, Debug, Identifiable, IncludesChanges)]
#[diesel(table_name = account)]
pub struct _DbAccountPatch {
    id: Uuid,
    updated_at: NaiveDateTime,
    email: Option<Option<String>>,
    username: Option<Option<String>>,
}

impl TryFrom<_DbAccount> for DbAccount {
    type Error = anyhow::Error;
    fn try_from(value: _DbAccount) -> Result<Self, Self::Error> {
        use anyhow::Error;
        Ok(Self {
            id: value.id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            deleted_at: value.deleted_at,
            identifier: match (value.email, value.username) {
                (Some(email), None) => Identifier::Email(email),
                (None, Some(username)) => Identifier::Username(username),
                (Some(_), Some(_)) => return Err(Error::msg("username and email are both non-null")),
                (None, None) => return Err(Error::msg("username and email are both null")),
            },
        })
    }
}

// Note: We could implement TryFrom<DbAccount> instead
// if the conversion to the Raw database representation
// is also fallible, although that is much less common.
impl From<DbAccount> for _DbAccount {
    fn from(value: DbAccount) -> Self {
        let (email, username) = match value.identifier {
            Identifier::Email(email) => (Some(email), None),
            Identifier::Username(username) => (None, Some(username)),
        };
        Self {
            id: value.id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            deleted_at: value.deleted_at,
            email,
            username,
        }
    }
}

impl From<DbAccountPatch> for _DbAccountPatch {
    fn from(value: DbAccountPatch) -> Self {
        let (email, username) = match value.identifier {
            Some(Identifier::Email(email)) => (Some(Some(email)), Some(None)),
            Some(Identifier::Username(username)) => (Some(None), Some(Some(username))),
            None => (None, None),
        };
        Self {
            id: value.id,
            updated_at: value.updated_at,
            email,
            username,
        }
    }
}
