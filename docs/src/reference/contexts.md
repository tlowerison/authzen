# Contexts
Authzen contexts are data types which hold all of the necessary http clients/connections to the underlying decision makers, data storage backends and transaction caches.
In rust, they are any type which implements [AuthorizationContext](https://docs.rs/authzen/latest/authzen/trait.AuthorizationContext.html).
`AuthorizationContext` can be derived on structs like so:
```rust
#[derive(Clone, Copy, authzen::Context, authzen::storage_backends::diesel::Db)]
pub struct Context<D> {
    #[subject]
    pub session: uuid::Uuid,
    #[db]
    #[storage_client]
    pub db: D,
    #[decision_maker]
    pub opa_client: authzen::decision_makers::opa::OPAClient,
    #[transaction_cache]
    pub mongodb_client: authzen::transaction_caches::mongodb::MongodbTxCollection,
}
```
or if you want to do so in a generic way you could define context like this
```rust
#[derive(Clone, Copy, Context, Derivative, Db)]
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
```
