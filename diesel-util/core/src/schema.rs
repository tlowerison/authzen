use serde_json::Value;

pub use lazy_static::lazy_static as diesel_util_lazy_static;

use diesel::sql_types::{is_nullable::*, *};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct DynamicSchema {
    pub name: &'static str,
    pub tables: HashSet<Table>,
    pub joinables: HashMap<JoinablePair<'static>, Joinable<'static>>,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub name: &'static str,
    pub primary_key: &'static str,
    pub columns: HashSet<Column>,
}

impl Borrow<str> for Table {
    fn borrow(&self) -> &str {
        self.name
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Column {
    pub name: &'static str,
    pub sql_type: ColumnSqlType,
}

impl Borrow<str> for Column {
    fn borrow(&self) -> &str {
        self.name
    }
}

#[derive(AsVariant, Clone, Derivative, IsVariant)]
#[derivative(Debug)]
pub enum ColumnSqlType {
    IsNullable(#[derivative(Debug = "ignore")] Arc<Box<dyn Send + Sync + SqlType<IsNull = IsNullable>>>),
    NotNull(#[derivative(Debug = "ignore")] Arc<Box<dyn Send + Sync + SqlType<IsNull = NotNull>>>),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct JoinablePair<'a> {
    pub child_table_name: &'a str,
    pub parent_table_name: &'a str,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Joinable<'a> {
    pub child_table_name: &'a str,
    pub parent_table_name: &'a str,
    pub child_table_foreign_key_column_name: &'a str,
}

impl DynamicSchema {
    pub fn get_join_on(&self, table_name1: &str, table_name2: &str) -> Option<&str> {
        self.joinables
            .get(&JoinablePair {
                parent_table_name: table_name1,
                child_table_name: table_name2,
            })
            .map(|joinable| joinable.child_table_foreign_key_column_name)
    }
}

impl Eq for Table {}

impl Hash for Table {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.name.hash(hasher)
    }
}

impl PartialEq for Table {
    fn eq(&self, rhs: &Self) -> bool {
        self.name == rhs.name
    }
}

impl<'a> PartialEq<&'a str> for Table {
    fn eq(&self, rhs: &&'a str) -> bool {
        self.name == *rhs
    }
}

impl PartialEq<str> for Table {
    fn eq(&self, rhs: &str) -> bool {
        self.name == rhs
    }
}

impl PartialEq<String> for Table {
    fn eq(&self, rhs: &String) -> bool {
        self.name == *rhs
    }
}

impl Eq for Column {}

impl Hash for Column {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        self.name.hash(hasher)
    }
}

impl PartialEq for Column {
    fn eq(&self, rhs: &Self) -> bool {
        self.name == rhs.name
    }
}

use diesel::backend::{Backend, DieselReserveSpecialization};
use diesel::expression::BoxableExpression;
use diesel::query_dsl::methods::{BoxedDsl, FilterDsl};

pub trait SchemaExpressionMethods {
    fn try_filter_eq<'a, DB, Q, O>(
        &self,
        query: Q,
        table_name: &str,
        column_name: &str,
        value: &'a Value,
    ) -> Result<O, anyhow::Error>
    where
        DB: Backend + DieselReserveSpecialization,
        Q: FilterDsl<Box<dyn BoxableExpression<Q, DB, SqlType = Bool>>>,
        <Q as FilterDsl<Box<dyn BoxableExpression<Q, DB, SqlType = Bool>>>>::Output: BoxedDsl<'a, DB, Output = O>;
}
