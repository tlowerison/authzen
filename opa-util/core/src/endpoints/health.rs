use hyper::Method;
use service_util::*;

#[derive(Clone, Debug, Default)]
pub struct Health {
    pub bundles: Option<bool>,
    pub plugins: Option<bool>,
}

impl Health {
    pub fn bundles(mut self, bundles: bool) -> Self {
        self.bundles = Some(bundles);
        self
    }
    pub fn plugins(mut self, plugins: bool) -> Self {
        self.plugins = Some(plugins);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct HealthParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    bundles: &'a Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plugins: &'a Option<bool>,
}

impl<'a> From<&'a Health> for HealthParams<'a> {
    fn from(partially_evaluate_query: &'a Health) -> Self {
        Self {
            bundles: &partially_evaluate_query.bundles,
            plugins: &partially_evaluate_query.plugins,
        }
    }
}

impl Endpoint for Health {
    const METHOD: Method = Method::GET;

    type Params<'a> = HealthParams<'a>;

    fn path(&self) -> Path {
        "/health".into()
    }
    fn params(&self) -> Self::Params<'_> {
        self.into()
    }
}
