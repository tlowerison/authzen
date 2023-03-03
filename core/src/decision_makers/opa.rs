use crate::{ActionType, DecisionMaker, Event, ObjectType};
use ::authzen_opa::OPAClient;
use ::authzen_service_util::*;
use ::hyper::{body::Bytes, http::header::*, Body, Method};
use ::serde::{Deserialize, Deserializer, Serialize};
use ::serde_json::Value;
use ::std::fmt::Debug;
use ::typed_builder::TypedBuilder;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct OPAEvent<E, TransactionId> {
    #[serde(flatten)]
    event: E,
    transaction_id: Option<TransactionId>,
}

#[async_trait]
impl<Subject, Action, Object, Input, Context, TransactionId>
    DecisionMaker<Subject, Action, Object, Input, Context, TransactionId> for OPAClient
where
    Event<Subject, Action, Object, Input, Context>: Send + Sync,
    Subject: Debug + Send + Serialize + Sync,
    Action: ?Sized + ActionType + Send + Sync,
    Object: ?Sized + ObjectType + Send + Sync,
    Input: Debug + Serialize + Send + Sync,
    Context: Debug + Send + Serialize + Sync,
    TransactionId: Debug + Send + Serialize + Sync,
{
    type Ok = ();
    type Error = authzen_service_util::Error;

    async fn can_act(
        &self,
        subject: Subject,
        input: &Input,
        context: Context,
        transaction_id: Option<TransactionId>,
    ) -> Result<Self::Ok, Self::Error>
    where
        Subject: 'async_trait,
        Action: 'async_trait,
        Object: 'async_trait,
        Input: 'async_trait,
        Context: 'async_trait,
        TransactionId: 'async_trait,
    {
        let explain = std::env::var("OPA_EXPLAIN").ok();
        let result: OPAQueryResult = OPAQuery {
            config: OPAQueryConfig::builder()
                .data_path(&*self.data_path)
                .query(&*self.query)
                .pretty(
                    std::env::var("OPA_PRETTY")
                        .ok()
                        .and_then(|x| x.parse::<bool>().ok())
                        .unwrap_or_default(),
                )
                .explain(explain.as_deref())
                .build(),
            data: None,
            input: OPAEvent {
                event: Event {
                    action: std::marker::PhantomData::<Action>,
                    object: std::marker::PhantomData::<Object>,
                    subject,
                    input,
                    context,
                },
                transaction_id,
            },
        }
        .query(self)
        .await?;
        if result == false {
            return Err(authzen_service_util::Error::bad_request());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, TypedBuilder)]
#[builder(field_defaults(setter(into)))]
#[skip_serializing_none]
pub struct OPAQuery<'a, Input> {
    #[serde(borrow, skip_serializing)]
    pub config: OPAQueryConfig<'a>,
    #[builder(default)]
    pub data: Option<Value>,
    pub input: Input,
}

/// You will likely have fixed values for `data_path` and `query` which
/// are used across all queries (e.g. `data_path` == `"app"` and `query`
/// == `"authz"`). This struct is a good candidate for a newtype struct
/// which provides those default values as part of an implementation
/// of Default for the wrapper struct.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, TypedBuilder)]
#[builder(field_defaults(default, setter(into)))]
#[skip_serializing_none]
pub struct OPAQueryConfig<'a> {
    #[builder(!default)]
    #[serde(skip_serializing)]
    pub data_path: &'a str,
    #[builder(!default)]
    #[serde(skip_serializing)]
    pub query: &'a str,
    pub explain: Option<&'a str>,
    pub pretty: Option<bool>,
    pub instrument: Option<bool>,
    pub metrics: Option<bool>,
}

impl<Input: Serialize> Endpoint for OPAQuery<'_, Input> {
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
