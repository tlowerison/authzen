use crate::*;
use authzen::*;
use std::borrow::Cow;

#[derive(AsRef, AuthzObject, Clone, Debug, Deref, From, Into, Serialize)]
#[authzen(service = "authzen_examples", ty = "account")]
pub struct Account<'a>(pub Cow<'a, DbAccount>);
