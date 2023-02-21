use crate::*;
use serde::de::{Deserialize, DeserializeOwned, Deserializer};
use serde::Serialize;
use service_util::{Error, Query};
use session_util::AccountSession;
use std::borrow::{Borrow, Cow};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use uuid::Uuid;

/// use this type in places where authorization disabling needs to be very clear in all contexts
/// e.g. as a function parameter, where both the function and the function caller should be
/// unambiguously aware of what the parameter they are using / passing means
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DisableAuthorization {
    True,
    False,
}

pub trait OPAIdentifiable {
    fn id(&self) -> Uuid;
}

pub trait AllOPAIdentifiable {
    fn ids(&self) -> Vec<Uuid>;
}

pub trait AuthzService {
    const NAME: &'static str;

    fn service(&self) -> &'static str {
        Self::NAME
    }
}

#[derive(AsRef, AsMut, Clone, Derivative, Deref, Serialize)]
#[derivative(Debug(bound = "T: Debug, <T as ToOwned>::Owned: Debug"))]
pub struct OPAType<'a, T: ToOwned + 'a + ?Sized>(#[serde(borrow)] pub Cow<'a, T>);

impl<'a, T: ToOwned + ?Sized> From<&'a T> for OPAType<'a, T> {
    fn from(value: &'a T) -> Self {
        Self(Cow::Borrowed(value))
    }
}

impl<T: ToOwned<Owned = T>> From<T> for OPAType<'_, T> {
    fn from(value: T) -> Self {
        Self(Cow::Owned(value))
    }
}

impl<'de, 'a, T: ?Sized> Deserialize<'de> for OPAType<'a, T>
where
    T: ToOwned,
    T::Owned: Deserialize<'de>,
{
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        T::Owned::deserialize(deserializer).map(Cow::Owned).map(OPAType)
    }
}

pub trait OPAKey {
    fn base_opa_key() -> String;
    fn opa_keys_with(ids: &[Uuid]) -> Vec<String> {
        let base_opa_key = Self::base_opa_key();
        let mut buffer = Uuid::encode_buffer();
        ids.iter()
            .map(|id| base_opa_key.clone() + "." + encode_id(id, &mut buffer))
            .collect()
    }
}

pub trait OPAIdentifiableEntity {
    /// output should look like "{service}.{entity}.{id}"
    fn opa_key(&self) -> String;
}

impl<T: AuthzServiceEntity> OPAKey for T {
    fn base_opa_key() -> String {
        <Self as AuthzServiceEntity>::Service::NAME.to_owned() + "." + Self::NAME
    }
}

impl<T: OPAKey + OPAIdentifiable> OPAIdentifiableEntity for T {
    fn opa_key(&self) -> String {
        let mut buffer = Uuid::encode_buffer();
        Self::base_opa_key() + "." + encode_id(&self.id(), &mut buffer)
    }
}

#[async_trait]
pub trait AuthzServiceEntity: Clone + Debug + Send + Sized + Serialize + Sync {
    type Service: AuthzService;
    type Patch<'a>: Clone + Debug + Send + Serialize + Sync = ();
    const NAME: &'static str;

    fn entity(&self) -> &'static str {
        Self::NAME
    }
    fn service(&self) -> &'static str {
        Self::Service::NAME
    }

    fn can_create_query<Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        records: impl IntoIterator<Item = impl Into<Self>>,
    ) -> OPAQuery<'_, Self> {
        OPAQuery::builder()
            .input(Cow::Owned(OPAQueryInput {
                transaction_id: ctx.transaction_id(),
                token: ctx.account_session().map(|x| &*x.value.token),
                action: OPAQueryInputAction::Create {
                    service: Cow::Borrowed(Self::Service::NAME),
                    entity: Cow::Borrowed(Self::NAME),
                    records: records.into_iter().map(Into::into).collect(),
                },
            }))
            .build()
    }

    fn can_delete_query<Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        ids: impl IntoIterator<Item = impl Borrow<Uuid>>,
    ) -> OPAQuery<'_, ()> {
        OPAQuery::builder()
            .input(Cow::Owned(OPAQueryInput {
                transaction_id: ctx.transaction_id(),
                token: ctx.account_session().map(|x| &*x.value.token),
                action: OPAQueryInputAction::Delete {
                    service: Cow::Borrowed(Self::Service::NAME),
                    entity: Cow::Borrowed(Self::NAME),
                    ids: ids.into_iter().map(|x| *x.borrow()).collect(),
                },
            }))
            .build()
    }

    fn can_read_query<Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        ids: impl IntoIterator<Item = impl Borrow<Uuid>>,
    ) -> OPAQuery<'_, ()> {
        OPAQuery::builder()
            .input(Cow::Owned(OPAQueryInput {
                transaction_id: ctx.transaction_id(),
                token: ctx.account_session().map(|x| &*x.value.token),
                action: OPAQueryInputAction::Read {
                    service: Cow::Borrowed(Self::Service::NAME),
                    entity: Cow::Borrowed(Self::NAME),
                    ids: ids.into_iter().map(|x| *x.borrow()).collect(),
                },
            }))
            .build()
    }

    fn can_update_query<'a, Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        patches: impl Debug + IntoIterator<Item = impl Into<Self::Patch<'a>>>,
    ) -> OPAQuery<'_, Self::Patch<'a>> {
        OPAQuery::builder()
            .input(Cow::Owned(OPAQueryInput {
                transaction_id: ctx.transaction_id(),
                token: ctx.account_session().map(|x| &*x.value.token),
                action: OPAQueryInputAction::Update {
                    service: Cow::Borrowed(Self::Service::NAME),
                    entity: Cow::Borrowed(Self::NAME),
                    patches: patches.into_iter().map(Into::into).collect(),
                },
            }))
            .build()
    }

    #[instrument(err(Debug), skip(ctx, records))]
    async fn can_create<Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        records: impl Debug + IntoIterator<Item = impl Into<Self> + Send> + Send,
    ) -> Result<(), Error> {
        let records = records.into_iter().collect::<Vec<_>>();
        if records.is_empty() {
            return Ok(());
        }
        let allowed: OPAQueryResult = Self::can_create_query(ctx, records).query(ctx.opa_client()).await?;
        allowed.ok_or_else(Error::bad_request)
    }

    #[instrument(err(Debug), skip(ctx, ids))]
    async fn can_delete<Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        ids: impl Debug + IntoIterator<Item = impl Borrow<Uuid> + Send> + Send,
    ) -> Result<(), Error> {
        let ids = ids.into_iter().collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(());
        }
        let allowed: OPAQueryResult = Self::can_delete_query(ctx, ids).query(ctx.opa_client()).await?;
        allowed.ok_or_else(Error::bad_request)
    }

    #[instrument(err(Debug), skip(ctx, ids))]
    async fn can_read<Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        ids: impl Debug + IntoIterator<Item = impl Borrow<Uuid> + Send> + Send,
    ) -> Result<(), Error> {
        let ids = ids.into_iter().collect::<Vec<_>>();
        if ids.is_empty() {
            return Ok(());
        }
        let allowed: OPAQueryResult = Self::can_read_query(ctx, ids).query(ctx.opa_client()).await?;
        allowed.ok_or_else(Error::bad_request)
    }

    #[instrument(err(Debug), skip(ctx, patches))]
    async fn can_update<Ctx: OPAContext + OPATxCacheContext>(
        ctx: &Ctx,
        patches: impl Debug + IntoIterator<Item = impl Debug + Into<Self::Patch<'_>> + Send> + Send,
    ) -> Result<(), Error> {
        let patches = patches.into_iter().collect::<Vec<_>>();
        if patches.is_empty() {
            return Ok(());
        }
        let allowed: OPAQueryResult = Self::can_update_query(ctx, patches).query(ctx.opa_client()).await?;
        allowed.ok_or_else(Error::bad_request)
    }

    async fn tx_cache_upsert<O>(
        ctx: O,
        entities: impl IntoIterator<Item = impl Into<Self>> + Send,
    ) -> Result<(), anyhow::Error>
    where
        Self: OPAIdentifiable,
        O: OPATxCacheContext,
    {
        if let Some(transaction_id) = ctx.transaction_id() {
            let entities = entities.into_iter().map(Into::into).collect::<Vec<_>>();
            let opa_tx_cache_client = ctx.opa_tx_cache_client();
            if let Err(err) = opa_tx_cache_client.upsert(transaction_id, entities).await {
                let msg = format!("unable to upsert into tx cache: {err}");
                error!("{msg}");
                return Err(anyhow::Error::msg(msg));
            }
        }
        Ok(())
    }

    async fn tx_cache_mark_deleted<O>(
        ctx: O,
        entities: impl IntoIterator<Item = impl Into<Self>> + Send,
    ) -> Result<(), anyhow::Error>
    where
        Self: OPAIdentifiable,
        O: OPATxCacheContext,
    {
        if let Some(transaction_id) = ctx.transaction_id() {
            let entities = entities.into_iter().map(Into::into).collect::<Vec<_>>();
            let opa_tx_cache_client = ctx.opa_tx_cache_client();
            if let Err(err) = opa_tx_cache_client.mark_deleted(transaction_id, entities).await {
                let msg = format!("unable to mark entities as deleted in tx cache: {err}");
                error!("{msg}");
                return Err(anyhow::Error::msg(msg));
            }
        }
        Ok(())
    }
}

impl OPAIdentifiable for Uuid {
    fn id(&self) -> Uuid {
        *self
    }
}

impl<T> OPAIdentifiable for (Uuid, T) {
    fn id(&self) -> Uuid {
        self.0
    }
}

impl<T: OPAIdentifiable> AllOPAIdentifiable for T {
    fn ids(&self) -> Vec<Uuid> {
        vec![self.id()]
    }
}

impl<T: OPAIdentifiable> AllOPAIdentifiable for Vec<T> {
    fn ids(&self) -> Vec<Uuid> {
        self.iter().map(|x| x.id()).collect()
    }
}

impl<T: OPAIdentifiable> AllOPAIdentifiable for &[T] {
    fn ids(&self) -> Vec<Uuid> {
        self.iter().map(|x| x.id()).collect()
    }
}

#[cfg(feature = "db")]
impl<T> OPAIdentifiable for authzen_diesel::WithTitle<T> {
    fn id(&self) -> Uuid {
        self.id
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccountSessionFields {
    pub role_ids: Vec<Uuid>,
}

pub trait OPAContext: Send + Sync {
    type AccountId: Send + Serialize + Sync;
    type AccountSessionFields: Send + Serialize + Sync;
    fn account_session(&self) -> Option<&AccountSession<Self::AccountId, Self::AccountSessionFields>>;
    fn opa_client(&self) -> &OPAClient;
}

pub trait OPATxCacheContext: Send + Sync {
    type TxCacheClient: OPATxCacheClient;
    fn opa_tx_cache_client(&self) -> Self::TxCacheClient;
    fn transaction_id(&self) -> Option<Uuid>;
}

impl<C: OPAContext> OPAContext for &C {
    type AccountId = C::AccountId;
    type AccountSessionFields = C::AccountSessionFields;
    fn account_session(&self) -> Option<&AccountSession<Self::AccountId, Self::AccountSessionFields>> {
        (*self).account_session()
    }
    fn opa_client(&self) -> &OPAClient {
        (*self).opa_client()
    }
}

impl<'a, C: OPATxCacheContext> OPATxCacheContext for &'a C {
    type TxCacheClient = C::TxCacheClient;
    fn transaction_id(&self) -> Option<Uuid> {
        (*self).transaction_id()
    }
    fn opa_tx_cache_client(&self) -> Self::TxCacheClient {
        (*self).opa_tx_cache_client()
    }
}

pub type OPADataCacheUpsert = (String, serde_json::Value);

#[async_trait]
pub trait OPATxCacheClient: Send + Sync {
    type Error: Debug + Display;
    async fn get_entities<T: AuthzServiceEntity + DeserializeOwned>(
        &self,
        transaction_id: Uuid,
    ) -> Result<HashMap<Uuid, OPATxEntity<T>>, Self::Error>;

    async fn get_by_ids<T: AuthzServiceEntity + DeserializeOwned>(
        &self,
        transaction_id: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<T>, Self::Error>;

    async fn upsert<T: AuthzServiceEntity + OPAIdentifiable>(
        &self,
        transaction_id: Uuid,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send,
    ) -> Result<(), Self::Error>;

    async fn mark_deleted<T: AuthzServiceEntity + OPAIdentifiable>(
        &self,
        transaction_id: Uuid,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send,
    ) -> Result<(), Self::Error>;
}

#[async_trait]
impl<O: OPATxCacheClient> OPATxCacheClient for &O {
    type Error = <O as OPATxCacheClient>::Error;

    async fn get_entities<T: AuthzServiceEntity + DeserializeOwned>(
        &self,
        transaction_id: Uuid,
    ) -> Result<HashMap<Uuid, OPATxEntity<T>>, Self::Error> {
        (*self).get_entities::<T>(transaction_id).await
    }

    async fn get_by_ids<T: AuthzServiceEntity + DeserializeOwned>(
        &self,
        transaction_id: Uuid,
        ids: &[Uuid],
    ) -> Result<Vec<T>, Self::Error> {
        (*self).get_by_ids::<T>(transaction_id, ids).await
    }

    async fn upsert<T: AuthzServiceEntity + OPAIdentifiable>(
        &self,
        transaction_id: Uuid,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send,
    ) -> Result<(), Self::Error> {
        (*self).upsert::<T>(transaction_id, entities).await
    }

    async fn mark_deleted<T: AuthzServiceEntity + OPAIdentifiable>(
        &self,
        transaction_id: Uuid,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send,
    ) -> Result<(), Self::Error> {
        (*self).mark_deleted(transaction_id, entities).await
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct OPATxEntity<V> {
    pub exists: bool,
    #[serde(serialize_with = "serialize_id")]
    pub id: Uuid,
    pub value: V,
}

impl<V> OPATxEntity<V> {
    pub fn map<U>(self, f: impl FnOnce(V) -> U) -> OPATxEntity<U> {
        OPATxEntity {
            exists: self.exists,
            id: self.id,
            value: f(self.value),
        }
    }
    pub fn try_map<U, E>(self, f: impl FnOnce(V) -> Result<U, E>) -> Result<OPATxEntity<U>, E> {
        Ok(OPATxEntity {
            exists: self.exists,
            id: self.id,
            value: f(self.value)?,
        })
    }
}

fn encode_id<'a>(id: &Uuid, buffer: &'a mut [u8; 45]) -> &'a str {
    id.as_hyphenated().encode_lower(buffer)
}

fn serialize_id<S: serde::Serializer>(id: &Uuid, ser: S) -> Result<S::Ok, S::Error> {
    ser.serialize_str(encode_id(id, &mut Uuid::encode_buffer()))
}

#[cfg(feature = "mongodb-tx-cache-backend")]
pub use opa_mongodb::*;
#[cfg(feature = "mongodb-tx-cache-backend")]
mod opa_mongodb {
    use super::*;
    use chrono::Utc;
    use futures::stream::TryStreamExt;
    use mongodb::bson::{self, doc, Bson, Document};
    use serde_json::Value;

    pub const TTL_INDEX_NAME: &str = "edited_at_ttl";
    pub const DEFAULT_TTL_SECONDS: u64 = 120;

    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct OPATxEntityFull {
        pub transaction_id: Uuid,
        pub service_name: &'static str,
        pub entity_name: &'static str,
        pub edited_at: Bson,
        #[serde(flatten)]
        pub entity: Value,
    }

    #[derive(Debug, Deserialize)]
    struct Group<T> {
        entity: OPATxEntity<T>,
    }

    pub type OPAMongoCollectionValue = OPATxEntityFull;
    pub type OPAMongoCollection = mongodb::Collection<OPAMongoCollectionValue>;

    /// converts a uuid::Uuid into Bson
    /// for some reason Mongo stores uuid::Uuid with a generic subtype when
    /// serialized from a struct so we need to change the subtype here as well
    fn bson_uuid(id: Uuid) -> Bson {
        let mut binary = bson::Binary::from_uuid(bson::Uuid::from_bytes(id.into_bytes()));
        binary.subtype = bson::spec::BinarySubtype::Generic;
        Bson::Binary(binary)
    }

    fn serialize_entity<T: AuthzServiceEntity + OPAIdentifiable>(
        transaction_id: Uuid,
        entity: impl Borrow<T>,
    ) -> Result<OPATxEntityFull, Error> {
        Ok(OPATxEntityFull {
            transaction_id,
            service_name: T::Service::NAME,
            entity_name: T::NAME,
            edited_at: Bson::DateTime(bson::DateTime::from(Utc::now())),
            entity: serde_json::to_value(OPATxEntity {
                exists: true,
                id: entity.borrow().id(),
                value: serde_json::to_value(entity.borrow()).map_err(Error::default_details)?,
            })
            .map_err(Error::default_details)?,
        })
    }

    fn group_pipeline<'a, T: AuthzServiceEntity>(
        transaction_id: Uuid,
        ids: impl Into<Option<&'a [Uuid]>>,
    ) -> [Document; 3] {
        let match_document = match ids.into() {
            Some(ids) => doc! {
                "$match": {
                    "transaction_id": bson_uuid(transaction_id),
                    "service_name": T::Service::NAME,
                    "entity_name": T::NAME,
                    "id": {
                        "$in": ids.iter().map(|id| encode_id(id, &mut Uuid::encode_buffer()).to_string()).collect::<Vec<_>>(),
                    },
                },
            },
            None => doc! {
                "$match": {
                    "transaction_id": bson_uuid(transaction_id),
                    "service_name": T::Service::NAME,
                    "entity_name": T::NAME,
                },
            },
        };
        [
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
        ]
    }

    pub async fn initialize_ttl_index(
        collection: &OPAMongoCollection,
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
                                        .unwrap_or_else(|| Duration::new(DEFAULT_TTL_SECONDS, 0)),
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

    #[async_trait]
    impl OPATxCacheClient for OPAMongoCollection {
        type Error = Error;

        #[framed]
        #[instrument(err(Debug), skip(self), ret)]
        async fn get_entities<T: AuthzServiceEntity + DeserializeOwned>(
            &self,
            transaction_id: Uuid,
        ) -> Result<HashMap<Uuid, OPATxEntity<T>>, Self::Error> {
            instrument_field!("service", T::Service::NAME);
            instrument_field!("entity", T::NAME);
            let pipeline = group_pipeline::<T>(transaction_id, None);
            let mut cursor = self.aggregate(pipeline, None).await.map_err(Error::default_details)?;

            let mut entities = Vec::<OPATxEntity<T>>::default();
            while let Some(document) = cursor.try_next().await.map_err(Error::default_details)? {
                let value: Value = serde_json::to_value(document).map_err(Error::default_details)?;
                let group: Group<T> = serde_json::from_value(value).map_err(Error::default_details)?;
                entities.push(group.entity);
            }
            Ok(entities.into_iter().map(|entity| (entity.id, entity)).collect())
        }

        #[framed]
        #[instrument(err(Debug), skip(self, ids))]
        async fn get_by_ids<T: AuthzServiceEntity + DeserializeOwned>(
            &self,
            transaction_id: Uuid,
            ids: &[Uuid],
        ) -> Result<Vec<T>, Self::Error> {
            instrument_field!("service", T::Service::NAME);
            instrument_field!("entity", T::NAME);
            let mut cursor = self
                .aggregate(group_pipeline::<T>(transaction_id, ids), None)
                .await
                .map_err(Error::default_details)?;

            let mut entities = Vec::<OPATxEntity<T>>::default();
            while let Some(document) = cursor.try_next().await.map_err(Error::default_details)? {
                let mut bytes = Vec::<u8>::new();
                document.to_writer(&mut bytes).map_err(Error::default_details)?;
                let group: Group<T> = serde_json::from_slice(&bytes).map_err(Error::default_details)?;
                entities.push(group.entity);
            }
            Ok(entities.into_iter().map(|entity| entity.value).collect())
        }

        #[framed]
        #[instrument(err(Debug), skip(self, entities))]
        async fn upsert<T: AuthzServiceEntity + OPAIdentifiable>(
            &self,
            transaction_id: Uuid,
            entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send,
        ) -> Result<(), Self::Error> {
            instrument_field!("service", T::Service::NAME);
            instrument_field!("entity", T::NAME);
            let entity_fulls = entities
                .into_iter()
                .map(|entity| serialize_entity(transaction_id, entity))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Error::default_details)?;
            self.insert_many(entity_fulls, None)
                .await
                .map_err(Error::default_details)?;
            Ok(())
        }

        #[framed]
        #[instrument(err(Debug), skip(self, entities))]
        async fn mark_deleted<T: AuthzServiceEntity + OPAIdentifiable>(
            &self,
            transaction_id: Uuid,
            entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send,
        ) -> Result<(), Self::Error> {
            instrument_field!("service", T::Service::NAME);
            instrument_field!("entity", T::NAME);
            let entity_fulls = entities
                .into_iter()
                .map(|entity| serialize_entity(transaction_id, entity))
                .collect::<Result<Vec<_>, _>>()
                .map_err(Error::default_details)?;
            self.insert_many(entity_fulls, None)
                .await
                .map_err(Error::default_details)?;
            Ok(())
        }
    }
}
