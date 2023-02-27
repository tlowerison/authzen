use authzen::service_util::{env, parse_allowed_origin};
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
    MONGODB_ARGS: Option<String>,
    MONGODB_COLLECTION: String,
    MONGODB_DATABASE: String,
    MONGODB_HOST: String,
    MONGODB_PASSWORD: Option<String>,
    MONGODB_PORT: Option<u16>,
    MONGODB_SCHEME: String,
    MONGODB_USERNAME: Option<String>,
}
