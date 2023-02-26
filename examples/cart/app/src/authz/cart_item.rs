use crate::*;
use authzen::*;
use std::borrow::Cow;

#[derive(AsRef, AuthzObject, Clone, Debug, Deref, Deserialize, From, Into, Serialize)]
#[authzen(service = "examples_cart", ty = "cart_item")]
pub struct CartItem<'a>(pub Cow<'a, DbCartItem>);
