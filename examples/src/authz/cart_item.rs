use crate::*;
use authzen::*;
use std::borrow::Cow;

#[derive(AsRef, AuthzObject, Clone, Debug, Deref, From, Into, Serialize)]
#[authzen(service = "authzen_examples", ty = "cart_item")]
pub struct CartItem<'a>(pub Cow<'a, DbCartItem>);
