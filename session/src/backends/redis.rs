use crate::*;
use anyhow::Error;
use deadpool::managed::{self, Object, Pool};
use derivative::Derivative;
use log::info;
use redis_cluster_async::redis::{self, aio::ConnectionLike, cmd, RedisError};
use ring::hmac::{Key, HMAC_SHA256};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::{Debug, Display};
use std::ops::{Deref, DerefMut};
use std::{marker::PhantomData, sync::Arc};
use typed_builder::TypedBuilder;
use url::Url;
use uuid::Uuid;

#[derive(Clone, Copy, Derivative, TypedBuilder)]
#[derivative(Debug)]
pub struct RedisStoreConfig<KN, K, U, P> {
    pub key_name: KN,
    pub key: K,
    pub username: Option<U>,
    #[derivative(Debug = "ignore")]
    pub password: Option<P>,
}

#[derive(Clone, Copy, Debug, TypedBuilder)]
pub struct RedisStoreNodeConfig<H> {
    pub host: H,
    pub port: Option<u16>,
    pub db: Option<u16>,
}

pub struct Manager<Client> {
    client: Client,
}

pub struct Connection<Client>(Object<Manager<Client>>)
where
    Manager<Client>: managed::Manager;

impl<Client> Deref for Connection<Client>
where
    Manager<Client>: managed::Manager,
{
    type Target = <Manager<Client> as managed::Manager>::Type;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<Client> DerefMut for Connection<Client>
where
    Manager<Client>: managed::Manager,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<Client> From<Object<Manager<Client>>> for Connection<Client>
where
    Manager<Client>: managed::Manager,
{
    fn from(object: Object<Manager<Client>>) -> Self {
        Self(object)
    }
}

#[async_trait]
impl managed::Manager for Manager<redis::Client> {
    type Type = redis::aio::Connection;
    type Error = RedisError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        self.client.get_async_connection().await
    }

    async fn recycle(&self, mut conn: &mut Self::Type) -> managed::RecycleResult<Self::Error> {
        cmd("PING").query_async::<_, ()>(conn.deref_mut()).await?;
        Ok(())
    }
}

#[async_trait]
impl managed::Manager for Manager<redis_cluster_async::Client> {
    type Type = redis_cluster_async::Connection;
    type Error = RedisError;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        self.client.get_connection().await
    }

    async fn recycle(&self, mut conn: &mut Self::Type) -> managed::RecycleResult<Self::Error> {
        cmd("PING").query_async::<_, ()>(conn.deref_mut()).await?;
        Ok(())
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct RedisStore<T, Pool> {
    key_name: String,
    key: Arc<Key>,
    #[derivative(Debug = "ignore")]
    pool: Pool,
    #[derivative(Debug = "ignore")]
    _value: PhantomData<T>,
}

#[async_trait]
impl<T, Manager, Connection, C> SessionStore for RedisStore<T, Pool<Manager, Connection>>
where
    T: 'static + Clone + DeserializeOwned + Serialize + Send + Sync,
    Manager: 'static + managed::Manager + Send + Sync,
    <Manager as managed::Manager>::Type: 'static + Send + Sync,
    <Manager as managed::Manager>::Error: 'static + Debug + Display + Send + Sync,
    Connection: 'static + From<Object<Manager>> + Send + Sync + DerefMut<Target = C>,
    C: ConnectionLike + Send,
{
    type Value = T;

    fn key_name(&self) -> &str {
        &self.key_name
    }
    fn key(&self) -> &Key {
        self.key.deref()
    }

    async fn set(
        &self,
        prefix: Option<String>,
        session_id: &Uuid,
        session: &Session<Self::Value>,
    ) -> Result<(), Error> {
        let mut conn = self.pool.get().await.map_err(Error::msg)?;
        let value = serde_json::to_string(&session).map_err(Error::msg)?;
        let expires = if let Some(max_age) = session.max_age.as_ref() {
            Some(format!("{}", max_age.num_seconds()))
        } else {
            session
                .expires
                .as_ref()
                .map(|expires| format!("{}", (*expires - session.created_at).num_seconds()))
        };

        let session_id = format!("{session_id}");

        let args = match expires.as_ref() {
            Some(expires) => vec![&*session_id, &*value, "EX", &**expires],
            None => vec![&*session_id, &*value],
        };

        if let Some(prefix) = prefix {
            cmd("SET")
                .arg(&args)
                .query_async(conn.deref_mut())
                .await
                .map_err(Error::msg)?;

            tokio::spawn(async move {
                cmd("SADD")
                    .arg(&[&prefix, &session_id])
                    .query_async::<_, ()>(conn.deref_mut())
                    .await
                    .map_err(Error::msg)?;
                Ok::<_, Error>(())
            });
        } else {
            cmd("SET")
                .arg(&args)
                .query_async::<_, ()>(conn.deref_mut())
                .await
                .map_err(Error::msg)?;
        }

        Ok(())
    }

    async fn get(&self, session_id: &Uuid) -> Result<Session<Self::Value>, Error> {
        let mut conn = self.pool.get().await.map_err(Error::msg)?;
        let value: String = cmd("GET")
            .arg(&[&format!("{session_id}")])
            .query_async(conn.deref_mut())
            .await
            .map_err(Error::msg)?;
        let mut session: Session<Self::Value> = serde_json::from_str(&value).map_err(Error::msg)?;
        session.session_id = *session_id;
        Ok(session)
    }

    async fn delete(&self, session_id: &Uuid) -> Result<(), Error> {
        let mut conn = self.pool.get().await.map_err(Error::msg)?;
        cmd("DEL")
            .arg(&[&format!("{session_id}")])
            .query_async::<_, ()>(conn.deref_mut())
            .await
            .map_err(Error::msg)?;
        Ok(())
    }
}

pub async fn redis_store_standalone<T, KN, K, U, P, H>(
    RedisStoreConfig {
        key_name,
        key,
        username,
        password,
    }: RedisStoreConfig<KN, K, U, P>,
    RedisStoreNodeConfig { host, port, db }: RedisStoreNodeConfig<H>,
) -> Result<RedisStore<T, Pool<Manager<redis::Client>, Connection<redis::Client>>>, Error>
where
    KN: ToString,
    K: ToString,
    U: ToString,
    P: ToString,
    H: ToString,
{
    let key_name = key_name.to_string();
    let key = key.to_string();
    let username = username.as_ref().map(ToString::to_string);
    let password = password.as_ref().map(ToString::to_string);
    let host = host.to_string();

    let path = db.map(|db| format!("/{db}"));

    let url = url(
        username.as_deref(),
        password.as_deref(),
        &host,
        port,
        path.as_deref(),
        None,
    )?;
    let safe_url = safe_url(
        username.as_deref(),
        password.as_deref(),
        &host,
        port,
        path.as_deref(),
        None,
    )?;

    info!("connecting to redis session stores at {safe_url}");

    let client = redis::Client::open(url)?;

    let pool = Pool::builder(Manager { client }).build().map_err(Error::msg)?;

    // confirm a connection can be made
    pool.get().await.map_err(Error::msg)?;

    Ok(RedisStore {
        key_name,
        key: Arc::new(Key::new(HMAC_SHA256, key.as_bytes())),
        _value: PhantomData,
        pool,
    })
}

pub async fn redis_store_cluster<T, KN, K, U, P, H>(
    RedisStoreConfig {
        key_name,
        key,
        username,
        password,
    }: RedisStoreConfig<KN, K, U, P>,
    node_configs: impl IntoIterator<Item = RedisStoreNodeConfig<H>>,
) -> Result<RedisStore<T, Pool<Manager<redis_cluster_async::Client>, Connection<redis_cluster_async::Client>>>, Error>
where
    KN: ToString,
    K: ToString,
    U: ToString,
    P: ToString,
    H: ToString,
{
    let key_name = key_name.to_string();
    let key = key.to_string();
    let username = username.as_ref().map(ToString::to_string);
    let password = password.as_ref().map(ToString::to_string);

    let (urls, safe_urls): (Vec<_>, Vec<_>) = node_configs
        .into_iter()
        .map(|RedisStoreNodeConfig { host, port, db }| {
            let host = host.to_string();
            let path = db.map(|db| format!("/{db}"));

            let url = url(
                username.as_deref(),
                password.as_deref(),
                &host,
                port,
                path.as_deref(),
                None,
            )?;
            let safe_url = safe_url(
                username.as_deref(),
                password.as_deref(),
                &host,
                port,
                path.as_deref(),
                None,
            )?;
            Ok((url, safe_url))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .into_iter()
        .unzip();

    if urls.is_empty() {
        return Err(Error::msg("no node config provided for cluster redis store"));
    }

    info!("connecting to redis session stores at:");
    for safe_url in safe_urls {
        info!("- {safe_url}");
    }

    let client = redis_cluster_async::Client::open(urls)?;

    let pool = Pool::builder(Manager { client }).build().map_err(Error::msg)?;

    // confirm a connection can be made
    pool.get().await.map_err(Error::msg)?;

    Ok(RedisStore {
        key_name,
        key: Arc::new(Key::new(HMAC_SHA256, key.as_bytes())),
        _value: PhantomData,
        pool,
    })
}

pub async fn redis_store<T, KN, K, U, P, H>(
    config: RedisStoreConfig<KN, K, U, P>,
    node_configs: impl IntoIterator<Item = RedisStoreNodeConfig<H>>,
    is_cluster: bool,
) -> Result<DynSessionStore<T>, Error>
where
    T: 'static + Clone + DeserializeOwned + Serialize + Send + Sync,
    KN: ToString,
    K: ToString,
    U: ToString,
    P: ToString,
    H: ToString,
{
    if is_cluster {
        redis_store_cluster(config, node_configs).await.map(|x| x.into_dyn())
    } else {
        let mut node_config_iter = node_configs.into_iter();
        let node_config = node_config_iter
            .next()
            .ok_or_else(|| Error::msg("no node config provided for standalone redis store"))?;
        if node_config_iter.next().is_some() {
            return Err(Error::msg(
                "more than one node config provided for standalone redis store",
            ));
        }
        redis_store_standalone(config, node_config).await.map(|x| x.into_dyn())
    }
}

fn url(
    username: Option<&str>,
    password: Option<&str>,
    host: &str,
    port: Option<u16>,
    path: Option<&str>,
    query: Option<&str>,
) -> Result<Url, Error> {
    let mut url = Url::parse(&format!("redis://{host}"))?;

    if let Some(username) = username {
        url.set_username(username)
            .map_err(|_| Error::msg("could not set url username"))?;
    }

    url.set_password(password)
        .map_err(|_| Error::msg("could not set url password"))?;

    url.set_port(port).map_err(|_| Error::msg("could not set url port"))?;

    if let Some(path) = path {
        url.set_path(path);
    }
    url.set_query(query);

    Ok(url)
}

fn safe_url(
    username: Option<&str>,
    password: Option<&str>,
    host: &str,
    port: Option<u16>,
    path: Option<&str>,
    query: Option<&str>,
) -> Result<String, Error> {
    let username = match username.is_some() || password.is_some() {
        true => Some("<credentials>"),
        false => None,
    };
    let safe_url = format!("{}", url(username, None, host, port, path, query)?);
    Ok(safe_url.replace("%3C", "<").replace("%3E", ">"))
}
