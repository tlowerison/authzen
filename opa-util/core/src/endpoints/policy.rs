use hyper::{http::header::*, Body, Method};
use service_util::*;

#[derive(Clone, Debug)]
pub struct ListPolicies;

impl Endpoint for ListPolicies {
    const METHOD: Method = Method::GET;

    fn path(&self) -> Path {
        "/v1/policies".into()
    }
    fn params(&self) {}
}

#[derive(Clone, Debug)]
pub struct GetPolicy {
    pub id: String,
    pub pretty: Option<bool>,
}

impl GetPolicy {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            pretty: None,
        }
    }

    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = Some(pretty);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GetPolicyParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pretty: &'a Option<bool>,
}

impl Endpoint for GetPolicy {
    const METHOD: Method = Method::GET;

    type Params<'a> = GetPolicyParams<'a>;

    fn path(&self) -> Path {
        format!("/v1/policies/{}", self.id).into()
    }
    fn params(&self) -> Self::Params<'_> {
        Self::Params { pretty: &self.pretty }
    }
}

#[derive(Clone, Debug)]
pub struct UpsertPolicy {
    pub id: String,
    pub policy: String,
    pub pretty: Option<bool>,
    pub metrics: Option<bool>,
}

impl UpsertPolicy {
    pub fn new(id: impl Into<String>, policy: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            policy: policy.into(),
            pretty: None,
            metrics: None,
        }
    }

    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = Some(pretty);
        self
    }

    pub fn metrics(mut self, metrics: bool) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct UpsertPolicyParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pretty: &'a Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: &'a Option<bool>,
}

impl Endpoint for UpsertPolicy {
    const METHOD: Method = Method::PUT;

    type Params<'a> = UpsertPolicyParams<'a>;

    fn path(&self) -> Path {
        format!("/v1/policies/{}", self.id).into()
    }
    fn params(&self) -> Self::Params<'_> {
        Self::Params {
            pretty: &self.pretty,
            metrics: &self.metrics,
        }
    }
    fn headers(&self) -> HeaderMap {
        HeaderMap::from_iter(vec![(CONTENT_TYPE, HeaderValue::from_str("text/plain").unwrap())])
    }
    fn body(&self) -> Body {
        Body::from(self.policy.as_bytes().to_owned())
    }
}

#[derive(Clone, Debug)]
pub struct DeletePolicy {
    pub id: String,
}

impl DeletePolicy {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

impl Endpoint for DeletePolicy {
    const METHOD: Method = Method::DELETE;

    fn path(&self) -> Path {
        format!("/v1/policies/{}", self.id).into()
    }
    fn params(&self) {}
}
