use authzen_service_util::*;
use hyper::Method;

#[derive(Clone, Debug, Default)]
pub struct GetStatus {
    pub pretty: Option<bool>,
}

impl GetStatus {
    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = Some(pretty);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GetStatusParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    pretty: &'a Option<bool>,
}

impl<'a> From<&'a GetStatus> for GetStatusParams<'a> {
    fn from(partially_evaluate_query: &'a GetStatus) -> Self {
        Self {
            pretty: &partially_evaluate_query.pretty,
        }
    }
}

impl Endpoint for GetStatus {
    const METHOD: Method = Method::GET;

    type Params<'a> = GetStatusParams<'a>;

    fn path(&self) -> Path {
        "/v1/status".into()
    }
    fn params(&self) -> Self::Params<'_> {
        self.into()
    }
}
