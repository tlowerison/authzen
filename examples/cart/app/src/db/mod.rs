use crate::env;
use authzen::data_sources::{diesel::connection::Db as _Db, TransactionalDataSource};
use diesel::pg::Pg;
use diesel_async::{pooled_connection as pc, AsyncPgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

pub mod models;
pub mod schema;

pub use models::*;

// trait alias workaround
pub trait Db: _Db<AsyncConnection = AsyncPgConnection, Backend = Pg> {}

impl<D: _Db<AsyncConnection = AsyncPgConnection, Backend = Pg>> Db for D {}

pub type DbPool = authzen::data_sources::diesel::pool::Pool<AsyncPgConnection>;

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

pub async fn db() -> Result<DbPool, anyhow::Error> {
    tokio::task::spawn_blocking(move || {
        info!("connecting to database for migrations");
        let mut conn: diesel::PgConnection = diesel::Connection::establish(database_migrations_url()?.as_str())
            .map_err(|err| anyhow::Error::msg(format!("error connecting to db for migrations: {err}")))?;

        info!("running pending database migrations");

        if let Err(err) = conn.run_pending_migrations(MIGRATIONS) {
            return Err(anyhow::Error::msg(format!("{err}")));
        }
        Ok(())
    })
    .await??;

    let mut inner_pool = pc::bb8::Pool::builder();

    if let Some(max_connections) = env::postgres_max_connections()? {
        inner_pool = inner_pool.max_size(max_connections);
    }

    let db = DbPool::bb8(
        inner_pool
            .build(pc::AsyncDieselConnectionManager::new(database_url()?.as_str()))
            .await?,
    );

    info!("pinging database");
    db.query(|conn| {
        use pc::PoolableConnection;
        Box::pin(conn.ping())
    })
    .await?;
    info!("pinged database");

    Ok(db)
}

fn database_url() -> Result<url::Url, anyhow::Error> {
    let mut database_url = url::Url::parse("postgres://localhost")?;
    if let Some(username) = env::postgres_username_server()? {
        database_url
            .set_username(&username)
            .map_err(|_| anyhow::Error::msg("unable to set db url username"))?;
    }
    let password_server = env::postgres_password_server()?;
    database_url
        .set_password(password_server.as_ref().map(|x| &**x))
        .map_err(|_| anyhow::Error::msg("unable to set db url password"))?;
    database_url.set_host(Some(&*env::postgres_host()?))?;
    database_url
        .set_port(env::postgres_port()?)
        .map_err(|_| anyhow::Error::msg("unable to set db url port"))?;
    database_url.set_path(&env::postgres_db()?);
    Ok(database_url)
}

fn database_migrations_url() -> Result<url::Url, anyhow::Error> {
    let mut database_migrations_url = url::Url::parse("postgres://localhost")?;
    if let Some(username) = env::postgres_username_migration()? {
        database_migrations_url
            .set_username(&username)
            .map_err(|_| anyhow::Error::msg("unable to set db migration url username"))?;
    }
    let password_migration = env::postgres_password_migration()?;
    database_migrations_url
        .set_password(password_migration.as_ref().map(|x| &**x))
        .map_err(|_| anyhow::Error::msg("unable to set db migration url password"))?;
    database_migrations_url.set_host(Some(&*env::postgres_host()?))?;
    database_migrations_url
        .set_port(env::postgres_port()?)
        .map_err(|_| anyhow::Error::msg("unable to set db migration url port"))?;
    database_migrations_url.set_path(&env::postgres_db()?);

    Ok(database_migrations_url)
}
