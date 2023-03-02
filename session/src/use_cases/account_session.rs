use crate::{
    get_session_id_from_request, DynSessionStore, RawSession, RequestSession, Session, SessionStore, SessionValue,
};
use anyhow::Error;
use chrono::{Duration, NaiveDateTime, Utc};
use derivative::Derivative;
use derive_more::*;
use http::Request;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{de::DeserializeOwned, Serialize};
use serde_with::skip_serializing_none;
use std::borrow::{Borrow, Cow};
use std::ops::Deref;
use typed_builder::TypedBuilder;
use uuid::Uuid;

pub const HTTP_ACCOUNT_SESSION_JWT_HEADER: &str = "x-account-session-jwt";

pub trait AccountSessionStore: SessionStore<Value = AccountSessionToken<()>> {}

impl<T: SessionStore<Value = AccountSessionToken<()>>> AccountSessionStore for T {}

pub type DynAccountSessionStore = DynSessionStore<AccountSessionToken<()>>;
pub type AccountSession<AccountId, Fields> = Session<AccountSessionToken<AccountSessionClaims<AccountId, Fields>>>;

/// wrapper struct for the session account id
/// useful for cases where all you want is the
/// account id and either do not want to or are
/// unable to specify the additional fields
/// included in the jwt claim
#[derive(AsRef, AsMut, Clone, Derivative, Deserialize, Deref, DerefMut, Display, Eq, From, PartialEq, Serialize)]
#[derivative(Debug = "transparent")]
pub struct AccountSessionSubject<AccountId>(pub AccountId);

impl<AccountId, Fields> RawSession<AccountSession<AccountId, Fields>> for Session<AccountSessionToken<()>>
where
    AccountId: Clone + std::fmt::Debug + DeserializeOwned + Send + Sync + 'static,
    Fields: DeserializeOwned + std::fmt::Debug + Send + Sync + 'static,
{
    type Key = DecodingKey;
    type Validation = Validation;
    fn try_decode(
        self,
        key: &Self::Key,
        validation: &Self::Validation,
    ) -> Result<AccountSession<AccountId, Fields>, anyhow::Error> {
        self.try_map(|value| {
            let token_data = decode::<AccountSessionClaims<AccountId, Fields>>(&value.token, key, validation)?;
            Ok(AccountSessionToken {
                token: value.token,
                claims: token_data.claims,
            })
        })
    }
    fn add_extensions(
        session: Result<Option<Self>, anyhow::Error>,
        key: &Self::Key,
        validation: &Self::Validation,
        extensions: &mut http::Extensions,
    ) {
        let parsed_session: Option<AccountSession<AccountId, Fields>> = match session {
            Ok(Some(session)) => {
                extensions.insert(Some(session.clone()));
                session.try_decode(key, validation).ok()
            }
            _ => None,
        };
        match parsed_session {
            Some(parsed_session) => {
                extensions.insert(Some(AccountSessionSubject(parsed_session.account_id.clone())));
                extensions.insert(Some(parsed_session));
            }
            _ => {
                extensions.insert(None::<AccountSessionSubject<AccountId>>);
                extensions.insert(None::<AccountSession<AccountId, Fields>>);
                extensions.insert(None::<Session<AccountSessionToken<()>>>);
            }
        }
    }
}

#[async_trait]
impl<ReqBody: Sync, S> SessionValue<ReqBody, S> for AccountSessionToken<()>
where
    S: SessionStore<Value = Self>,
{
    fn get_unparsed_request_session(store: &S, req: &Request<ReqBody>) -> Result<RequestSession<S::Value>, Error> {
        if let Some(service_account_jwt) = req.headers().get(HTTP_ACCOUNT_SESSION_JWT_HEADER) {
            return Ok(RequestSession::Session(Session {
                session_id: Uuid::new_v4(),
                created_at: Utc::now().naive_utc(),
                value: AccountSessionToken {
                    token: service_account_jwt.to_str()?.into(),
                    claims: (),
                },
                max_age: None,
                expires: None,
            }));
        }

        match get_session_id_from_request(store, req) {
            Some(session_id) => Ok(RequestSession::SessionId(session_id)),
            None => Ok(RequestSession::None),
        }
    }
}

#[derive(Clone, Derivative, Deserialize, Eq, PartialEq, Serialize)]
#[derivative(Debug)]
pub struct AccountSessionToken<Claims: private::AccountSessionClaimsTrait> {
    #[derivative(Debug = "ignore")]
    pub token: String,
    pub claims: Claims,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TypedBuilder)]
#[serde(rename_all = "camelCase")]
pub struct AccountSessionClaims<AccountId, Fields = ()> {
    /// Audience
    #[builder(default, setter(into, strip_option))]
    pub aud: Option<Cow<'static, str>>,
    /// Expiration time (as UTC seconds timestamp, validate_exp defaults to true in validation)
    #[builder(setter(into))]
    pub exp: u64,
    /// Issued at (as UTC seconds timestamp)
    #[builder(setter(into, strip_option))]
    pub iat: Option<u64>,
    /// Issuer
    #[builder(setter(into))]
    pub iss: Cow<'static, str>,
    /// Not Before (as UTC seconds timestamp)
    #[builder(setter(into, strip_option))]
    pub nbf: Option<u64>,
    /// Session state
    pub state: AccountSessionState<AccountId, Fields>,
    /// Subject (whom token refers to)
    pub sub: AccountId,
}

#[skip_serializing_none]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, TypedBuilder)]
pub struct AccountSessionState<AccountId, Fields = ()> {
    pub account_id: AccountId,
    #[serde(flatten)]
    pub fields: Fields,
}

impl<AccountId, Fields> Session<AccountSessionToken<AccountSessionClaims<AccountId, Fields>>> {
    pub fn account_id(&self) -> &AccountId {
        &self.value.claims.state.account_id
    }
    pub fn fields(&self) -> &Fields {
        &self.value.claims.state.fields
    }
}

impl<AccountId: Clone, Fields> AccountSessionClaims<AccountId, Fields> {
    pub fn new(
        state: AccountSessionState<AccountId, Fields>,
        iss: impl Into<Cow<'static, str>>,
        exp: NaiveDateTime,
    ) -> Self {
        Self {
            aud: None,
            exp: exp.timestamp() as u64,
            iat: Some(Utc::now().naive_utc().timestamp() as u64),
            iss: iss.into(),
            nbf: None,
            sub: state.account_id.clone(),
            state,
        }
    }
    pub fn new_exp_in(
        state: AccountSessionState<AccountId, Fields>,
        iss: impl Into<Cow<'static, str>>,
        exp_in: Duration,
    ) -> Self {
        let now = Utc::now().naive_utc();
        Self {
            aud: None,
            exp: (now + exp_in).timestamp() as u64,
            iat: Some(now.timestamp() as u64),
            iss: iss.into(),
            nbf: None,
            sub: state.account_id.clone(),
            state,
        }
    }
}

impl<AccountId, Fields> AccountSessionClaims<AccountId, Fields> {
    pub fn aud(mut self, aud: impl Into<Cow<'static, str>>) -> Self {
        self.aud = Some(aud.into());
        self
    }
    pub fn exp(mut self, exp: NaiveDateTime) -> Self {
        self.exp = exp.timestamp() as u64;
        self
    }
    pub fn exp_in(mut self, exp_in: Duration) -> Self {
        self.exp = (Utc::now().naive_utc() + exp_in).timestamp() as u64;
        self
    }
    pub fn iat(mut self, iat: NaiveDateTime) -> Self {
        self.iat = Some(iat.timestamp() as u64);
        self
    }
    pub fn iss(mut self, iss: impl Into<Cow<'static, str>>) -> Self {
        self.iss = iss.into();
        self
    }
    pub fn nbf(mut self, nbf: NaiveDateTime) -> Self {
        self.nbf = Some(nbf.timestamp() as u64);
        self
    }
    pub fn nbf_in(mut self, nbf_in: Duration) -> Self {
        self.nbf = Some((Utc::now().naive_utc() + nbf_in).timestamp() as u64);
        self
    }
    pub fn state<NewAccountId: Clone, NewFields>(
        self,
        state: AccountSessionState<NewAccountId, NewFields>,
    ) -> AccountSessionClaims<NewAccountId, NewFields> {
        AccountSessionClaims {
            aud: self.aud,
            exp: self.exp,
            iat: self.iat,
            iss: self.iss,
            nbf: self.nbf,
            sub: state.account_id.clone(),
            state,
        }
    }
}

impl<AccountId: Serialize, Fields: Serialize> AccountSessionClaims<AccountId, Fields> {
    pub fn encode(self, header: &Header, encoding_key: &EncodingKey) -> Result<AccountSessionToken<()>, anyhow::Error> {
        Ok(AccountSessionToken {
            token: encode(header, &self, encoding_key)?,
            claims: (),
        })
    }
}

impl<Claims: private::AccountSessionClaimsTrait> Deref for AccountSessionToken<Claims> {
    type Target = Claims;
    fn deref(&self) -> &Self::Target {
        &self.claims
    }
}

impl<AccountId, Fields> Deref for AccountSessionClaims<AccountId, Fields> {
    type Target = AccountSessionState<AccountId, Fields>;
    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<AccountId, Fields> Deref for AccountSessionState<AccountId, Fields> {
    type Target = AccountId;
    fn deref(&self) -> &Self::Target {
        &self.account_id
    }
}

mod private {
    use super::*;

    pub trait AccountSessionClaimsTrait {}
    impl AccountSessionClaimsTrait for () {}
    impl<AccountId, Fields> AccountSessionClaimsTrait for AccountSessionClaims<AccountId, Fields> {}
}

pub trait BorrowAccountSession<AccountId, Fields> {
    fn borrow_account_session(&self) -> Option<&AccountSession<AccountId, Fields>>;
}

impl<AccountId, Fields, T: Borrow<AccountSession<AccountId, Fields>>> BorrowAccountSession<AccountId, Fields> for T {
    fn borrow_account_session(&self) -> Option<&AccountSession<AccountId, Fields>> {
        Some(self.borrow())
    }
}

impl<AccountId, Fields, T: Borrow<AccountSession<AccountId, Fields>>> BorrowAccountSession<AccountId, Fields>
    for Option<T>
{
    fn borrow_account_session(&self) -> Option<&AccountSession<AccountId, Fields>> {
        self.as_ref().map(|x| x.borrow())
    }
}

impl<AccountId, Fields, T: Borrow<AccountSession<AccountId, Fields>>> BorrowAccountSession<AccountId, Fields>
    for &Option<T>
{
    fn borrow_account_session(&self) -> Option<&AccountSession<AccountId, Fields>> {
        self.as_ref().map(|x| x.borrow())
    }
}
