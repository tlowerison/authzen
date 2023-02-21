use float_cmp::approx_eq;
use serde_json::Value;
use std::borrow::Cow;
use std::fmt::{Debug, Display};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

#[derive(Clone, Debug, Deserialize)]
pub struct OPAResult<T> {
    pub result: T,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum OPAPolicyASTNode<'a> {
    Array(Box<Vec<OPAPolicyASTNode<'a>>>),
    Boolean(bool),
    Call(Box<Vec<OPAPolicyASTNode<'a>>>),
    Number(f64),
    Ref(OPAPolicyASTNodeRef<'a>),
    Set(Vec<serde_json::Value>),
    String(&'a str),
    Var(&'a str),
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct OPAPolicyASTNodeRef<'a>(#[serde(borrow)] pub Vec<OPAPolicyPathNode<'a>>);

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum OPAPolicyPathNode<'a> {
    Number(usize),
    String(&'a str),
    Var(&'a str),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct OPAPolicyASTNodeRefRef<'a: 'b, 'b>(pub CowSlice<'b, OPAPolicyPathNode<'a>>);

impl<'a> OPAPolicyASTNode<'a> {
    pub fn deserialize_terms<'de: 'a, D>(deserializer: D) -> Result<Vec<OPAPolicyASTNode<'a>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(
            match <_OPAPolicyASTNodeTerms as serde::Deserialize>::deserialize(deserializer)? {
                _OPAPolicyASTNodeTerms::Single(opa_policy_ast_node) => vec![opa_policy_ast_node],
                _OPAPolicyASTNodeTerms::Multiple(opa_policy_ast_nodes) => opa_policy_ast_nodes,
            },
        )
    }
}

impl<'a> TryFrom<&'a OPAPolicyASTNodeRef<'_>> for Vec<&'a str> {
    type Error = ();
    fn try_from(node_ref: &'a OPAPolicyASTNodeRef<'_>) -> Result<Self, Self::Error> {
        let mut values = Vec::with_capacity(node_ref.0.len());
        for node in node_ref.0.iter() {
            match node {
                OPAPolicyPathNode::Number(_) => return Err(()),
                OPAPolicyPathNode::String(string) => values.push(*string),
                OPAPolicyPathNode::Var(var) => values.push(*var),
            }
        }
        Ok(values)
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum _OPAPolicyASTNodeTerms<'a> {
    Single(#[serde(borrow)] OPAPolicyASTNode<'a>),
    Multiple(#[serde(borrow)] Vec<OPAPolicyASTNode<'a>>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CowSlice<'a, T> {
    Borrowed(&'a [T]),
    Owned(Vec<T>),
}

impl Eq for OPAPolicyASTNode<'_> {}

impl PartialEq for OPAPolicyASTNode<'_> {
    fn eq(&self, other: &Self) -> bool {
        match self {
            Self::Array(array1) => {
                if let Self::Array(array2) = other {
                    array1.eq(array2)
                } else {
                    false
                }
            }
            Self::Boolean(boolean1) => {
                if let Self::Boolean(boolean2) = other {
                    boolean1.eq(boolean2)
                } else {
                    false
                }
            }
            Self::Call(call1) => {
                if let Self::Call(call2) = other {
                    call1.eq(call2)
                } else {
                    false
                }
            }
            Self::Number(number1) => {
                if let Self::Number(number2) = other {
                    approx_eq!(f64, *number1, *number2, ulps = 2)
                } else {
                    false
                }
            }
            Self::Ref(ref1) => {
                if let Self::Ref(ref2) = other {
                    ref1.0.eq(&ref2.0)
                } else {
                    false
                }
            }
            Self::Set(set1) => {
                if let Self::Set(set2) = other {
                    set1.eq(set2)
                } else {
                    false
                }
            }
            Self::String(string1) => {
                if let Self::String(string2) = other {
                    string1.eq(string2)
                } else {
                    false
                }
            }
            Self::Var(var1) => {
                if let Self::Var(var2) = other {
                    var1.eq(var2)
                } else {
                    false
                }
            }
        }
    }
}

impl TryInto<Value> for &OPAPolicyASTNode<'_> {
    type Error = anyhow::Error;
    fn try_into(self) -> Result<Value, Self::Error> {
        Ok(match self {
            OPAPolicyASTNode::Array(array) => {
                Value::Array(array.iter().map(TryInto::try_into).collect::<Result<_, _>>()?)
            }
            OPAPolicyASTNode::Boolean(bool) => Value::Bool(*bool),
            OPAPolicyASTNode::Call(_) => {
                return Err(anyhow::Error::msg(
                    "cannot convert opa policy ast `call` node into a json value",
                ))
            }
            OPAPolicyASTNode::Number(number) => Value::Number(serde_json::Number::from_f64(*number).unwrap()),
            OPAPolicyASTNode::Ref(_) => {
                return Err(anyhow::Error::msg(
                    "cannot convert opa policy ast `ref` node into a json value",
                ))
            }
            OPAPolicyASTNode::Set(_) => {
                return Err(anyhow::Error::msg(
                    "cannot convert opa policy ast `set` node into a json value",
                ))
            }
            OPAPolicyASTNode::String(string) => Value::String(string.to_string()),
            OPAPolicyASTNode::Var(_) => {
                return Err(anyhow::Error::msg(
                    "cannot convert opa policy ast `var` node into a json value",
                ))
            }
        })
    }
}

impl<'a> From<Vec<OPAPolicyPathNode<'a>>> for OPAPolicyASTNodeRef<'a> {
    fn from(path_nodes: Vec<OPAPolicyPathNode<'a>>) -> Self {
        Self(path_nodes)
    }
}

impl<'a: 'b, 'b> From<&'b OPAPolicyASTNodeRef<'a>> for OPAPolicyASTNodeRefRef<'a, 'b> {
    fn from(opa_policy_ast_node_ref: &'b OPAPolicyASTNodeRef<'a>) -> OPAPolicyASTNodeRefRef<'a, 'b> {
        OPAPolicyASTNodeRefRef(CowSlice::Borrowed(&opa_policy_ast_node_ref.0[..]))
    }
}

impl Display for OPAPolicyASTNodeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", OPAPolicyASTNodeRefRef(CowSlice::Borrowed(&self.0[..])))
    }
}

impl Display for OPAPolicyASTNodeRefRef<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() > 0 {
            match &self.0[0] {
                OPAPolicyPathNode::Number(number) => write!(f, "{number}")?,
                OPAPolicyPathNode::String(str) => write!(f, "{str}")?,
                OPAPolicyPathNode::Var(var) => write!(f, "{var}")?,
            }
        }
        for path_node in self.0.iter().skip(1) {
            match path_node {
                OPAPolicyPathNode::Number(number) => write!(f, "[{number}]")?,
                OPAPolicyPathNode::String(str) => write!(f, ".{str}")?,
                OPAPolicyPathNode::Var(var) => write!(f, ".{var}")?,
            }
        }
        Ok(())
    }
}

impl<'a> Deref for OPAPolicyASTNodeRef<'a> {
    type Target = Vec<OPAPolicyPathNode<'a>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq<&str> for OPAPolicyPathNode<'_> {
    fn eq(&self, rhs: &&str) -> bool {
        match self {
            Self::Number(_) => false,
            Self::String(str) => str == rhs,
            Self::Var(str) => str == rhs,
        }
    }
}

impl PartialEq<usize> for OPAPolicyPathNode<'_> {
    fn eq(&self, rhs: &usize) -> bool {
        match self {
            Self::Number(number) => number == rhs,
            Self::String(_) | Self::Var(_) => false,
        }
    }
}

impl ToString for OPAPolicyPathNode<'_> {
    fn to_string(&self) -> String {
        match self {
            Self::Number(number) => format!("{number}"),
            Self::String(str) => str.to_string(),
            Self::Var(str) => str.to_string(),
        }
    }
}

impl<T: Clone> CowSlice<'_, T> {
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        match self {
            Self::Borrowed(slice) => slice.iter(),
            Self::Owned(vec) => vec.iter(),
        }
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        match self {
            Self::Borrowed(slice) => slice.len(),
            Self::Owned(vec) => vec.len(),
        }
    }
}

impl<T: Hash> Hash for CowSlice<'_, T> {
    fn hash<H: Hasher>(&self, hasher: &mut H) {
        match self {
            Self::Borrowed(slice) => slice.hash(hasher),
            Self::Owned(vec) => vec.hash(hasher),
        }
    }
}

impl<T> std::ops::Index<usize> for CowSlice<'_, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        match self {
            Self::Borrowed(slice) => &slice[index],
            Self::Owned(vec) => &vec[index],
        }
    }
}

impl<T> From<Vec<T>> for CowSlice<'_, T> {
    fn from(vec: Vec<T>) -> Self {
        Self::Owned(vec)
    }
}

impl<'a, T> From<&'a [T]> for CowSlice<'a, T> {
    fn from(slice: &'a [T]) -> Self {
        Self::Borrowed(slice)
    }
}

impl<'a: 'b, 'b> From<Cow<'b, OPAPolicyASTNodeRef<'a>>> for OPAPolicyASTNodeRefRef<'a, 'b> {
    fn from(node_ref: Cow<'b, OPAPolicyASTNodeRef<'a>>) -> Self {
        OPAPolicyASTNodeRefRef(match node_ref {
            Cow::Borrowed(borrowed) => CowSlice::Borrowed(borrowed.as_slice()),
            Cow::Owned(owned) => CowSlice::Owned(owned.0),
        })
    }
}
