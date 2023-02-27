use authzen::service_util::{env, parse_allowed_origin};
use jsonwebtoken::{DecodingKey, EncodingKey};
use session_util::{parse_decoding_key, parse_encoding_key};
use std::net::IpAddr;

env! {
    ALLOW_CREDENTIALS: bool = true,
    ALLOWED_ORIGIN: tower_http::cors::AllowOrigin | parse_allowed_origin,
    AUTH_TOKEN: String,
    BASE_URL: String,
    CONCURRENCY_LIMIT: usize = 250usize,
    DEFAULT_OAUTH_REDIRECT_URI: String,
    IP_ADDR: IpAddr,
    PORT: u16,
    REQUEST_TIMEOUT_IN_SECS: u64 = 15u64,
}
env! {
    GRAPHQL_INCLUDE_PLAYGROUND: bool = false,
    GRAPHQL_LIMIT_COMPLEXITY: usize = 1000usize,
    GRAPHQL_LIMIT_DEPTH: usize = 5usize,
    GRAPHQL_LIMIT_RECURSIVE_DEPTH: usize = 5usize,
    SHOULD_GRAPHQL_PLAYGROUND_REQUIRE_AUTHN: bool = true,
}
env! {
    MONGODB_ARGS: Option<String>,
    MONGODB_COLLECTION: String,
    MONGODB_DATABASE: String,
    MONGODB_HOST: String,
    MONGODB_PASSWORD: Option<String>,
    MONGODB_PORT: Option<u16>,
    MONGODB_SCHEME: String,
    MONGODB_USERNAME: Option<String>,
}
env! {
    OPA_SCHEME: String = "http",
    OPA_HOST: String,
    OPA_PORT: Option<u16>,
    OPA_DATA_PATH: String,
    OPA_QUERY: String,
}
env! {
    OTEL_ENABLED: bool = true,
}
env! {
    POSTGRES_ARGS: Option<String>,
    POSTGRES_DB: String,
    POSTGRES_HOST: String,
    POSTGRES_MAX_CONNECTIONS: Option<u32>,
    POSTGRES_PASSWORD_MIGRATION: Option<String>,
    POSTGRES_PASSWORD_OPAL: Option<String>,
    POSTGRES_PASSWORD_SERVER: Option<String>,
    POSTGRES_PORT: Option<u16>,
    POSTGRES_SCHEME: String = "postgres",
    POSTGRES_USERNAME_MIGRATION: Option<String>,
    POSTGRES_USERNAME_OPAL: String,
    POSTGRES_USERNAME_SERVER: Option<String>,
}
env! {
    REDIS_HOST: String,
    REDIS_IS_CLUSTER: bool,
    REDIS_PASSWORD: Option<String>,
    REDIS_PORT: Option<u16>,
    REDIS_USERNAME: Option<String>,
}
env! {
    SERVICE_ACCOUNT_JWT: String,
    SESSION_DOMAIN: String,
    SESSION_JWT_PRIVATE_CERTIFICATE: EncodingKey | parse_encoding_key::<String>, // expects RSA-PKCS1.5 PEM format
    SESSION_JWT_PUBLIC_CERTIFICATE: DecodingKey | parse_decoding_key::<String>,  // expects RSA-PKCS1.5 PEM format
    SESSION_MAX_AGE: chrono::Duration | chrono::Duration::minutes,
    SESSION_MAX_AGE_LONG: chrono::Duration | chrono::Duration::minutes,
    SESSION_PATH: String,
    SESSION_SECRET: String,
    SESSION_SECURE: bool,
}
