use crate::*;
use authzen::*;
use std::borrow::Cow;

#[derive(AsRef, AuthzObject, Clone, Debug, Deref, Deserialize, From, Into, Serialize)]
#[authzen(service = "examples_cart", ty = "account")]
pub struct Account<'a>(pub Cow<'a, DbAccount>);
