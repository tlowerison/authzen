use authzen_service_util::*;
use hyper::{http::header::*, Method};
use serde_json::Value;

#[derive(Clone, Debug)]
pub struct GetDocument {
    pub path: String,
    pub input: Option<Value>,
    pub pretty: Option<bool>,
    pub provenance: Option<bool>,
    pub explain: Option<bool>,
    pub metrics: Option<bool>,
    pub instrument: Option<bool>,
    pub strict_builtin_errors: Option<bool>,
}

impl GetDocument {
    pub fn new<E>(path: impl TryInto<std::path::PathBuf, Error = E>) -> Result<Self, anyhow::Error>
    where
        anyhow::Error: From<E>,
    {
        let path = path.try_into()?;
        if path.is_absolute() {
            return Err(anyhow::Error::msg("cannot use an absolute path"));
        }
        let path = path
            .components()
            .map(|component| match component {
                std::path::Component::Normal(os_str) => Ok(os_str.to_str().ok_or_else(|| {
                    anyhow::Error::msg(format!(
                        "could not construct path to submit to request: invalid OsStr path component: {os_str:?}"
                    ))
                })?),
                _ => Err(anyhow::Error::msg(
                    "path components can only consist of normal text (i.e. no platform specific path info)",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        Ok(Self {
            input: None,
            pretty: None,
            provenance: None,
            explain: None,
            metrics: None,
            instrument: None,
            strict_builtin_errors: None,
            path,
        })
    }

    pub fn input(mut self, input: Value) -> Self {
        self.input = Some(input);
        self
    }
    pub fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = Some(pretty);
        self
    }
    pub fn provenance(mut self, provenance: bool) -> Self {
        self.provenance = Some(provenance);
        self
    }
    pub fn explain(mut self, explain: bool) -> Self {
        self.explain = Some(explain);
        self
    }
    pub fn metrics(mut self, metrics: bool) -> Self {
        self.metrics = Some(metrics);
        self
    }
    pub fn instrument(mut self, instrument: bool) -> Self {
        self.instrument = Some(instrument);
        self
    }
    pub fn strict_builtin_errors(mut self, strict_builtin_errors: bool) -> Self {
        self.strict_builtin_errors = Some(strict_builtin_errors);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct GetDocumentParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    input: &'a Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pretty: &'a Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provenance: &'a Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    explain: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: &'a Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instrument: &'a Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strict_builtin_errors: &'a Option<bool>,
}

impl<'a> From<&'a GetDocument> for GetDocumentParams<'a> {
    fn from(get_document: &'a GetDocument) -> Self {
        Self {
            input: &get_document.input,
            pretty: &get_document.pretty,
            provenance: &get_document.provenance,
            explain: get_document
                .explain
                .and_then(|explain| if explain { Some("full") } else { None }),
            metrics: &get_document.metrics,
            instrument: &get_document.instrument,
            strict_builtin_errors: &get_document.strict_builtin_errors,
        }
    }
}

impl Endpoint for GetDocument {
    const METHOD: Method = Method::GET;

    type Params<'a> = GetDocumentParams<'a>;

    fn path(&self) -> Path {
        format!("/v1/data/{}", self.path).into()
    }
    fn params(&self) -> Self::Params<'_> {
        self.into()
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct GetDocumentResult<T> {
    pub result: T,
    pub metrics: Option<Value>,
    pub decision_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct UpsertDocument {
    pub path: String,
    pub input: Value,
    pub metrics: Option<bool>,
    pub should_not_overwrite: Option<bool>,
}

impl UpsertDocument {
    pub fn new<E>(path: impl TryInto<std::path::PathBuf, Error = E>, input: Value) -> Result<Self, anyhow::Error>
    where
        anyhow::Error: From<E>,
    {
        let path = path.try_into()?;
        if path.is_absolute() {
            return Err(anyhow::Error::msg("cannot use an absolute path"));
        }
        let path = path
            .components()
            .map(|component| match component {
                std::path::Component::Normal(os_str) => Ok(os_str.to_str().ok_or_else(|| {
                    anyhow::Error::msg(format!(
                        "could not construct path to submit to request: invalid OsStr path component: {os_str:?}"
                    ))
                })?),
                _ => Err(anyhow::Error::msg(
                    "path components can only consist of normal text (i.e. no platform specific path info)",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        Ok(Self {
            metrics: None,
            should_not_overwrite: None,
            path,
            input,
        })
    }
    pub fn metrics(mut self, metrics: bool) -> Self {
        self.metrics = Some(metrics);
        self
    }
    pub fn should_not_overwrite(mut self, should_not_overwrite: bool) -> Self {
        self.should_not_overwrite = Some(should_not_overwrite);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct UpsertDocumentParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: &'a Option<bool>,
}

impl<'a> From<&'a UpsertDocument> for UpsertDocumentParams<'a> {
    fn from(upsert_document: &'a UpsertDocument) -> Self {
        Self {
            metrics: &upsert_document.metrics,
        }
    }
}

impl Endpoint for UpsertDocument {
    const METHOD: Method = Method::PUT;

    type Params<'a> = UpsertDocumentParams<'a>;

    fn path(&self) -> Path {
        format!("/v1/data/{}", self.path).into()
    }
    fn params(&self) -> Self::Params<'_> {
        self.into()
    }
    fn headers(&self) -> HeaderMap {
        if let Some(true) = &self.should_not_overwrite {
            HeaderMap::from_iter(vec![
                (CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap()),
                (IF_NONE_MATCH, HeaderValue::from_str("*").unwrap()),
            ])
        } else {
            HeaderMap::from_iter(vec![(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap())])
        }
    }
}

/// uses [json patch format](https://tools.ietf.org/html/rfc6902)
#[derive(Clone, Debug)]
pub struct PatchDocument {
    pub path: String,
    pub inputs: Vec<Value>,
    pub should_not_overwrite: Option<bool>,
}

impl PatchDocument {
    pub fn new<E>(
        path: impl TryInto<std::path::PathBuf, Error = E>,
        inputs: impl IntoIterator<Item = Value>,
    ) -> Result<Self, anyhow::Error>
    where
        anyhow::Error: From<E>,
    {
        let path = path.try_into()?;
        if path.is_absolute() {
            return Err(anyhow::Error::msg("cannot use an absolute path"));
        }
        let path = path
            .components()
            .map(|component| match component {
                std::path::Component::Normal(os_str) => Ok(os_str.to_str().ok_or_else(|| {
                    anyhow::Error::msg(format!(
                        "could not construct path to submit to request: invalid OsStr path component: {os_str:?}"
                    ))
                })?),
                _ => Err(anyhow::Error::msg(
                    "path components can only consist of normal text (i.e. no platform specific path info)",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        Ok(Self {
            inputs: inputs.into_iter().collect(),
            should_not_overwrite: None,
            path,
        })
    }
    pub fn should_not_overwrite(mut self, should_not_overwrite: bool) -> Self {
        self.should_not_overwrite = Some(should_not_overwrite);
        self
    }
}

impl Endpoint for PatchDocument {
    const METHOD: Method = Method::PATCH;

    fn path(&self) -> Path {
        format!("/v1/data/{}", self.path).into()
    }
    fn params(&self) {}
    fn headers(&self) -> HeaderMap {
        if let Some(true) = &self.should_not_overwrite {
            HeaderMap::from_iter(vec![
                (CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap()),
                (IF_NONE_MATCH, HeaderValue::from_str("*").unwrap()),
            ])
        } else {
            HeaderMap::from_iter(vec![(CONTENT_TYPE, HeaderValue::from_str("application/json").unwrap())])
        }
    }
}

#[derive(Clone, Debug)]
pub struct DeleteDocument {
    pub path: String,
    pub metrics: Option<bool>,
}

impl DeleteDocument {
    pub fn new<E>(path: impl TryInto<std::path::PathBuf, Error = E>) -> Result<Self, anyhow::Error>
    where
        anyhow::Error: From<E>,
    {
        let path = path.try_into()?;
        if path.is_absolute() {
            return Err(anyhow::Error::msg("cannot use an absolute path"));
        }
        let path = path
            .components()
            .map(|component| match component {
                std::path::Component::Normal(os_str) => Ok(os_str.to_str().ok_or_else(|| {
                    anyhow::Error::msg(format!(
                        "could not construct path to submit to request: invalid OsStr path component: {os_str:?}"
                    ))
                })?),
                _ => Err(anyhow::Error::msg(
                    "path components can only consist of normal text (i.e. no platform specific path info)",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?
            .join("/");
        Ok(Self { metrics: None, path })
    }
    pub fn metrics(mut self, metrics: bool) -> Self {
        self.metrics = Some(metrics);
        self
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DeleteDocumentParams<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    metrics: &'a Option<bool>,
}

impl<'a> From<&'a DeleteDocument> for DeleteDocumentParams<'a> {
    fn from(delete_document: &'a DeleteDocument) -> Self {
        Self {
            metrics: &delete_document.metrics,
        }
    }
}

impl Endpoint for DeleteDocument {
    const METHOD: Method = Method::DELETE;

    type Params<'a> = DeleteDocumentParams<'a>;

    fn path(&self) -> Path {
        format!("/v1/data/{}", self.path).into()
    }
    fn params(&self) -> Self::Params<'_> {
        self.into()
    }
}
