use chrono::{Duration, NaiveDateTime};
use data_encoding::BASE64;
use percent_encoding::{percent_decode, utf8_percent_encode, AsciiSet, CONTROLS};
use ring::hmac::{sign, Key};
use serde::de::{Deserializer, Error};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use uuid::Uuid;

const PERCENT_ENCODING_ASCII_SET: &AsciiSet = &CONTROLS.add(b':').add(b'=');

#[derive(Clone, Debug)]
pub struct CookieValue {
    pub id: Uuid,
    pub signature: String,
}

impl CookieValue {
    pub(crate) fn new(key: &Key) -> Result<Self, anyhow::Error> {
        let id = Uuid::new_v4();
        let signature = BASE64.encode(sign(key, id.as_bytes()).as_ref());
        Ok(Self { id, signature })
    }

    pub(crate) fn encode(&self) -> String {
        serde_plain::to_string(&self).unwrap()
    }
}

impl Serialize for CookieValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = format!(
            "s:{}.{}",
            self.id.as_simple().encode_lower(&mut Uuid::encode_buffer()),
            self.signature
        );
        let encoded = utf8_percent_encode(&formatted, PERCENT_ENCODING_ASCII_SET).to_string();
        serializer.collect_str(&encoded)
    }
}

impl<'de> Deserialize<'de> for CookieValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded_cookie_value = String::deserialize(deserializer)?;
        let cookie_value = percent_decode(encoded_cookie_value.as_bytes())
            .decode_utf8()
            .map_err(|_| D::Error::custom("invalid cookie value"))?;
        if &cookie_value[0..2] != "s:" {
            return Err(D::Error::custom("invalid cookie value"));
        }
        let mut sequence: Vec<_> = cookie_value[2..].split('.').map(|item| item.to_owned()).collect();
        if sequence.len() != 2 {
            return Err(D::Error::custom("invalid cookie value"));
        }
        let signature = sequence.pop().unwrap();
        let id = Uuid::parse_str(&sequence.pop().unwrap()).map_err(|_| D::Error::custom("invalid cookie value"))?;

        Ok(CookieValue { signature, id })
    }
}

#[derive(Clone, Debug)]
pub struct CookieConfig<'a, T: 'a + Clone> {
    pub value: &'a T,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: SameSite,
    pub domain: Option<Cow<'a, str>>,
    pub path: Option<Cow<'a, str>>,
    pub max_age: Option<Duration>,
    pub expires: Option<NaiveDateTime>,
}

impl<'a, T: 'a + Clone + Deserialize<'a> + Serialize> CookieConfig<'a, T> {
    pub fn new(value: &'a T) -> Self {
        Self {
            http_only: true,
            secure: true,
            same_site: SameSite::Lax,
            domain: None,
            path: None,
            max_age: None,
            expires: None,
            value,
        }
    }
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }
    pub fn max_age(mut self, max_age: impl Into<Option<Duration>>) -> Self {
        self.max_age = max_age.into();
        self
    }
    pub fn expires(mut self, expires: impl Into<Option<NaiveDateTime>>) -> Self {
        self.expires = expires.into();
        self
    }
    pub fn domain<S: Into<Cow<'a, str>>>(mut self, domain: impl Into<Option<S>>) -> Self {
        self.domain = domain.into().map(Into::into);
        self
    }
    pub fn path<S: Into<Cow<'a, str>>>(mut self, path: impl Into<Option<S>>) -> Self {
        self.path = path.into().map(Into::into);
        self
    }
}

#[derive(Clone, Debug)]
pub enum SameSite {
    Strict,
    Lax,
    None,
}

impl std::fmt::Display for SameSite {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self::Strict => write!(f, "Strict"),
            Self::Lax => write!(f, "Lax"),
            Self::None => write!(f, "None"),
        }
    }
}
