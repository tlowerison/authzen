use diesel_util::*;
use uuid::Uuid;

#[derive(authzen::Context, Clone, Copy, Debug, Db)]
pub struct Context<D> {
    #[subject]
    pub account_id: Uuid,
    #[db]
    pub db: D,
}
