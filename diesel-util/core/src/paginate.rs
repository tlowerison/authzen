use diesel::backend::Backend;
use diesel::prelude::*;
use diesel::query_builder::*;
use diesel::serialize::ToSql;
use diesel::sql_types::BigInt;
use itertools::Itertools;
use std::borrow::Borrow;
use std::cmp::{Ordering, PartialOrd};
use std::hash::Hash;

const DEFAULT_MAX_PAGINATE_COUNT: u32 = 100;
static SUBQUERY_ALIAS: &str = "t";
lazy_static! {
    pub static ref MAX_PAGINATE_COUNT: u32 = std::env::var("MAX_PAGINATE_COUNT")
        .map(|max_paginate_count| max_paginate_count.parse().unwrap_or(DEFAULT_MAX_PAGINATE_COUNT))
        .unwrap_or(DEFAULT_MAX_PAGINATE_COUNT);
    static ref WHERE_CLAUSE_INITIAL_FRAGMENT: String = format!(") {SUBQUERY_ALIAS} ) s where true ");
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Page {
    index: u32,
    count: u32,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct _Page {
    count: i64,
    left: i64,
    right: i64,
}

impl Page {
    pub fn new(index: u32, count: u32) -> Option<Page> {
        if count > *MAX_PAGINATE_COUNT {
            None
        } else {
            Some(Page { index, count })
        }
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn merge(pages: &[&Self]) -> Vec<Self> {
        let mut pages = pages.iter().map(|page| **page).collect::<Vec<Self>>();
        pages.sort();
        let mut merged = Vec::new();
        if !pages.is_empty() {
            merged.push(pages[0]);
        }
        for page in pages.into_iter().skip(1) {
            let merged_len = merged.len();
            let last_merged = &mut merged[merged_len - 1];
            let overlap = (last_merged.index + last_merged.count) as i64 - page.index as i64;
            if overlap >= 0 {
                last_merged.count += page.index - overlap as u32;
            } else {
                merged.push(page);
            }
        }
        merged
    }
}

impl From<(u32, u32)> for Page {
    fn from((index, count): (u32, u32)) -> Self {
        Self { index, count }
    }
}

impl _Page {
    fn merge(mut _pages: Vec<Self>) -> Vec<Self> {
        _pages.sort();
        let mut merged = Vec::new();
        if !_pages.is_empty() {
            merged.push(_pages[0]);
        }
        for _page in _pages.into_iter().skip(1) {
            let merged_len = merged.len();
            let last_merged = &mut merged[merged_len - 1];
            let overlap = last_merged.right - _page.left;
            if overlap >= 0 {
                last_merged.count += _page.left - overlap;
            } else {
                merged.push(_page);
            }
        }
        merged
    }
}

impl PartialOrd for Page {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        let lhs_left = self.index;
        let lhs_right = self.index + self.count;
        let rhs_left = rhs.index;
        let rhs_right = rhs.index + rhs.count;

        if lhs_left != rhs_left {
            lhs_left.partial_cmp(&rhs_left)
        } else {
            lhs_right.partial_cmp(&rhs_right)
        }
    }
}

impl Ord for Page {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.partial_cmp(rhs).unwrap()
    }
}

impl PartialOrd for _Page {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        if self.left != rhs.left {
            self.left.partial_cmp(&rhs.left)
        } else {
            self.right.partial_cmp(&rhs.right)
        }
    }
}

impl Ord for _Page {
    fn cmp(&self, rhs: &Self) -> Ordering {
        self.partial_cmp(rhs).unwrap()
    }
}

#[derive(Debug, Clone, QueryId)]
pub struct Paginated<T, Q = ()> {
    query: T,
    _pages: Option<Vec<_Page>>,
    partition: Option<Q>,
}

pub trait CanPaginate {
    fn as_page(&self) -> Option<&Page>;
}

pub trait Paginate: AsQuery + Sized {
    fn paginate<P: CanPaginate>(self, page: P) -> Paginated<Self::Query> {
        let page = page.as_page();
        Paginated {
            query: self.as_query(),
            _pages: page.map(|page| {
                vec![_Page {
                    count: page.count as i64,
                    left: (page.index * page.count) as i64,
                    right: (page.count + page.index * page.count) as i64,
                }]
            }),
            partition: None,
        }
    }

    fn multipaginate<P, I>(self, pages: I) -> Paginated<Self::Query>
    where
        P: for<'a> PageRef<'a>,
        I: Iterator<Item = P>,
    {
        Paginated {
            query: self.as_query(),
            _pages: Some(_Page::merge(
                pages
                    .map(|page| {
                        let page = page.page_ref();
                        _Page {
                            count: page.count as i64,
                            left: (page.index * page.count) as i64,
                            right: (page.count + page.index * page.count) as i64,
                        }
                    })
                    .collect(),
            )),
            partition: None,
        }
    }
}

// necessary to use instead of something like Borrow
// because coercion from double reference to single reference
// does not occur automatically when passing in a value that needs
// to implement Borrow<Page>
// e.g. &[&Page].iter() == Iterator<Item = &&Page> and &&Page: !Borrow<Page>
pub trait PageRef<'a> {
    fn page_ref(&'a self) -> &'a Page;
}

impl AsRef<Page> for Page {
    fn as_ref(&self) -> &Page {
        self
    }
}

impl<'a, T> PageRef<'a> for T
where
    T: AsRef<Page>,
{
    fn page_ref(&'a self) -> &'a Page {
        self.as_ref()
    }
}

impl<T, Q> Paginated<T, Q> {
    pub fn partition<Expr>(self, expr: Expr) -> Paginated<T, Expr>
    where
        Expr: Partition + Send,
    {
        Paginated {
            query: self.query,
            _pages: self._pages,
            partition: Some(expr),
        }
    }
}

pub trait Partition {
    fn encode(&self) -> Result<String, diesel::result::Error>;
}

impl Partition for () {
    fn encode(&self) -> Result<String, diesel::result::Error> {
        Ok("".into())
    }
}

macro_rules! partition {
    ($($len:literal: $($gen:ident)+),*$(,)?) => {
        $(
            #[allow(unused_parens)]
            impl<$($gen),+> Partition for ($($gen,)+)
            where
                $($gen: diesel::Column),+
            {
                fn encode(&self) -> Result<String, diesel::result::Error> {
                    let unique_min_column_names = [$(
                        format!("{SUBQUERY_ALIAS}.{}", <$gen as diesel::Column>::NAME.split(".").last().unwrap()),
                    )+]
                        .into_iter()
                        .unique()
                        .collect::<Vec<_>>();
                    if unique_min_column_names.len() < $len {
                        return Err(diesel::result::Error::QueryBuilderError("could not encode group by clause as a row number partition for pagination because the column names included in the group by clause have identical names".into()));
                    }
                    Ok(unique_min_column_names.join(", "))
                }
            }
        )*
    };
}

impl<T: AsQuery> Paginate for T {}

impl<T: Query, Q> Query for Paginated<T, Q> {
    type SqlType = T::SqlType;
}

impl<C: Connection, T, Q> RunQueryDsl<C> for Paginated<T, Q> {}

impl<DB, T, Q> QueryFragment<DB> for Paginated<T, Q>
where
    DB: Backend,
    T: QueryFragment<DB>,
    Q: Partition,
    i64: ToSql<BigInt, DB>,
{
    fn walk_ast<'b>(&'b self, mut pass: AstPass<'_, 'b, DB>) -> QueryResult<()> {
        let _pages = match &self._pages {
            Some(_pages) => _pages,
            None => {
                self.query.walk_ast(pass.reborrow())?;
                return Ok(());
            }
        };
        if _pages.is_empty() {
            return Err(diesel::result::Error::QueryBuilderError(
                "no pages specified for a paginated query".into(),
            ));
        }
        pass.push_sql("select * from (select *, row_number() over (");
        if let Some(partition) = self.partition.as_ref() {
            pass.push_sql("partition by ");
            pass.push_sql(&partition.encode()?);
        }
        pass.push_sql(") as offset from ( ");
        self.query.walk_ast(pass.reborrow())?;
        pass.push_sql(&WHERE_CLAUSE_INITIAL_FRAGMENT);
        for _page in _pages.iter() {
            // row_number starts at 1
            pass.push_sql(" and s.offset > ");
            pass.push_bind_param::<BigInt, _>(&_page.left)?;
            pass.push_sql(" and s.offset <= ");
            pass.push_bind_param::<BigInt, _>(&_page.right)?;
        }
        Ok(())
    }
}

partition!(
     1: A,
     2: A B,
     3: A B C,
     4: A B C D,
     5: A B C D E,
     6: A B C D E F,
     7: A B C D E F G,
     8: A B C D E F G H,
     9: A B C D E F G H I,
    10: A B C D E F G H I J,
    11: A B C D E F G H I J K,
    12: A B C D E F G H I J K L,
    13: A B C D E F G H I J K L M,
    14: A B C D E F G H I J K L M N,
    15: A B C D E F G H I J K L M N O,
    16: A B C D E F G H I J K L M N O P,
    17: A B C D E F G H I J K L M N O P Q,
    18: A B C D E F G H I J K L M N O P Q R,
    19: A B C D E F G H I J K L M N O P Q R S,
    20: A B C D E F G H I J K L M N O P Q R S T,
    21: A B C D E F G H I J K L M N O P Q R S T U,
    22: A B C D E F G H I J K L M N O P Q R S T U V,
    23: A B C D E F G H I J K L M N O P Q R S T U V W,
    24: A B C D E F G H I J K L M N O P Q R S T U V W X,
    25: A B C D E F G H I J K L M N O P Q R S T U V W X Y,
    26: A B C D E F G H I J K L M N O P Q R S T U V W X Y Z,
);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Paged<K> {
    pub page: Page,
    pub key: K,
}

impl<K> AsRef<Page> for Paged<K> {
    fn as_ref(&self) -> &Page {
        &self.page
    }
}

impl<P: Borrow<Page>> CanPaginate for P {
    fn as_page(&self) -> Option<&Page> {
        Some(self.borrow())
    }
}

impl CanPaginate for Option<Page> {
    fn as_page(&self) -> Option<&Page> {
        self.as_ref()
    }
}

impl CanPaginate for &Option<Page> {
    fn as_page(&self) -> Option<&Page> {
        self.as_ref()
    }
}

impl CanPaginate for Option<&Page> {
    fn as_page(&self) -> Option<&Page> {
        *self
    }
}

impl CanPaginate for &Option<&Page> {
    fn as_page(&self) -> Option<&Page> {
        **self
    }
}
