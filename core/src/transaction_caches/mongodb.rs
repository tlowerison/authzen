use crate::actions::*;
use crate::*;
use ::serde::{Deserialize, Serialize};
use chrono::Utc;
use futures::future::BoxFuture;
use futures::stream::TryStreamExt;
use mongodb::bson::{self, doc, Bson, Document};
use service_util::{instrument_field, Error};

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

impl<SC: ?Sized + StorageClient> TransactionCache<SC> for MongodbTxCollection {
    type Error = Error;

    fn get_entities<'life0, 'life1, 'async_trait, O, T>(
        &'life0 self,
        transaction_id: SC::TransactionId<'life1>,
    ) -> BoxFuture<'async_trait, Result<HashMap<T::Id, TxCacheEntity<T, T::Id>>, Self::Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
    {
        Box::pin(async move {
            instrument_field!("service_name", O::SERVICE);
            instrument_field!("object_type", O::TYPE);
            let pipeline =
                group_pipeline::<SC::TransactionId<'_>, O, T>(transaction_id, None).map_err(Error::default_details)?;
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

    fn get_by_ids<'life0, 'life1, 'life2, 'async_trait, O, T>(
        &'life0 self,
        transaction_id: SC::TransactionId<'life1>,
        ids: &'life2 [T::Id],
    ) -> BoxFuture<'async_trait, Result<Vec<T>, Self::Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
    {
        Box::pin(async move {
            instrument_field!("service_name", O::SERVICE);
            instrument_field!("object_type", O::TYPE);
            let pipeline =
                group_pipeline::<SC::TransactionId<'_>, O, T>(transaction_id, ids).map_err(Error::default_details)?;
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

    fn upsert<'life0, 'life1, 'life2, 'async_trait, O, T>(
        &'life0 self,
        transaction_id: SC::TransactionId<'life1>,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life2,
    ) -> BoxFuture<'async_trait, Result<(), Self::Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        SC::TransactionId<'life1>: Clone + Serialize,
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

    fn mark_deleted<'life0, 'life1, 'life2, 'async_trait, O, T>(
        &'life0 self,
        transaction_id: SC::TransactionId<'life1>,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life2,
    ) -> BoxFuture<'async_trait, Result<(), Self::Error>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        SC::TransactionId<'life1>: Clone + Serialize,
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
    fn manage_cache<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        transaction_id: SC::TransactionId<'life1>,
        ok: &'life2 <Create<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache<SC>>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        <Self as TransactionCache<SC>>::upsert::<O, T>(self, transaction_id, ok)
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
    fn manage_cache<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        transaction_id: SC::TransactionId<'life1>,
        ok: &'life2 <Delete<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache<SC>>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        <Self as TransactionCache<SC>>::mark_deleted::<O, T>(self, transaction_id, ok)
    }
}

impl<O, SC, I> TransactionCacheAction<Read<O>, SC, I> for MongodbTxCollection
where
    O: ?Sized + ObjectType,
    SC: ?Sized + StorageClient + Send + Sync,
    Read<O>: StorageAction<SC, I> + Send,
{
    fn manage_cache<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        _: SC::TransactionId<'life1>,
        _: &'life2 <Read<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache<SC>>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
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
    fn manage_cache<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        transaction_id: SC::TransactionId<'life1>,
        ok: &'life2 <Update<O> as StorageAction<SC, I>>::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache<SC>>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        <Self as TransactionCache<SC>>::upsert::<O, T>(self, transaction_id, ok)
    }
}
