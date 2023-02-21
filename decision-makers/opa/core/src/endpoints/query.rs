use crate::OPA_EXPLAIN;
use futures::future::try_join_all;
use hyper::{body::Bytes, http::header::*, Body, Method};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use service_util::*;
use std::borrow::Cow;
use std::fmt::Debug;
use uuid::Uuid;

const DEFAULT_DATA_PATH: &str = "app";
const DEFAULT_QUERY: &str = "authz";

pub async fn query_all<'a, Data: Clone + Debug + Send + Serialize + Sync + 'a>(
    opa_client: &crate::OPAClient,
    queries: impl IntoIterator<Item = OPAQuery<'a, Data>>,
) -> Result<(), anyhow::Error> {
    let allowed: Vec<OPAQueryResult> = try_join_all(queries.into_iter().map(|query_input| async move {
        let result: OPAQueryResult = query_input.query(opa_client).await?;
        Ok::<OPAQueryResult, anyhow::Error>(result)
    }))
    .await?;

    if !allowed.into_iter().all(Into::into) {
        return Err(anyhow::Error::msg("Unauthorized"));
    }

    Ok(())
}

#[derive(Clone, Debug, Deserialize, Serialize, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
#[skip_serializing_none]
pub struct OPAQuery<'a, Data: Clone = Value> {
    #[serde(borrow)]
    pub input: Cow<'a, OPAQueryInput<'a, Data>>,
    #[builder(default)]
    #[serde(skip_serializing)]
    pub config: OPAQueryConfig<'a>,
    #[builder(default)]
    pub data: Option<Value>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[skip_serializing_none]
pub struct OPAQueryConfig<'a> {
    #[builder(default = DEFAULT_DATA_PATH)]
    #[serde(skip_serializing)]
    pub data_path: &'a str,
    #[builder(default = DEFAULT_QUERY)]
    #[serde(skip_serializing)]
    pub query: &'a str,
    #[builder(default = OPA_EXPLAIN.as_ref().map(|x| &**x))]
    pub explain: Option<&'a str>,
    pub pretty: Option<bool>,
    pub instrument: Option<bool>,
    pub metrics: Option<bool>,
}

impl Default for OPAQueryConfig<'_> {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Clone, Derivative, Deserialize, Serialize, TypedBuilder)]
#[derivative(Debug)]
#[skip_serializing_none]
pub struct OPAQueryInput<'a, Data = Value> {
    #[derivative(Debug = "ignore")]
    pub token: Option<&'a str>,
    #[builder(setter(into))]
    pub action: OPAQueryInputAction<'a, Data>,
    #[builder(default)]
    pub transaction_id: Option<Uuid>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "object")]
pub enum OPAQueryInputAction<'a, Data = Value> {
    Create {
        service: Cow<'a, str>,
        entity: Cow<'a, str>,
        records: Vec<Data>,
    },
    Delete {
        service: Cow<'a, str>,
        entity: Cow<'a, str>,
        ids: Vec<Uuid>,
    },
    Read {
        service: Cow<'a, str>,
        entity: Cow<'a, str>,
        ids: Vec<Uuid>,
    },
    Update {
        service: Cow<'a, str>,
        entity: Cow<'a, str>,
        patches: Vec<Data>,
    },
}

impl<Data: Clone + Serialize> Endpoint for OPAQuery<'_, Data> {
    const METHOD: Method = Method::POST;
    type Params<'a> = OPAQueryConfig<'a> where Self: 'a;

    fn params(&self) -> Self::Params<'_> {
        self.config
    }
    fn path(&self) -> Path {
        format!("/v1/data/{}/{}", self.config.data_path, self.config.query).into()
    }
    fn headers(&self) -> HeaderMap {
        HeaderMap::from_iter(vec![(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap())])
    }
    fn body(&self) -> Body {
        let body = serde_json::to_string(&self).unwrap();
        if *crate::OPA_DEBUG {
            info!("OPA Request: {}", self.path());
            info!("{body}");
        }
        Body::from(Bytes::copy_from_slice(body.as_bytes()))
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut, Eq, From, Into, Ord, PartialEq, PartialOrd, Serialize)]
pub struct OPAQueryResult(pub bool);

impl PartialEq<bool> for OPAQueryResult {
    fn eq(&self, rhs: &bool) -> bool {
        self.0 == *rhs
    }
}

impl PartialEq<OPAQueryResult> for bool {
    fn eq(&self, rhs: &OPAQueryResult) -> bool {
        *self == rhs.0
    }
}

#[derive(Clone, Debug, Deserialize)]
struct _OPAQueryResult {
    result: Option<Value>,
}

impl<'de> Deserialize<'de> for OPAQueryResult {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let _simple_query_result = _OPAQueryResult::deserialize(deserializer)?;
        Ok(OPAQueryResult(
            _simple_query_result
                .result
                .map(|result| if let Value::Bool(bool) = result { bool } else { false })
                .unwrap_or_default(),
        ))
    }
}

impl OPAQueryResult {
    pub fn ok_or<E>(self, e: E) -> Result<(), E> {
        if self.0 {
            Ok(())
        } else {
            Err(e)
        }
    }

    pub fn ok_or_else<E, F: FnOnce() -> E>(self, f: F) -> Result<(), E> {
        if self.0 {
            Ok(())
        } else {
            Err(f())
        }
    }
}

impl<'a, Data: Clone> OPAQueryInput<'a, Data> {
    pub fn map_data<T: Clone, F>(self, f: F) -> OPAQueryInput<'a, T>
    where
        F: FnMut(Data) -> T,
    {
        OPAQueryInput {
            token: self.token,
            action: self.action.map_data(f),
            transaction_id: self.transaction_id,
        }
    }
}

impl<'a, Data> OPAQueryInputAction<'a, Data> {
    pub fn map_data<T, F>(self, f: F) -> OPAQueryInputAction<'a, T>
    where
        F: FnMut(Data) -> T,
    {
        match self {
            Self::Create {
                service,
                entity,
                records,
            } => OPAQueryInputAction::Create {
                service,
                entity,
                records: records.into_iter().map(f).collect(),
            },
            Self::Delete { service, entity, ids } => OPAQueryInputAction::Delete { service, entity, ids },
            Self::Read { service, entity, ids } => OPAQueryInputAction::Read { service, entity, ids },
            Self::Update {
                service,
                entity,
                patches,
            } => OPAQueryInputAction::Update {
                service,
                entity,
                patches: patches.into_iter().map(f).collect(),
            },
        }
    }
}
