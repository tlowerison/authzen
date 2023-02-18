use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub trait Enum: Clone + Sized {
    type Variants: Iterator<Item = Self>;
    fn id(&self) -> Uuid;
    fn variants() -> Self::Variants;
    fn with_title(&self) -> WithTitle<Self>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WithTitle<T> {
    pub id: Uuid,
    pub title: T,
}
