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

    // TODO: unfinished
    #[instrument(err(Debug), skip(ctx))]
    async fn filter<Ctx: OPAContext + OPATxCacheContext>(ctx: &Ctx) -> Result<(), Error> {
        let account_session = ctx.account_session();
        let evaluation = pe_query(
            ctx.opa_client(),
            PEQuery::builder()
                .explain(OPA_EXPLAIN.as_ref().map(|x| &**x))
                .input(
                    PEQueryInput::builder()
                        .token(account_session.map(|x| &*x.value.token))
                        .action(PEQueryInputAction::Read {
                            service: Cow::Borrowed(Self::Service::NAME),
                            entity: Cow::Borrowed(Self::NAME),
                        })
                        .build(),
                )
                .build(),
            account_session
                .map(|x| json!({ "app": { "subject": x.value.claims.state } }))
                .unwrap_or_else(|| json!({})),
        )
        .await
        .map_err(Error::bad_request_details);

        info!("{evaluation:#?}");

        Ok(())
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
impl<T> OPAIdentifiable for diesel_util::WithTitle<T> {
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

#[cfg(feature = "db")]
pub use db::*;
#[cfg(feature = "db")]
mod db {
    use super::*;
    use diesel::associations::HasTable;
    use diesel::backend::Backend;
    use diesel::expression::{AsExpression, Expression};
    use diesel::expression_methods::ExpressionMethods;
    use diesel::helper_types as ht;
    use diesel::query_dsl::methods::{FilterDsl, FindDsl};
    use diesel::query_source::QuerySource;
    use diesel::sql_types::SqlType;
    use diesel::{query_builder::*, Identifiable};
    use diesel::{Insertable, Table};
    use diesel_async::methods::*;
    use diesel_async::AsyncConnection;
    use diesel_util::*;
    use std::fmt::Debug;
    use std::hash::Hash;

    pub trait AuthzServiceDbEntity: AuthzServiceEntity + OPAIdentifiable + Sized {
        type AsyncConnection: AsyncConnection<Backend = Self::Backend>;
        type Backend: Backend;
        type Constructor<'a>: AuthzServiceDbEntity<
                Backend = Self::Backend,
                AsyncConnection = Self::AsyncConnection,
                DbRecord = Self::DbRecord,
                DbPost<'a> = Self::DbPost<'a>,
            > + Send = Self;
        type DbRecord: Clone + DbEntity + Debug + Send + Sync;
        type DbPost<'a>: Clone + Debug + Send + Sync = Self::DbRecord;
        type DbPatch<'a>: Clone + Debug + Send + Sync = ();
    }

    #[async_trait]
    pub trait AuthzServiceDbEntityCreate<'query, 'v: 'query, DbRecord, Ctx>:
        AuthzServiceDbEntity<DbRecord = DbRecord>
    where
        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send
            + Sync
            + for<'a> DbInsert<PostHelper<'a> = <Self::Constructor<'a> as AuthzServiceDbEntity>::DbPost<'a>>,
        <DbRecord::Raw as TryInto<DbRecord>>::Error: Display + Send,

        for<'a> &'a Self::DbPost<'v>: Into<Self::Constructor<'a>>,
        for<'a> &'a DbRecord: Into<Self::Constructor<'a>>,
    {
        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_create(
            ctx: &Ctx,
            db_posts: impl IntoIterator<Item = impl Into<Self::DbPost<'v>>> + Send + 'v,
        ) -> Result<Vec<DbRecord>, Error>
        where
            Self: 'v,
            Ctx: 'query,

            // Insertable bounds
            Vec<<DbRecord as DbInsert>::Post<'v>>: Insertable<DbRecord::Table> + Send,
            <Vec<<DbRecord as DbInsert>::Post<'v>> as Insertable<DbRecord::Table>>::Values: Send,
            <DbRecord::Table as QuerySource>::FromClause: Send,

            // Insert bounds
            for<'a> InsertStatement<
                DbRecord::Table,
                <Vec<<DbRecord as DbInsert>::Post<'v>> as Insertable<DbRecord::Table>>::Values,
            >: LoadQuery<'query, Ctx::AsyncConnection, DbRecord::Raw>,
            for<'a> InsertStatement<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                    <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                >>::Values,
            >: ExecuteDsl<Ctx::AsyncConnection>,

            // Audit bounds
            DbRecord::Raw: Audit + Clone,
            <DbRecord::Raw as Audit>::AuditRow: Send,
            Vec<<DbRecord::Raw as Audit>::AuditRow>: Insertable<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>
                + UndecoratedInsertRecord<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>,
            <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table: Table + QueryId + Send,
            <<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table as QuerySource>::FromClause: Send,
            <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
            >>::Values: Send,
        {
            let db_posts = db_posts.into_iter().map(Into::into).collect::<Vec<_>>();
            if db_posts.is_empty() {
                return Ok(vec![]);
            }
            Self::Constructor::can_create(ctx, db_posts.iter()).await?;
            let records = DbRecord::insert(ctx, db_posts).await?;
            Self::Constructor::tx_cache_upsert(ctx, records.iter()).await?;
            Ok(records)
        }

        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_create_one(ctx: &Ctx, db_post: impl Into<Self::DbPost<'v>> + Send) -> Result<DbRecord, Error>
        where
            Self: 'v,
            Ctx: 'query,

            // Insertable bounds
            Vec<<DbRecord as DbInsert>::Post<'v>>: Insertable<DbRecord::Table> + Send,
            <Vec<<DbRecord as DbInsert>::Post<'v>> as Insertable<DbRecord::Table>>::Values: Send,
            <DbRecord::Table as QuerySource>::FromClause: Send,

            // Insert bounds
            for<'a> InsertStatement<
                DbRecord::Table,
                <Vec<<DbRecord as DbInsert>::Post<'v>> as Insertable<DbRecord::Table>>::Values,
            >: LoadQuery<'query, Ctx::AsyncConnection, DbRecord::Raw>,
            for<'a> InsertStatement<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                    <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                >>::Values,
            >: ExecuteDsl<Ctx::AsyncConnection>,

            // Audit bounds
            DbRecord::Raw: Audit + Clone,
            <DbRecord::Raw as Audit>::AuditRow: Send,
            Vec<<DbRecord::Raw as Audit>::AuditRow>: Insertable<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>
                + UndecoratedInsertRecord<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>,
            <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table: Table + QueryId + Send,
            <<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table as QuerySource>::FromClause: Send,
            <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
            >>::Values: Send,
        {
            let db_post = db_post.into();
            Self::Constructor::can_create(ctx, [&db_post]).await?;
            let record = DbRecord::insert(ctx, [db_post])
                .await?
                .pop()
                .ok_or_else(Error::default)?;
            Self::Constructor::tx_cache_upsert(ctx, [&record]).await?;
            Ok(record)
        }
    }

    #[async_trait]
    pub trait AuthzServiceDbEntityRead<'query, 'v, DbRecord, Ctx>: AuthzServiceDbEntity<DbRecord = DbRecord>
    where
        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send + Sync,
    {
        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_read<F>(
            ctx: &Ctx,
            ids: impl IntoIterator<Item = impl Borrow<DbRecord::Id>> + Send,
        ) -> Result<Vec<DbRecord>, Error>
        where
            // temporary Uuid bound
            DbRecord::Id: Borrow<Uuid>,

            DbRecord: DbGet,

            DbRecord::Id: Clone + AsExpression<ht::SqlTypeOf<<DbRecord::Table as Table>::PrimaryKey>> + Sync,

            <DbRecord::Raw as TryInto<DbRecord>>::Error: Display + Send,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Id bounds
            DbRecord::Id: Debug + Send,
            for<'a> &'a DbRecord::Raw: Identifiable<Id = &'a DbRecord::Id>,
            <DbRecord::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            DbRecord::Table:
                FilterDsl<ht::EqAny<<DbRecord::Table as Table>::PrimaryKey, Vec<DbRecord::Id>>, Output = F>,
            F: IsNotDeleted<'query, Ctx::AsyncConnection, DbRecord::Raw, DbRecord::Raw>,
        {
            let ids = ids.into_iter().map(|id| id.borrow().clone()).collect::<Vec<_>>();
            if ids.is_empty() {
                return Ok(vec![]);
            }
            Self::can_read(ctx, ids.clone()).await?;
            Ok(DbRecord::get(ctx, ids).await?)
        }

        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_read_one<F>(
            ctx: &Ctx,
            id: impl Borrow<DbRecord::Id> + Debug + Send + Sync,
        ) -> Result<DbRecord, Error>
        where
            // temporary Uuid bound
            DbRecord::Id: Borrow<Uuid>,

            DbRecord: DbGet,

            DbRecord::Id: Clone + AsExpression<ht::SqlTypeOf<<DbRecord::Table as Table>::PrimaryKey>> + Sync,

            <DbRecord::Raw as TryInto<DbRecord>>::Error: Display + Send,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Id bounds
            DbRecord::Id: Debug + Send,
            for<'a> &'a DbRecord::Raw: Identifiable<Id = &'a DbRecord::Id>,
            <DbRecord::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            DbRecord::Table:
                FilterDsl<ht::EqAny<<DbRecord::Table as Table>::PrimaryKey, Vec<DbRecord::Id>>, Output = F>,
            F: IsNotDeleted<'query, Ctx::AsyncConnection, DbRecord::Raw, DbRecord::Raw>,
        {
            let id = id.borrow().clone();
            Self::can_read(ctx, [id.clone()]).await?;
            Ok(DbRecord::get_one(ctx, id).await?)
        }
    }

    #[async_trait]
    pub trait AuthzServiceDbEntitySoftDelete<'query, 'v: 'query, DbRecord, Ctx>:
        AuthzServiceDbEntity<DbRecord = DbRecord>
    where
        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send + Sync + DbSoftDelete,
        <DbRecord::Raw as TryInto<Self::DbRecord>>::Error: Display + Send,

        for<'a> &'a DbRecord: Into<Self::Constructor<'a>>,
    {
        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_delete<T, F>(
            ctx: &Ctx,
            ids: impl IntoIterator<Item = T> + Send + 'v,
        ) -> Result<Vec<Self::DbRecord>, Error>
        where
            Ctx: 'query,

            // temporary Uuid bounds
            T: Borrow<Uuid>,
            for<'a> &'a T: Borrow<Uuid>,
            Uuid: for<'a> Into<DbRecord::DeletePatchHelper<'a>>,

            T: Debug + Send + Sync,

            // Id bounds
            DbRecord::Id: Clone + Hash + Eq + Send + Sync,
            for<'a> &'a DbRecord::Raw: Identifiable<Id = &'a DbRecord::Id>,
            <DbRecord::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Changeset bounds
            <DbRecord::DeletePatch<'v> as AsChangeset>::Changeset: Send,
            for<'a> &'a DbRecord::DeletePatch<'v>:
                HasTable<Table = DbRecord::Table> + Identifiable<Id = &'a DbRecord::Id> + IntoUpdateTarget,
            for<'a> <&'a DbRecord::DeletePatch<'v> as IntoUpdateTarget>::WhereClause: Send,
            <DbRecord::Table as QuerySource>::FromClause: Send,

            // UpdateStatement bounds
            DbRecord::Table: FindDsl<DbRecord::Id>,
            ht::Find<DbRecord::Table, DbRecord::Id>: HasTable<Table = DbRecord::Table> + IntoUpdateTarget + Send,
            <ht::Find<DbRecord::Table, DbRecord::Id> as IntoUpdateTarget>::WhereClause: Send,
            ht::Update<ht::Find<DbRecord::Table, DbRecord::Id>, DbRecord::DeletePatch<'v>>:
                AsQuery + LoadQuery<'query, Ctx::AsyncConnection, DbRecord::Raw> + Send,

            // Filter bounds for records whose changesets do not include any changes
            DbRecord::Table:
                FilterDsl<ht::EqAny<<DbRecord::Table as Table>::PrimaryKey, Vec<DbRecord::Id>>, Output = F>,
            F: IsNotDeleted<'query, Ctx::AsyncConnection, DbRecord::Raw, DbRecord::Raw>,

            // Audit bounds
            DbRecord::Raw: Audit + Clone,
            <DbRecord::Raw as Audit>::AuditRow: Send,
            Vec<<DbRecord::Raw as Audit>::AuditRow>: Insertable<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>
                + UndecoratedInsertRecord<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>,
            <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table: Table + QueryId + Send,
            <<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table as QuerySource>::FromClause: Send,
            <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
            >>::Values: Send,
            InsertStatement<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                    <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                >>::Values,
            >: ExecuteDsl<Ctx::AsyncConnection>,
        {
            let ids = ids.into_iter().collect::<Vec<T>>();
            if ids.is_empty() {
                return Ok(vec![]);
            }

            let db_delete_patch_helpers = ids
                .iter()
                .map(|id| (*id.borrow()).into())
                .collect::<Vec<DbRecord::DeletePatchHelper<'_>>>();

            Self::can_delete(ctx, ids.iter()).await?;
            let records = DbRecord::delete(ctx, db_delete_patch_helpers).await?;
            Self::Constructor::tx_cache_mark_deleted(ctx, records.iter()).await?;
            Ok(records)
        }

        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_delete_one<T, F>(ctx: &Ctx, id: T) -> Result<Option<Self::DbRecord>, Error>
        where
            Ctx: 'query,

            // temporary Uuid bounds
            T: Borrow<Uuid>,
            for<'a> &'a T: Borrow<Uuid>,
            Uuid: for<'a> Into<DbRecord::DeletePatchHelper<'a>>,

            T: Debug + Send + Sync,

            // Id bounds
            DbRecord::Id: Clone + Hash + Eq + Send + Sync,
            for<'a> &'a DbRecord::Raw: Identifiable<Id = &'a DbRecord::Id>,
            <DbRecord::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Changeset bounds
            <DbRecord::DeletePatch<'v> as AsChangeset>::Changeset: Send,
            for<'a> &'a DbRecord::DeletePatch<'v>:
                HasTable<Table = DbRecord::Table> + Identifiable<Id = &'a DbRecord::Id> + IntoUpdateTarget,
            for<'a> <&'a DbRecord::DeletePatch<'v> as IntoUpdateTarget>::WhereClause: Send,
            <DbRecord::Table as QuerySource>::FromClause: Send,

            // UpdateStatement bounds
            DbRecord::Table: FindDsl<DbRecord::Id>,
            ht::Find<DbRecord::Table, DbRecord::Id>: HasTable<Table = DbRecord::Table> + IntoUpdateTarget + Send,
            <ht::Find<DbRecord::Table, DbRecord::Id> as IntoUpdateTarget>::WhereClause: Send,
            ht::Update<ht::Find<DbRecord::Table, DbRecord::Id>, DbRecord::DeletePatch<'v>>:
                AsQuery + LoadQuery<'query, Ctx::AsyncConnection, DbRecord::Raw> + Send,

            // Filter bounds for records whose changesets do not include any changes
            DbRecord::Table:
                FilterDsl<ht::EqAny<<DbRecord::Table as Table>::PrimaryKey, Vec<DbRecord::Id>>, Output = F>,
            F: IsNotDeleted<'query, Ctx::AsyncConnection, DbRecord::Raw, DbRecord::Raw>,

            // Audit bounds
            DbRecord::Raw: Audit + Clone,
            <DbRecord::Raw as Audit>::AuditRow: Send,
            Vec<<DbRecord::Raw as Audit>::AuditRow>: Insertable<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>
                + UndecoratedInsertRecord<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>,
            <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table: Table + QueryId + Send,
            <<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table as QuerySource>::FromClause: Send,
            <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
            >>::Values: Send,
            InsertStatement<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                    <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                >>::Values,
            >: ExecuteDsl<Ctx::AsyncConnection>,
        {
            let id = *id.borrow();
            Self::can_delete(ctx, [id]).await?;
            let mut records = DbRecord::delete(ctx, [id.into()]).await?;
            Self::Constructor::tx_cache_mark_deleted(ctx, records.iter()).await?;
            Ok(records.pop())
        }
    }

    #[async_trait]
    pub trait AuthzServiceDbEntityUpdate<'query, 'v: 'query, DbRecord, Ctx>:
        AuthzServiceDbEntity<DbRecord = DbRecord>
    where
        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send + Sync + for<'a> DbUpdate<PatchHelper<'a> = Self::DbPatch<'a>>,
        <DbRecord::Raw as TryInto<Self::DbRecord>>::Error: Display + Send,

        for<'a> &'a Self::DbPatch<'v>: Into<Self::Patch<'a>>,
        for<'a> &'a DbRecord: Into<Self::Constructor<'a>>,
    {
        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_update<F>(
            ctx: &Ctx,
            db_patches: impl IntoIterator<Item = impl Into<Self::DbPatch<'v>>> + Send + 'v,
        ) -> Result<Vec<DbRecord>, Error>
        where
            Self: 'v,
            Ctx: 'query,

            // Id bounds
            DbRecord::Id: Clone + Hash + Eq + Send + Sync,
            for<'a> &'a DbRecord::Raw: Identifiable<Id = &'a DbRecord::Id>,
            <DbRecord::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Changeset bounds
            <DbRecord::Patch<'v> as AsChangeset>::Changeset: Send,
            for<'a> &'a DbRecord::Patch<'v>:
                HasTable<Table = DbRecord::Table> + Identifiable<Id = &'a DbRecord::Id> + IntoUpdateTarget,
            for<'a> <&'a DbRecord::Patch<'v> as IntoUpdateTarget>::WhereClause: Send,
            <DbRecord::Table as QuerySource>::FromClause: Send,

            // UpdateStatement bounds
            DbRecord::Table: FindDsl<DbRecord::Id>,
            ht::Find<DbRecord::Table, DbRecord::Id>: HasTable<Table = DbRecord::Table> + IntoUpdateTarget + Send,
            <ht::Find<DbRecord::Table, DbRecord::Id> as IntoUpdateTarget>::WhereClause: Send,
            ht::Update<ht::Find<DbRecord::Table, DbRecord::Id>, DbRecord::Patch<'v>>:
                AsQuery + LoadQuery<'query, Ctx::AsyncConnection, DbRecord::Raw> + Send,

            // Filter bounds for records whose changesets do not include any changes
            DbRecord::Table:
                FilterDsl<ht::EqAny<<DbRecord::Table as Table>::PrimaryKey, Vec<DbRecord::Id>>, Output = F>,
            F: IsNotDeleted<'query, Ctx::AsyncConnection, DbRecord::Raw, DbRecord::Raw>,

            // Audit bounds
            DbRecord::Raw: Audit + Clone,
            <DbRecord::Raw as Audit>::AuditRow: Send,
            Vec<<DbRecord::Raw as Audit>::AuditRow>: Insertable<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>
                + UndecoratedInsertRecord<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>,
            <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table: Table + QueryId + Send,
            <<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table as QuerySource>::FromClause: Send,
            <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
            >>::Values: Send,
            InsertStatement<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                    <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                >>::Values,
            >: ExecuteDsl<Ctx::AsyncConnection>,
        {
            let db_patches = db_patches.into_iter().map(Into::into).collect::<Vec<_>>();
            if db_patches.is_empty() {
                return Ok(vec![]);
            }
            Self::can_update(ctx, db_patches.iter()).await?;
            let records = DbRecord::update(ctx, db_patches).await?;
            Self::Constructor::tx_cache_upsert(ctx, records.iter()).await?;
            Ok(records)
        }

        #[framed]
        #[instrument(err(Debug), skip_all)]
        async fn try_update_one<F>(ctx: &Ctx, db_patch: impl Into<Self::DbPatch<'v>> + Send) -> Result<DbRecord, Error>
        where
            Self: 'v,
            Ctx: 'query,

            // Id bounds
            DbRecord::Id: Clone + Hash + Eq + Send + Sync,
            for<'a> &'a DbRecord::Raw: Identifiable<Id = &'a DbRecord::Id>,
            <DbRecord::Table as Table>::PrimaryKey: Expression + ExpressionMethods,
            <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,

            // Changeset bounds
            <DbRecord::Patch<'v> as AsChangeset>::Changeset: Send,
            for<'a> &'a DbRecord::Patch<'v>:
                HasTable<Table = DbRecord::Table> + Identifiable<Id = &'a DbRecord::Id> + IntoUpdateTarget,
            for<'a> <&'a DbRecord::Patch<'v> as IntoUpdateTarget>::WhereClause: Send,
            <DbRecord::Table as QuerySource>::FromClause: Send,

            // UpdateStatement bounds
            DbRecord::Table: FindDsl<DbRecord::Id>,
            ht::Find<DbRecord::Table, DbRecord::Id>: HasTable<Table = DbRecord::Table> + IntoUpdateTarget + Send,
            <ht::Find<DbRecord::Table, DbRecord::Id> as IntoUpdateTarget>::WhereClause: Send,
            ht::Update<ht::Find<DbRecord::Table, DbRecord::Id>, DbRecord::Patch<'v>>:
                AsQuery + LoadQuery<'query, Ctx::AsyncConnection, DbRecord::Raw> + Send,

            // Filter bounds for records whose changesets do not include any changes
            DbRecord::Table:
                FilterDsl<ht::EqAny<<DbRecord::Table as Table>::PrimaryKey, Vec<DbRecord::Id>>, Output = F>,
            F: IsNotDeleted<'query, Ctx::AsyncConnection, DbRecord::Raw, DbRecord::Raw>,

            // Audit bounds
            DbRecord::Raw: Audit + Clone,
            <DbRecord::Raw as Audit>::AuditRow: Send,
            Vec<<DbRecord::Raw as Audit>::AuditRow>: Insertable<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>
                + UndecoratedInsertRecord<<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table>,
            <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table: Table + QueryId + Send,
            <<<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table as QuerySource>::FromClause: Send,
            <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
            >>::Values: Send,
            InsertStatement<
                <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                <Vec<<DbRecord::Raw as Audit>::AuditRow> as Insertable<
                    <<DbRecord::Raw as Audit>::AuditTable as HasTable>::Table,
                >>::Values,
            >: ExecuteDsl<Ctx::AsyncConnection>,
        {
            let db_patch = db_patch.into();
            Self::can_update(ctx, [&db_patch]).await?;
            let record = DbRecord::update_one(ctx, db_patch).await?;
            Self::Constructor::tx_cache_upsert(ctx, [&record]).await?;
            Ok(record)
        }
    }

    #[async_trait]
    impl<'query, 'v: 'query, Ctx, DbRecord, T> AuthzServiceDbEntityCreate<'query, 'v, DbRecord, Ctx> for T
    where
        Self: AuthzServiceDbEntity<DbRecord = DbRecord>,

        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send
            + Sync
            + for<'a> DbInsert<PostHelper<'a> = <Self::Constructor<'a> as AuthzServiceDbEntity>::DbPost<'a>>,
        <DbRecord::Raw as TryInto<DbRecord>>::Error: Display + Send,

        for<'a> &'a Self::DbPost<'v>: Into<Self::Constructor<'a>>,
        for<'a> &'a DbRecord: Into<Self::Constructor<'a>>,
    {
    }

    #[async_trait]
    impl<'query, 'v: 'query, Ctx, DbRecord, T> AuthzServiceDbEntityRead<'query, 'v, DbRecord, Ctx> for T
    where
        Self: AuthzServiceDbEntity<DbRecord = DbRecord>,

        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send + Sync + DbGet<Id = Uuid>,
        <DbRecord::Raw as TryInto<DbRecord>>::Error: Display + Send,

        Uuid: diesel::expression::AsExpression<ht::SqlTypeOf<<DbRecord::Table as Table>::PrimaryKey>>,
        <<DbRecord::Table as Table>::PrimaryKey as Expression>::SqlType: SqlType,
    {
    }

    #[async_trait]
    impl<'query, 'v: 'query, Ctx, DbRecord, T> AuthzServiceDbEntitySoftDelete<'query, 'v, DbRecord, Ctx> for T
    where
        Self: AuthzServiceDbEntity<DbRecord = DbRecord>,

        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send + Sync + DbSoftDelete,
        <DbRecord::Raw as TryInto<Self::DbRecord>>::Error: Display + Send,

        for<'a> &'a DbRecord: Into<Self::Constructor<'a>>,
    {
    }

    #[async_trait]
    impl<'query, 'v: 'query, Ctx, DbRecord, T> AuthzServiceDbEntityUpdate<'query, 'v, DbRecord, Ctx> for T
    where
        Self: AuthzServiceDbEntity<DbRecord = DbRecord>,

        Ctx: OPAContext + OPATxCacheContext + _Db<AsyncConnection = Self::AsyncConnection, Backend = Self::Backend>,
        <Ctx as OPATxCacheContext>::TxCacheClient: Send,

        DbRecord: Send + Sync + for<'a> DbUpdate<PatchHelper<'a> = Self::DbPatch<'a>>,
        <DbRecord::Raw as TryInto<Self::DbRecord>>::Error: Display + Send,

        for<'a> &'a Self::DbPatch<'v>: Into<Self::Patch<'a>>,
        for<'a> &'a DbRecord: Into<Self::Constructor<'a>>,
    {
    }
}
