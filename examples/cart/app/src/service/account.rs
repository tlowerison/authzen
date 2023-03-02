use crate::*;
use authzen::service_util::Error;
use authzen::session::{AccountSessionClaims, AccountSessionState, CookieConfig, DynAccountSessionStore};
use authzen::storage_backends::diesel::prelude::*;
use http::response::Parts;
use hyper::{Body, Response};
use serde::{de::DeserializeOwned, Serialize};

#[instrument(skip(session_store))]
pub async fn sign_up<D: Db>(
    ctx: &CtxOptSession<'_, D>,
    session_store: &DynAccountSessionStore,
    identifier: crate::db::Identifier,
) -> Result<(Parts, DbAccount), Error> {
    let db_account = DbAccount::insert_one(ctx, DbAccount::builder().identifier(identifier).build()).await?;

    let token = AccountSessionClaims::new_exp_in(
        AccountSessionState {
            account_id: db_account.id,
            fields: (),
        },
        SESSION_ISSUER,
        env::session_max_age()?,
    )
    .encode(&SESSION_JWT_HEADER, &env::session_jwt_private_certificate()?)?;

    let mut response = Response::new(Body::empty());
    session_store
        .store_session_and_set_cookie(
            &mut response,
            cookie_config(&token)?,
            Some(format!("{}", db_account.id)),
        )
        .await?;

    Ok((response.into_parts().0, db_account))
}

pub fn cookie_config<'a, T: 'a + Clone + DeserializeOwned + Serialize>(
    value: &'a T,
) -> Result<CookieConfig<'a, T>, Error> {
    Ok(CookieConfig::new(value)
        .domain(env::session_domain()?)
        .path(env::session_path()?)
        .secure(env::session_secure()?)
        .max_age(env::session_max_age()?))
}
