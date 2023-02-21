use authzen::decision_makers::opa::OPAClient;
use authzen::storage_backends::diesel::*;
use uuid::Uuid;

#[derive(authzen::Context, Clone, Copy, Debug, Db)]
pub struct Context<D, C> {
    #[subject]
    pub account_id: Uuid,
    #[db]
    #[storage_client]
    pub db: D,
    #[decision_maker]
    pub opa_client: C,
}

pub type Ctx<'a, D> = Context<D, &'a OPAClient>;
