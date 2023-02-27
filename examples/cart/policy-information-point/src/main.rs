use ::authzen::transaction_caches::mongodb::{mongodb_client, MongodbConfig};
use ::std::net::SocketAddr;
use ::uuid::Uuid;
use cart_policy_information_point::{env, Clients, Ctx, Request};

authzen::server!(::<Request, Ctx, Uuid>(
    SocketAddr::new(env::ip_addr()?, env::port()?),
    Clients {
        db: cart_app::db().await?,
        tx_cache_client: mongodb_client(MongodbConfig {
            scheme: env::mongodb_scheme()?,
            username: env::mongodb_username()?,
            password: env::mongodb_password()?,
            host: env::mongodb_host()?,
            port: env::mongodb_port()?,
            args: env::mongodb_args()?,
            database: env::mongodb_database()?,
            collection: env::mongodb_collection()?,
        }).await?.1,
    },
    authzen::policy_information_point::ServerConfig::builder()
        .allow_origin(::tower_http::cors::AllowOrigin::any())
        .timeout_duration(::std::time::Duration::from_secs(15))
        .build(),
));
