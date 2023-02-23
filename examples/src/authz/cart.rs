use crate::*;
use authzen::*;
use std::borrow::Cow;

#[derive(AsRef, AuthzObject, Clone, Debug, Deref, From, Into, Serialize)]
#[authzen(service = "authzen_examples", ty = "cart")]
pub struct Cart<'a>(pub Cow<'a, DbCart>);
