# diesel
Integration with diesel as a storage client is fully supported and comes with some features.
The base export is [DbEntity](https://docs.rs/authzen-diesel/latest/authzen_diesel/trait.DbEntity.html), which is automatically implemented for any type which
implements [diesel::associations::HasTable](https://docs.rs/diesel/latest/diesel/associations/trait.HasTable.html). Of note, this includes all types which implement [diesel::Insertable](https://docs.rs/diesel/latest/diesel/trait.Insertable.html).
Types which implement `DbEntity` can then implement the following operation traits:
- [DbGet](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbGet.html) provides utility methods for retrieving records
  - [get](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbGet.html#method.get): given a collection of ids for this `DbEntity`, retrieve the corresponding records
  - [get_one](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbGet.html#method.get_one): given an id for this `DbEntity`, retrieve the corresponding record
  - [get_by_column](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbGet.html#method.get_by_column): given a column belonging to this `DbEntity`'s table and a collection of values which
    match the sql type of that column, get all corresponding records
  - [get_page](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbGet.html#method.get_page): given a [Page](https://docs.rs/authzen-diesel/latest/authzen_diesel/paginate/struct.Page.html), return
    a collection of records that match the specified page; currently Page only supports (index, count) matching but a goal is to also support cursor based pagination in the future
  - [get_pages](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbGet.html#method.get_pages): given a collection of pages, return records matching any of the pages; note that this method only
    makes one database query :)
- [DbInsert](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbInsert.html)
  - [DbInsert::Post](https://docs.rs/authzen-diesel/0.1.0-alpha.0/authzen_diesel/operations/trait.DbInsert.html#associatedtype.Post):
      the data type which will actually be used to insert the record -- for any type `T` which implements `Insertable`, `T` will automatically implement `DbInsert<Post = T>`
  - [DbInsert::PostHelper](https://docs.rs/authzen-diesel/0.1.0-alpha.0/authzen_diesel/operations/trait.DbInsert.html#associatedtype.PostHelper):
      the data type which will be passed to `insert`; this defaults to `DbInsert::Post`, however if you have a data type which you want to use to represent database records
      but which cannot directly implement `Insertable`, `PostHelper` can be set to that type and then at the time of insert it will be converted to the `DbInsert::Post` type
  - [insert](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbInsert.html#method.insert): given a collection of `DbInsert::PostHelper` types, insert them into the database;
    note that if this type implementing `DbInsert` also implements [Audit](https://docs.rs/authzen-diesel/latest/authzen_diesel/audit/trait.Audit.html), then audit records will be automatically
    inserted for all records inserted as well
- [DbUpdate](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbUpdate.html)
  - [DbUpdate::Patch](https://docs.rs/authzen-diesel/0.1.0-alpha.0/authzen_diesel/operations/trait.DbUpdate.html#associatedtype.Patch):
      the data type which will actually be used to update the record -- for any type `T` which implements `Changeset`, `T` will automatically implement `DbUpdate<Patch = T>`
  - [DbUpdate::PatchHelper](https://docs.rs/authzen-diesel/0.1.0-alpha.0/authzen_diesel/operations/trait.DbUpdate.html#associatedtype.PatchHelper):
      the data type which will be passed to `insert`; this defaults to `DbUpdate::Patch`, however if you have a data type which you want to use to represent database records
      but which cannot directly implement `Insertable`, `PatchHelper` can be set to that type and then at the time of insert it will be converted to the `DbUpdate::Patch` type
### Example
The PostHelper/PatchHelper terminology in DbInsert/DbUpdate can be a little confusing without an example. The major win from this design is the ability to represent discriminated unions in tables easily and safely.
As an example, let's take a case where an `item` table can either be an inventory item or a general item. General items have no count, while inventory items do have a count of how many are owned and how many are needed.
A typical representation of this table with a diesel model would just use an option for the two counts, and we will use that as a "raw" model, but the type we'd rather work with in service code is one which makes the
distinction between the two item types with an enum.
```rust
use authzen::data_sources::diesel::prelude::*;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct DbItem {
    pub id: Uuid,
    pub item_type: ItemType,
}

#[derive(Clone, Debug)]
pub enum ItemType {
    General,
    Inventory {
        owned: i32,
        needed: i32,
    },
}

/// raw diesel model
/// note that this type automatically implements DbInsert<Post<'v> = _DbItem, PostHelper<'v> = _DbItem>
#[derive(Clone, Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = item)]
pub struct _DbItem {
    pub id: Uuid,
    pub is_inventory_type: bool,
    pub owned: Option<i32>,
    pub needed: Option<i32>,
}

/// for DbItem to implement DbInsert<Post<'v> = _DbItem, PostHelper<'v> = DbItem>, DbItem must implement Into<_DbItem>
impl From<DbItem> for _DbItem {
    fn from(value: DbItem) -> Self {
        let (is_inventory_type, owned, needed) = match value.item_type {
            ItemType::General => (false, None, None),
            ItemType::Inventory { owned, needed } => (true, Some(owned), Some(needed)),
        };
        Self { id: value.id, is_inventory_type, owned, needed }
    }
}

/// to be able to call <DbItem as DbInsert>::insert, _DbItem must implement TryInto<DbItem>
impl TryFrom<_DbItem> for DbItem {
    type Error = anyhow::Error;
    fn try_from(value: _DbItem) -> Result<Self, Self::Error> {
      let item_type = match (value.is_inventory_type, value.owned, value.needed) {
          (false, None, None) => ItemType::General,
          (true, Some(owned), Some(needed)) => ItemType::InventoryType { owned, needed },
          (is_inventory_type, owned, needed) => return Err(anyhow::Error::msg(format!(
            "unexpected inventory type found in database record: is_inventory_type = {is_inventory_type}, owned = {owned:#?}, needed = {needed:#?}",
          ))),
      };
      Ok(Self { id: value.id, item_type })
    }
}


impl DbInsert for DbItem {
    type Post<'v> = _DbItem;
}

/// service code
///
/// the Db trait here is imported in the authzen diesel prelude
/// it is a wrapper type for various types which allow us to
/// asynchronously get a diesel connection, i.e. it's implemented
/// for diesel_async::AsyncPgConnection as well as various connection pools
pub fn insert_an_item<D: Db>(db: &D, db_item: DbItem) -> Result<DbItem, DbEntityError<anyhow::Error>> {
    DbItem::insert_one(db, db_item).await
}
```

Adding in updates would look like this:
```rust
#[derive(Clone, Debug)]
pub struct DbItemPatch {
    pub id: Uuid,
    pub item_type: Option<ItemTypePatch>,
}

#[derive(Clone, Debug)]
pub enum ItemTypePatch {
    General,
    Inventory {
        owned: Option<i32>,
        needed: Option<i32>,
    },
}

#[derive(AsChangeset, Clone, Debug, Changeset, Identifiable, IncludesChanges)]
pub struct _DbItemPatch {
    pub id: Uuid,
    pub is_inventory_type: Option<bool>,
    pub owned: Option<Option<i32>>,
    pub needed: Option<Option<i32>>,
}

/// for DbItem to implement DbUpdate<Patch<'v> = _DbItemPatch, PatchHelper<'v> = DbItemPatch>,
/// DbItemPatch must implement Into<_DbItemPatch>
impl From<DbItemPatch> for _DbItemPatch {
    fn from(value: DbItemPatch) -> Self {
        let (is_inventory_type, owned, needed) = match value.item_type {
            None => (None, None, None),
            Some(ItemTypePatch::General) => (Some(false), Some(None), Some(None)),
            Some(ItemTypePatch::InventoryType { owned, needed }) => (Some(true), Some(owned), Some(needed)),
        };
        Self { id: value.id, is_inventory_type, owned, needed }
    }
}

impl DbUpdate for DbItem {
    type Patch<'v> = _DbItemPatch;
    type PatchHelper<'v> = DbItemPatch;
}

/// service code
pub fn update_an_item<D: Db>(db: &D, db_item_patch: DbItemPatch) -> Result<DbItem, DbEntityError<anyhow::Error>> {
    DbItem::update_one(db, db_item_patch).await
}
```

If you also want to create a record in a separate table `item_audit` any time a new record is inserted to or updated in the `item` table,
this can be achieved automatically any time `DbInsert::insert` or `DbUpdate::update` are called by deriving [Audit](https://docs.rs/authzen-diesel/latest/authzen_diesel/audit/trait.Audit.html)
on `_DbItem`. This example assumes the table `item_audit` is defined like this
```sql
create table if not exists item_audit (
    id                 uuid     primary key,
    item_id            uuid     not null,
    is_inventory_type  boolean  not null,
    owned              int,
    needed             int,
    foreign key (item_id) references item (id)
);
```
Note that the placement of `item_id` as the second column in the table *is required*, otherwise, there is a chance that
the diesel table model will still compile but with the ids swapped for example if the `id` and `item_id` columns are swapped in the sql table definition.
```rust
#[derive(Audit, Clone, Debug, Identifiable, Insertable, Queryable)]
#[audit(foreign_key = item_id)]
#[diesel(table_name = item)]
pub struct _DbItem {
    pub id: Uuid,
    pub is_inventory_type: bool,
    pub owned: Option<i32>,
    pub needed: Option<i32>,
}
```
The inclusion of `#[audit(foreign_key = item_id)]` is only necessary if the audit foreign key back to the original table does not follow the naming scheme `{original_table_name}_id`.
So the above example could be reduced as below since the foreign key's name is `item_id` which follows the expected audit foreign key naming scheme.
```rust
#[derive(Audit, Clone, Debug, Identifiable, Insertable, Queryable)]
#[diesel(table_name = item)]
pub struct _DbItem {
    pub id: Uuid,
    pub is_inventory_type: bool,
    pub owned: Option<i32>,
    pub needed: Option<i32>,
}
```


Soft deletes are also supported out of the box:
- queries used in any of the `DbGet` methods will omit records for which the soft delete column is not null
- deletions will update the soft deleted column to the current timestamp rather than deleting the record from the database
```rust
#[derive(Audit, Clone, Debug, Identifiable, Insertable, Queryable, SoftDelete)]
#[audit(foreign_key = item_id)]
#[diesel(table_name = item)]
#[soft_delete(db_entity = DbAccount, deleted_at = deleted_at)]
pub struct _DbAccount {
    pub id: Uuid,
    pub deleted_at: Option<chrono::NaiveDateTime>,
    pub is_inventory_type: bool,
    pub owned: Option<i32>,
    pub needed: Option<i32>,
}
```
Note that updates can be still made on records which have already been soft deleted (not sure yet if this behavior is desirable; at the very least, it gives the ability to un-delete easily).

