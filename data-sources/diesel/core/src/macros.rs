use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[doc(hidden)]
pub trait Enum: Clone + Sized {
    type Variants: Iterator<Item = Self>;
    fn id(&self) -> Uuid;
    fn variants() -> Self::Variants;
    fn with_title(&self) -> WithTitle<Self>;
}

#[doc(hidden)]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WithTitle<T> {
    pub id: Uuid,
    pub title: T,
}

pub trait IncludesChanges {
    fn includes_changes(&self) -> bool;
}
