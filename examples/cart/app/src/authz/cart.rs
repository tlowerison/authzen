use crate::*;
use authzen::*;
use std::borrow::Cow;

#[derive(AsRef, AuthzObject, Clone, Debug, Deref, Deserialize, From, Into, Serialize)]
#[authzen(service = "examples_cart", ty = "cart")]
pub struct Cart<'a>(pub Cow<'a, DbCart>);
