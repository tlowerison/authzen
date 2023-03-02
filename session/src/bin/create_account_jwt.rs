use anyhow::Error;
use authzen_session::*;
use chrono::{Duration, Utc};
use clap::Parser;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde_json::Value;
use std::borrow::Cow;
use std::path::Path;
use uuid::Uuid;

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None, trailing_var_arg=true)]
struct Args {
    /// jwt claim subject (aka "sub") and `state.account_id`
    #[clap(long, env = "JWT_ACCOUNT_ID")]
    account_id: String,
    /// jwt encryption algorithm
    #[clap(short, long = "alg", env = "JWT_ALGORITHM")]
    algorithm: Algorithm,
    /// duration in minutes
    #[clap(short, long = "dur", env = "JWT_DURATION")]
    duration: u32,
    /// additional fields to add to the jwt claim state
    #[clap(long = "field")]
    fields: Vec<String>,
    /// jwt claim issuer
    #[clap(short, long = "iss", env = "JWT_ISSUER")]
    issuer: String,
    ///
    #[clap(long = "private-key", env = "JWT_PRIVATE_CERTIFICATE")]
    jwt_private_certificate: String,
    #[clap(long = "public-key", env = "JWT_PUBLIC_CERTIFICATE")]
    jwt_public_certificate: String,
}

fn main() -> Result<(), Error> {
    let Args {
        account_id,
        algorithm,
        duration,
        fields: named_fields,
        issuer,
        jwt_private_certificate,
        jwt_public_certificate,
    } = Args::parse();

    let account_id: Value = serde_json::from_str(&account_id)?;

    let jwt_private_certificate: EncodingKey =
        parse_encoding_key(find_and_parse_pem(&jwt_private_certificate, &["PRIVATE KEY"])?); // expects RSA-PKCS1.5 PEM format
    let jwt_public_certificate: DecodingKey =
        parse_decoding_key(find_and_parse_pem(&jwt_public_certificate, &["PUBLIC KEY"])?); // expects RSA-PKCS1.5 PEM format

    let mut fields = serde_json::Map::default();

    for named_field in named_fields {
        let (name, value) = named_field.split_once('=').ok_or_else(|| Error::msg(format!("invalid field arg, must be of format `--field=<key>=<value>` where value is a serialized json value: found {named_field}")))?;
        let value = serde_json::from_str(value).map_err(|err| Error::msg(format!("invalid field arg, must be of format `--field=<key>=<value>` where value is a serialized json value: found {named_field}: {err}")))?;
        fields.insert(name.to_string(), value);
    }

    let claims = AccountSessionClaims::new(
        AccountSessionState {
            account_id,
            fields: Value::Object(fields),
        },
        issuer.clone(),
        Utc::now().naive_utc() + Duration::minutes(duration as i64),
    );

    let encoded = claims.encode(&Header::new(algorithm), &jwt_private_certificate)?;

    let validation = {
        let mut validation = Validation::new(algorithm);
        validation.set_issuer(&[&issuer]);
        validation.set_required_spec_claims(&["exp", "iss"]);
        validation
    };

    // ensure that session can be decoded
    <Session<AccountSessionToken<()>> as RawSession<AccountSession<Value, Value>>>::try_decode(
        Session {
            session_id: Uuid::new_v4(),
            created_at: Utc::now().naive_utc(),
            value: AccountSessionToken {
                token: encoded.token.clone(),
                claims: (),
            },
            max_age: None,
            expires: None,
        },
        &jwt_public_certificate,
        &validation,
    )?;

    println!("{}", encoded.token);

    Ok(())
}

fn find_and_parse_pem<'a>(arg: &'a str, searches: &[&str]) -> Result<Cow<'a, str>, Error> {
    let mut all_searches_match = true;
    for search in searches {
        if !arg.contains(search) {
            all_searches_match = false;
            break;
        }
    }
    if all_searches_match {
        return Ok(Cow::Borrowed(arg));
    }

    let path = Path::new(arg);
    if !path.try_exists()? {
        return Err(Error::msg("private key could not be found"));
    }

    Ok(Cow::Owned(String::from_utf8(std::fs::read(path)?)?))
}
