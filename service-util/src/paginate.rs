#[cfg(feature = "graphql")]
use async_graphql::*;
use diesel_util::{Page, MAX_PAGINATE_COUNT};
use serde::{Deserialize, Serialize};

/// useful for unwrapping pages
/// and returning early if a page count is 0
#[macro_export]
macro_rules! check_page {
    ($expr: expr) => {{
        let expr = $expr;
        if expr.is_empty() {
            return Ok(vec![]);
        }
        TryInto::<Page>::try_into(expr)?
    }};
}

const DEFAULT_COUNT: usize = 25;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "graphql", derive(InputObject), graphql(name = "Page"))]
pub struct ApiPage {
    pub index: usize,
    pub count: usize,
}

impl ApiPage {
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum ApiPageError {
    #[error("Invalid paginate.count: larger than maximum allowed value {}", *MAX_PAGINATE_COUNT)]
    CountTooLarge,
}

impl TryInto<Page> for ApiPage {
    type Error = ApiPageError;

    fn try_into(self) -> Result<Page, Self::Error> {
        Page::new(self.index as u32, self.count as u32).ok_or(Self::Error::CountTooLarge)
    }
}

impl Default for ApiPage {
    fn default() -> Self {
        Self {
            index: 0,
            count: DEFAULT_COUNT,
        }
    }
}
