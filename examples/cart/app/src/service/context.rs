use authzen::decision_makers::opa::OPAClient;
use authzen::storage_backends::diesel::*;
use authzen::transaction_caches::mongodb::MongodbTxCollection;
use uuid::Uuid;

pub type AccountId = Uuid;
pub type AccountSession = session_util::AccountSession<AccountId, ()>;

#[derive(authzen::Context, Clone, Copy, Derivative, Db)]
#[derivative(Debug)]
pub struct Context<D, S, C, M> {
    #[subject]
    pub session: S,
    #[db]
    #[derivative(Debug = "ignore")]
    #[storage_client]
    pub db: D,
    #[decision_maker]
    #[derivative(Debug = "ignore")]
    pub opa_client: C,
    #[transaction_cache]
    #[derivative(Debug = "ignore")]
    pub mongodb_client: M,
}

pub type Ctx<'a, D> = Context<D, &'a AccountSession, &'a OPAClient, &'a MongodbTxCollection>;
pub type CtxOptSession<'a, D> = Context<D, Option<&'a AccountSession>, &'a OPAClient, &'a MongodbTxCollection>;

pub static SESSION_ISSUER: &'static str = "examples_cart";
pub const SESSION_JWT_ALGORITHM: jsonwebtoken::Algorithm = jsonwebtoken::Algorithm::RS512;
lazy_static! {
    pub static ref SESSION_DECODING_KEY: jsonwebtoken::DecodingKey = {
        let jsonwebtoken_public_certificate = std::env::var("JWT_PUBLIC_CERTIFICATE")
            .expect("expected an environment variable JWT_PUBLIC_CERTIFICATE to exist");
        session_util::parse_decoding_key(jsonwebtoken_public_certificate)
    };
    pub static ref SESSION_ENCODING_KEY: jsonwebtoken::EncodingKey = {
        let jsonwebtoken_private_certificate = std::env::var("JWT_PRIVATE_CERTIFICATE")
            .expect("expected an environment variable JWT_PRIVATE_CERTIFICATE to exist");
        session_util::parse_encoding_key(jsonwebtoken_private_certificate)
    };
    pub static ref SESSION_JWT_VALIDATION: jsonwebtoken::Validation = {
        let mut validation = jsonwebtoken::Validation::new(SESSION_JWT_ALGORITHM);
        validation.set_issuer(&[SESSION_ISSUER]);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        validation
    };
    pub static ref SESSION_JWT_HEADER: jsonwebtoken::Header = jsonwebtoken::Header::new(SESSION_JWT_ALGORITHM);
}
