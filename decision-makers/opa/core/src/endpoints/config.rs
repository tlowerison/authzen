use hyper::Method;
use service_util::*;

#[derive(Clone, Debug, Default)]
pub struct GetConfig {
    pub pretty: Option<bool>,
}

impl GetConfig {
    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = Some(pretty);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GetConfigParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pretty: &'a Option<bool>,
}

impl<'a> From<&'a GetConfig> for GetConfigParams<'a> {
    fn from(partially_evaluate_query: &'a GetConfig) -> Self {
        Self {
            pretty: &partially_evaluate_query.pretty,
        }
    }
}

impl Endpoint for GetConfig {
    const METHOD: Method = Method::GET;

    type Params<'a> = GetConfigParams<'a>;

    fn path(&self) -> Path {
        "/v1/config".into()
    }
    fn params(&self) -> Self::Params<'_> {
        self.into()
    }
}
