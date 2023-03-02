use crate::actions::*;
use crate::*;
use ::authzen_service_util::{instrument_field, Error};
use ::chrono::Utc;
use ::derivative::Derivative;
use ::futures::future::BoxFuture;
use ::futures::stream::TryStreamExt;
use ::mongodb::bson::{self, doc, Bson, Document};
use ::serde::{Deserialize, Serialize};

pub const TTL_INDEX_NAME: &str = "edited_at_ttl";
pub const DEFAULT_TTL_SECONDS: u64 = 120;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TxEntityFull {
    pub transaction_id: Bson,
    pub service_name: &'static str,
    pub object_type: &'static str,
    pub edited_at: Bson,
    #[serde(flatten)]
    pub entity: Bson,
}

#[derive(Debug, Deserialize)]
struct Group<T, Id> {
    entity: TxCacheEntity<T, Id>,
}

pub type MongodbTxCollection = mongodb::Collection<TxEntityFull>;

impl TxEntityFull {
    fn try_from<TransactionId, O, T>(transaction_id: TransactionId, entity: &T) -> Result<Self, bson::ser::Error>
    where
        TransactionId: Serialize,
        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
    {
        Ok(TxEntityFull {
            transaction_id: bson::to_bson(&transaction_id)?,
            service_name: O::SERVICE,
            object_type: O::TYPE,
            edited_at: Bson::DateTime(bson::DateTime::from(Utc::now())),
            entity: bson::to_bson(&TxCacheEntity {
                exists: true,
                id: entity.id(),
                value: bson::to_bson(entity)?,
            })?,
        })
    }
}

fn group_pipeline<'a, TransactionId, O, T>(
    transaction_id: TransactionId,
    ids: impl Into<Option<&'a [T::Id]>>,
) -> Result<[Document; 3], bson::ser::Error>
where
    TransactionId: Serialize,
    O: ?Sized + ObjectType,
    T: Identifiable,
{
    let match_document = match ids.into() {
        Some(ids) => doc! {
            "$match": {
                "transaction_id": bson::to_bson(&transaction_id)?,
                "service_name": O::SERVICE,
                "object_type": O::TYPE,
                "id": {
                    "$in": bson::to_bson(ids)?,
                },
            },
        },
        None => doc! {
            "$match": {
                "transaction_id": bson::to_bson(&transaction_id)?,
                "service_name": O::SERVICE,
                "object_type": O::TYPE,
            },
        },
    };
    Ok([
        match_document,
        doc! {
            "$sort": {
                "edited_at": -1,
            },
        },
        doc! {
            "$group": {
                "_id": {
                    "id": "$id",
                },
                "entity": {
                    "$first": {
                        "exists": "$exists",
                        "id": "$id",
                        "value": "$value",
                    },
                },
            },
        },
    ])
}

pub async fn initialize_ttl_index(
    collection: &MongodbTxCollection,
    ttl_duration: impl Into<Option<std::time::Duration>>,
) -> Result<(), anyhow::Error> {
    let ttl_index_exists = collection
        .list_index_names()
        .await
        .map_err(anyhow::Error::msg)?
        .into_iter()
        .any(|index_name| index_name == TTL_INDEX_NAME);
    if !ttl_index_exists {
        collection
            .create_index(
                mongodb::IndexModel::builder()
                    .keys(doc! {
                        "edited_at": 1,
                    })
                    .options(Some(
                        mongodb::options::IndexOptions::builder()
                            .name(TTL_INDEX_NAME.to_string())
                            .expire_after(
                                ttl_duration
                                    .into()
                                    .unwrap_or_else(|| std::time::Duration::new(DEFAULT_TTL_SECONDS, 0)),
                            )
                            .build(),
                    ))
                    .build(),
                None,
            )
            .await
            .map_err(anyhow::Error::msg)?;
    }
    Ok(())
}

impl TransactionCache for MongodbTxCollection {
    type Error = Error;

    fn get_entities<'life0, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
    ) -> BoxFuture<'async_trait, Result<HashMap<T::Id, TxCacheEntity<T, T::Id>>, Self::Error>>
    where
        'life0: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize,
    {
        Box::pin(async move {
            instrument_field!("service_name", O::SERVICE);
            instrument_field!("object_type", O::TYPE);
            let pipeline = group_pipeline::<_, O, T>(transaction_id, None).map_err(Error::default_details)?;
            let mut cursor = self.aggregate(pipeline, None).await.map_err(Error::default_details)?;

            let mut entities = Vec::<TxCacheEntity<T, T::Id>>::default();
            while let Some(document) = cursor.try_next().await.map_err(Error::default_details)? {
                let group: Group<T, T::Id> =
                    bson::from_bson(Bson::Document(document)).map_err(Error::default_details)?;
                entities.push(group.entity);
            }
            Ok(entities.into_iter().map(|entity| (entity.id.clone(), entity)).collect())
        })
    }

    fn get_by_ids<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        ids: &'life1 [T::Id],
    ) -> BoxFuture<'async_trait, Result<Vec<T>, Self::Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize,
    {
        Box::pin(async move {
            instrument_field!("service_name", O::SERVICE);
            instrument_field!("object_type", O::TYPE);
            let pipeline = group_pipeline::<_, O, T>(transaction_id, ids).map_err(Error::default_details)?;
            let mut cursor = self.aggregate(pipeline, None).await.map_err(Error::default_details)?;

            let mut entities = Vec::<TxCacheEntity<T, T::Id>>::default();
            while let Some(document) = cursor.try_next().await.map_err(Error::default_details)? {
                let group: Group<T, T::Id> =
                    bson::from_bson(Bson::Document(document)).map_err(Error::default_details)?;
                entities.push(group.entity);
            }
            Ok(entities.into_iter().map(|entity| entity.value).collect())
        })
    }

    fn upsert<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> BoxFuture<'async_trait, Result<(), Self::Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Clone + Send + Serialize,
    {
        Box::pin(async move {
            instrument_field!("service_name", O::SERVICE);
            instrument_field!("object_type", O::TYPE);
            let entity_fulls = entities
                .into_iter()
                .map(|entity| TxEntityFull::try_from::<_, O, T>(transaction_id.clone(), entity.borrow()))
                .collect::<Result<Vec<TxEntityFull>, _>>()
                .map_err(Error::default_details)?;
            self.insert_many(entity_fulls, None)
                .await
                .map_err(Error::default_details)?;
            Ok(())
        })
    }

    fn mark_deleted<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> BoxFuture<'async_trait, Result<(), Self::Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Clone + Send + Serialize,
    {
        Box::pin(async move {
            instrument_field!("service_name", O::SERVICE);
            instrument_field!("object_type", O::TYPE);
            let entity_fulls = entities
                .into_iter()
                .map(|entity| TxEntityFull::try_from::<_, O, T>(transaction_id.clone(), entity.borrow()))
                .collect::<Result<Vec<TxEntityFull>, _>>()
                .map_err(Error::default_details)?;
            self.insert_many(entity_fulls, None)
                .await
                .map_err(Error::default_details)?;
            Ok(())
        })
    }
}

impl<O, SC, I, T> TransactionCacheAction<Create<O>, SC, I> for MongodbTxCollection
where
    O: ?Sized + ObjectType,
    SC: ?Sized + StorageClient + Send + Sync,
    Create<O>: StorageAction<SC, I> + Send,
    for<'a> &'a <Create<O> as StorageAction<SC, I>>::Ok: IntoIterator<Item = &'a T>,
    T: Identifiable + Serialize + Sync + 'static,
{
    fn manage_cache<'life0, 'life1, 'async_trait>(
        &'life0 self,
        transaction_id: SC::TransactionId,
        ok: &'life1 <Create<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        SC::TransactionId: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        <Self as TransactionCache>::upsert::<O, T, SC::TransactionId>(self, transaction_id, ok)
    }
}

impl<O, SC, I, T> TransactionCacheAction<Delete<O>, SC, I> for MongodbTxCollection
where
    O: ?Sized + ObjectType,
    SC: ?Sized + StorageClient + Send + Sync,
    Delete<O>: StorageAction<SC, I> + Send,
    for<'a> &'a <Delete<O> as StorageAction<SC, I>>::Ok: IntoIterator<Item = &'a T>,
    T: Identifiable + Serialize + Sync + 'static,
{
    fn manage_cache<'life0, 'life1, 'async_trait>(
        &'life0 self,
        transaction_id: SC::TransactionId,
        ok: &'life1 <Delete<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        SC::TransactionId: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        <Self as TransactionCache>::mark_deleted::<O, T, SC::TransactionId>(self, transaction_id, ok)
    }
}

impl<O, SC, I> TransactionCacheAction<Read<O>, SC, I> for MongodbTxCollection
where
    O: ?Sized + ObjectType,
    SC: ?Sized + StorageClient + Send + Sync,
    Read<O>: StorageAction<SC, I> + Send,
{
    fn manage_cache<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _: SC::TransactionId,
        _: &'life1 <Read<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        SC::TransactionId: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        Box::pin(async { Ok(()) })
    }
}

impl<O, SC, I, T> TransactionCacheAction<Update<O>, SC, I> for MongodbTxCollection
where
    O: ?Sized + ObjectType,
    SC: ?Sized + StorageClient + Send + Sync,
    Update<O>: StorageAction<SC, I> + Send,
    for<'a> &'a <Update<O> as StorageAction<SC, I>>::Ok: IntoIterator<Item = &'a T>,
    T: Identifiable + Serialize + Sync + 'static,
{
    fn manage_cache<'life0, 'life1, 'async_trait>(
        &'life0 self,
        transaction_id: SC::TransactionId,
        ok: &'life1 <Update<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        SC::TransactionId: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        <Self as TransactionCache>::upsert::<O, T, SC::TransactionId>(self, transaction_id, ok)
    }
}

#[derive(Clone, Derivative, TypedBuilder)]
#[derivative(Debug)]
pub struct MongodbConfig {
    #[builder(setter(into))]
    pub scheme: String,
    #[builder(default)]
    #[derivative(Debug = "ignore")]
    pub username: Option<String>,
    #[builder(default)]
    #[derivative(Debug = "ignore")]
    pub password: Option<String>,
    #[builder(setter(into))]
    pub host: String,
    #[builder(setter(into))]
    pub port: Option<u16>,
    #[builder(default)]
    pub args: Option<String>,
    #[builder(setter(into))]
    pub database: String,
    #[builder(setter(into))]
    pub collection: String,
}

#[cfg_attr(feature = "tracing", instrument)]
pub async fn mongodb_client(
    config: MongodbConfig,
) -> Result<(::mongodb::Database, MongodbTxCollection), anyhow::Error> {
    let mut connection_string = url::Url::parse("mongodb://localhost")?;

    connection_string
        .set_scheme(&config.scheme)
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url scheme"))?;

    if let Some(username) = config.username {
        connection_string
            .set_username(&username)
            .map_err(|_| anyhow::Error::msg("unable to set mongodb url username"))?;
    }

    connection_string
        .set_password(config.password.as_ref().map(|x| &**x))
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url password"))?;

    connection_string
        .set_host(Some(&config.host))
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url host"))?;
    connection_string.set_path("/");
    connection_string
        .set_port(config.port)
        .map_err(|_| anyhow::Error::msg("unable to set mongodb url port"))?;

    connection_string.set_query(config.args.as_ref().map(|x| &**x));

    log::info!("connecting to mongodb");

    let mut mongodb_client_options = mongodb::options::ClientOptions::parse(connection_string).await?;
    mongodb_client_options.app_name = Some("accounts".into());
    let client = mongodb::Client::with_options(mongodb_client_options)?;

    let db = client.database(&config.database);

    log::info!("pinging mongodb");
    db.run_command(mongodb::bson::doc! {"ping": 1}, None).await?;

    log::info!("connected to mongodb successfully");

    let has_collection = db
        .list_collection_names(None)
        .await?
        .into_iter()
        .any(|collection_name| collection_name == config.collection);
    if !has_collection {
        log::info!("creating mongodb collection `{}`", config.collection);
        db.create_collection(&config.collection, None).await?;
        log::info!("created mongodb collection `{}`", config.collection);
    }

    let collection = db.collection::<TxEntityFull>(&config.collection);

    log::info!("initializing mongodb ttl index");
    initialize_ttl_index(&collection, None).await?;

    log::info!("initialized mongodb ttl index");

    Ok((db, collection))
}
