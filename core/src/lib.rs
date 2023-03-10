#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate cfg_if;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde_with;

#[cfg(feature = "tracing")]
#[macro_use]
extern crate tracing;

mod authz_engines;
mod data_sources;

/// Helper traits for implementing a policy information point.
#[cfg(feature = "policy-information-point")]
pub mod policy_information_point;

/// Implementations of common transaction cache clients.
pub mod transaction_caches;

#[cfg(feature = "extra-traits")]
mod extra_traits;

use ::authzen_data_sources::*;
use ::derive_getters::{Dissolve, Getters};
use ::serde::{de::DeserializeOwned, Deserialize, Serialize};
use ::std::borrow::Borrow;
use ::std::collections::HashMap;
use ::std::fmt::Debug;
use ::std::future::Future;
use ::std::hash::Hash;
use ::std::marker::PhantomData;
use ::std::pin::Pin;
use ::typed_builder::TypedBuilder;

/// Compile time information about an action.
pub trait ActionType {
    /// The type name of this action.
    const TYPE: &'static str;
}

/// An action which requires authorization.
#[doc(hidden)]
#[async_trait]
pub trait TryAct<DS, AE, Subject, Object, Input, Context, TC>:
    Into<Event<Subject, Self::Action, Object, Input, Context>>
where
    DS: ?Sized + DataSource + Send + Sync,
    AE: ?Sized + AuthzEngine<Subject, Self::Action, Object, Input, Context, DS::TransactionId> + Sync,
    Subject: Send + Sync,
    Object: ?Sized + Send + ObjectType + Sync,
    Input: Send + Sync,
    Context: Send + Sync,
    TC: Send + Sync + TransactionCache + TransactionCacheAction<Self::Action, DS, Input>,
{
    /// Action to be authorized and performed.
    type Action: ActionType + StorageAction<DS, Input> + Send + Sync;

    async fn try_act(
        self,
        authz_engine: &AE,
        data_source: &DS,
        transaction_cache: &TC,
    ) -> Result<
        <Self::Action as StorageAction<DS, Input>>::Ok,
        ActionError<
            <AE as AuthzEngine<Subject, Self::Action, Object, Input, Context, DS::TransactionId>>::Error,
            <Self::Action as StorageAction<DS, Input>>::Error,
            TC::Error,
        >,
    >
    where
        AE: 'async_trait,
        DS: 'async_trait,
        TC: 'async_trait,
        Input: 'async_trait,
    {
        let event = self.into();
        authz_engine
            .can_act(event.subject, &event.input, event.context, data_source.transaction_id())
            .await
            .map_err(ActionError::authz)?;
        let ok = Self::Action::act(data_source, event.input)
            .await
            .map_err(ActionError::DataSource)?;
        transaction_cache
            .handle_success(data_source, &ok)
            .await
            .map_err(ActionError::transaction_cache)?;
        Ok(ok)
    }
}

#[async_trait]
impl<DS, AE, Subject, A, Object, Input, Context, TC> TryAct<DS, AE, Subject, Object, Input, Context, TC>
    for Event<Subject, A, Object, Input, Context>
where
    DS: ?Sized + DataSource + Send + Sync,
    AE: ?Sized + AuthzEngine<Subject, A, Object, Input, Context, DS::TransactionId> + Sync,
    Subject: Send + Sync,
    Object: ?Sized + ObjectType + Send + Sync,
    Input: Send + Sync,
    Context: Send + Sync,
    TC: Send + Sync + TransactionCache + TransactionCacheAction<A, DS, Input>,

    A: ActionType + StorageAction<DS, Input> + Send + Sync,
{
    type Action = A;
}

/// Compile time information about an object.
pub trait ObjectType {
    /// The service this object belongs to.
    const SERVICE: &'static str;
    /// The type name of this object.
    const TYPE: &'static str;
}

/// The unit of work in an authorization query, which will either be accepted or rejected by an authorization engine.
#[skip_serializing_none]
#[derive(Clone, Deserialize, Dissolve, Eq, Getters, PartialEq, Serialize, TypedBuilder)]
#[serde(bound(
    serialize = "Subject: Serialize, Action: ActionType, Object: ObjectType, Input: Serialize, Context: Serialize",
    deserialize = "Subject: Deserialize<'de>, Action: ActionType, Object: ObjectType, Input: Deserialize<'de>, Context: Deserialize<'de>",
))]
pub struct Event<Subject, Action: ?Sized, Object: ?Sized, Input, Context = ()> {
    /// the entity performing the action; typically links back to a user / account;
    /// could be represented with an account id, a jwt, or even more information if necessary
    pub subject: Subject,
    /// the action which the subject is trying to perform; typically this will be a
    /// struct dervied from the [`action`](macro.action.html) macro (e.g. `Create<MyObject>`)
    #[getter(skip)]
    #[serde(with = "serde::action")]
    pub action: PhantomData<Action>,
    /// the object wich the subject is attempting to act upon; note that this parameter
    /// is only used as a specification of the object's type and service it belongs to
    #[getter(skip)]
    #[serde(with = "serde::object")]
    pub object: PhantomData<Object>,
    /// the data provided which uniquely identifies the object(s) being acted upon; the
    /// type used here can be anything which is recognized as valid input for the specific action (see
    /// [`StorageAction`] to see how actions specify their acceptable inputs)
    pub input: Input,
    /// any additional data which should or must be provided in order to fulfill the
    /// authorization decision; use this for any data which is not referring to objects being acted on
    pub context: Context,
}

impl<Subject, Action, Object, Input, Context> Debug for Event<Subject, Action, Object, Input, Context>
where
    Subject: Debug,
    Action: ?Sized + ActionType,
    Object: ?Sized + ObjectType,
    Input: Debug,
    Context: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Event")
            .field("subject", &self.subject)
            .field("action", &Action::TYPE)
            .field("object", &fmt::DebugObject(self.object))
            .field("subject", &self.subject)
            .finish()
    }
}

impl<Subject, A, Objects, Context> ActionType for Event<Subject, A, Objects, Context>
where
    A: ActionType,
{
    const TYPE: &'static str = A::TYPE;
}

/// Connects an object with its backend representation for a specific backend.
pub trait AsStorage<Backend>: ObjectType {
    /// A type method of producing this type with a narrower lifetime. If Self has no lifetime
    /// parameters, this type should be `Self`. Otherwise say you have an object with one lifetime
    /// parameter, `MyObject<'a>`, then a concrete `Constructor` implementation would look like
    /// ```rs
    /// impl<'a> AsStorage<MyBackend> for MyObject<'a> {
    ///     type Constructor<'v> = MyObject<'v>;
    /// }
    /// ```
    type Constructor<'a>: AsStorage<Backend>;
    /// This object's storage representation for the specified `Backend`.
    type StorageObject: DeserializeOwned + Identifiable + StorageObject<Backend> + Send + Serialize + Sync;
}

/// An object's representation specific to a particular backend. Often this will only be
/// implemented once unless an object is stored in multipled different backends.
pub trait StorageObject<Backend> {}

/// Encapsulates the actual performance of a specific action given a suitable client and input.
#[async_trait]
pub trait StorageAction<Client: ?Sized, Input>
where
    Client: DataSource + Send,
{
    type Ok: Send + Sync;
    type Error: Debug + Send + StorageError;

    /// Carries out the intended action in the data source of `Client`.
    async fn act(client: &Client, input: Input) -> Result<Self::Ok, Self::Error>
    where
        Client: 'async_trait,
        Input: 'async_trait;
}

pub trait StorageError {
    fn not_found() -> Self;
}

/// Represents a policy decision point (could be astracted over an in-process memory, a remote api,
/// etc.) which is capable of making authorization decisions using the provided [`Event`].
#[async_trait]
pub trait AuthzEngine<Subject, Action, Object, Input, Context, TransactionId>
where
    Event<Subject, Action, Object, Input, Context>: Send + Sync,
    Action: ?Sized,
    Object: ?Sized,
{
    type Ok: Debug + Send;
    type Error: Debug + Send;
    async fn can_act(
        &self,
        subject: Subject,
        input: &Input,
        context: Context,
        transaction_id: Option<TransactionId>,
    ) -> Result<Self::Ok, Self::Error>
    where
        Subject: 'async_trait,
        Action: 'async_trait,
        Object: 'async_trait,
        Input: 'async_trait,
        Context: 'async_trait,
        TransactionId: 'async_trait;
}

#[async_trait]
impl<Subject, Action, Object, Input, Context, TransactionId, T>
    AuthzEngine<Subject, Action, Object, Input, Context, TransactionId> for &T
where
    Event<Subject, Action, Object, Input, Context>: Send + Sync,
    Subject: Send + Serialize,
    Action: ?Sized + ActionType + Sync,
    Object: ?Sized + ObjectType + Sync,
    Input: Serialize + Sync,
    Context: Send + Serialize,
    TransactionId: Send,
    T: ?Sized + AuthzEngine<Subject, Action, Object, Input, Context, TransactionId> + Send + Sync,
{
    type Ok = T::Ok;
    type Error = T::Error;
    async fn can_act(
        &self,
        subject: Subject,
        input: &Input,
        context: Context,
        transaction_id: Option<TransactionId>,
    ) -> Result<Self::Ok, Self::Error>
    where
        Subject: 'async_trait,
        Action: 'async_trait,
        Object: 'async_trait,
        Input: 'async_trait,
        Context: 'async_trait,
        TransactionId: 'async_trait,
    {
        <T as AuthzEngine<Subject, Action, Object, Input, Context, TransactionId>>::can_act(
            *self,
            subject,
            input,
            context,
            transaction_id,
        )
        .await
    }
}

pub trait Identifiable {
    type Id: Clone + DeserializeOwned + Eq + Hash + Send + Serialize + Sync + 'static;
    fn id(&self) -> &Self::Id;
}

pub trait TransactionCache {
    type Error: Debug + Send;

    #[allow(clippy::type_complexity)]
    fn get_entities<'life0, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
    ) -> Pin<Box<dyn Future<Output = Result<HashMap<T::Id, TxCacheEntity<T, T::Id>>, Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize;

    #[allow(clippy::type_complexity)]
    fn get_by_ids<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        ids: &'life1 [T::Id],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<T>, Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize;

    fn upsert<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Clone + Send + Serialize;

    fn mark_deleted<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Clone + Send + Serialize;
}

impl TransactionCache for () {
    type Error = std::convert::Infallible;

    fn get_entities<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        _: TransactionId,
    ) -> Pin<Box<dyn Future<Output = Result<HashMap<T::Id, TxCacheEntity<T, T::Id>>, Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize,
    {
        Box::pin(async { Ok(Default::default()) })
    }

    fn get_by_ids<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        _: TransactionId,
        _: &'life1 [T::Id],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<T>, Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize,
    {
        Box::pin(async { Ok(Default::default()) })
    }

    fn upsert<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        _: TransactionId,
        _: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Send + Serialize,
    {
        Box::pin(async { Ok(()) })
    }

    fn mark_deleted<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        _: TransactionId,
        _: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Send + Serialize,
    {
        Box::pin(async { Ok(()) })
    }
}

impl<TC: TransactionCache> TransactionCache for &TC {
    type Error = TC::Error;

    fn get_entities<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
    ) -> Pin<Box<dyn Future<Output = Result<HashMap<T::Id, TxCacheEntity<T, T::Id>>, Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize,
    {
        (*self).get_entities::<O, T, TransactionId>(transaction_id)
    }

    fn get_by_ids<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        ids: &'life1 [T::Id],
    ) -> Pin<Box<dyn Future<Output = Result<Vec<T>, Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: DeserializeOwned + Identifiable + Send,
        T::Id: Clone,
        TransactionId: Send + Serialize,
    {
        (*self).get_by_ids::<O, T, TransactionId>(transaction_id, ids)
    }

    fn upsert<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Clone + Send + Serialize,
    {
        (*self).upsert::<O, T, TransactionId>(transaction_id, entities)
    }

    fn mark_deleted<'life0, 'life1, 'async_trait, O, T, TransactionId>(
        &'life0 self,
        transaction_id: TransactionId,
        entities: impl IntoIterator<Item = impl Borrow<T> + Send> + Send + 'life1,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        TransactionId: 'async_trait,

        O: ?Sized + ObjectType,
        T: Identifiable + Serialize,
        TransactionId: Clone + Send + Serialize,
    {
        (*self).mark_deleted::<O, T, TransactionId>(transaction_id, entities)
    }
}

pub trait TransactionCacheAction<A, DS, I>: TransactionCache
where
    DS: ?Sized + DataSource + Send + Sync,
    A: StorageAction<DS, I> + Send,
{
    fn handle_success<'life0, 'life1, 'life2, 'async_trait>(
        &'life0 self,
        data_source: &'life1 DS,
        ok: &'life2 A::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), Self::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
        'life2: 'async_trait,
    {
        if let Some(transaction_id) = data_source.transaction_id() {
            self.manage_cache(transaction_id, ok)
        } else {
            Box::pin(async { Ok(()) })
        }
    }

    fn manage_cache<'life0, 'life1, 'async_trait>(
        &'life0 self,
        transaction_id: DS::TransactionId,
        ok: &'life1 A::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        DS::TransactionId: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait;
}

impl<A, DS, I> TransactionCacheAction<A, DS, I> for ()
where
    DS: ?Sized + DataSource + Send + Sync,
    A: StorageAction<DS, I> + Send,
{
    fn manage_cache<'life0, 'life1, 'async_trait>(
        &'life0 self,
        _: DS::TransactionId,
        _: &'life1 A::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        DS::TransactionId: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        Box::pin(async { Ok(()) })
    }
}

impl<A, DS, I, TCA> TransactionCacheAction<A, DS, I> for &TCA
where
    DS: ?Sized + DataSource + Send + Sync,
    A: StorageAction<DS, I> + Send,
    TCA: TransactionCacheAction<A, DS, I> + Sync,
{
    fn manage_cache<'life0, 'life1, 'async_trait>(
        &'life0 self,
        transaction_id: DS::TransactionId,
        ok: &'life1 A::Ok,
    ) -> Pin<Box<dyn Future<Output = Result<(), <Self as TransactionCache>::Error>> + Send + 'async_trait>>
    where
        Self: Sync,
        Self: 'async_trait,
        DS::TransactionId: 'async_trait,
        'life0: 'async_trait,
        'life1: 'async_trait,
    {
        (*self).manage_cache(transaction_id, ok)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct TxCacheEntity<T, Id> {
    pub exists: bool,
    pub id: Id,
    pub value: T,
}

impl<T, Id> TxCacheEntity<T, Id> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> TxCacheEntity<U, Id> {
        TxCacheEntity {
            exists: self.exists,
            id: self.id,
            value: f(self.value),
        }
    }
    pub fn try_map<U, E>(self, f: impl FnOnce(T) -> Result<U, E>) -> Result<TxCacheEntity<U, Id>, E> {
        Ok(TxCacheEntity {
            exists: self.exists,
            id: self.id,
            value: f(self.value)?,
        })
    }
}

/// Wraps all components of an action requiring authorization
/// which can be expected to be carried around in an
/// application context.
pub trait AuthorizationContext<AE, DS, TC> {
    /// decision context
    type Context<'a>: Send + Sync
    where
        Self: 'a;
    /// decision subject
    type Subject<'a>: Send + Sync
    where
        Self: 'a;

    fn context(&self) -> Self::Context<'_>;
    fn subject(&self) -> Self::Subject<'_>;
    fn authz_engine(&self) -> &AE;
    fn data_source(&self) -> &DS;
    fn transaction_cache(&self) -> &TC;
}

/// Represents the possible sources of error when performing
/// an action which requires authorization.
#[derive(Clone, Copy, Debug, Error, IsVariant, Unwrap)]
pub enum ActionError<E1, E2, E3> {
    /// Wraps an error returned from a [`AuthzEngine`] when the subject is either not authorized to
    /// perform an action or some other issue occurs while communicating with the
    /// [`AuthzEngine`].
    Authz(E1),
    /// Wraps an error returned from a [`DataSource`] when the subject has been authorized to
    /// perform an action but there an error occurs while actually performing the error. Examples
    /// of this include network errors while communicating with an api or database, unique
    /// constraint violations raised by a database, etc.
    DataSource(E2),
    /// Wraps an error returned from a [`TransactionCache`] when updating the transaction
    /// cache after a successful performance of the action.
    TransactionCache(E3),
}

impl<E1, E2, E3> ActionError<E1, E2, E3> {
    pub fn authz(err: E1) -> Self {
        Self::Authz(err)
    }
    pub fn data_source(err: E2) -> Self {
        Self::DataSource(err)
    }
    pub fn transaction_cache(err: E3) -> Self {
        Self::TransactionCache(err)
    }
}

/// Standard actions which are useful across many applications.
///
/// Custom actions can be generated using the [`action`](authzen_proc_macros::action) macro.
pub mod actions {
    use super::*;
    use authzen_proc_macros::*;

    action!(__authzen_internal, Create);
    action!(__authzen_internal, Delete);
    action!(__authzen_internal, Read);
    action!(__authzen_internal, Update);
}

#[doc(hidden)]
mod serde {
    use super::*;

    pub mod action {
        use super::*;
        use ::serde::de::{self, Error, Visitor};
        use ::serde::{Deserializer, Serializer};
        use std::fmt;

        pub fn serialize<T, S>(_: &PhantomData<T>, ser: S) -> Result<S::Ok, S::Error>
        where
            T: ?Sized + ActionType,
            S: Serializer,
        {
            ser.serialize_str(T::TYPE)
        }

        pub fn deserialize<'de, T, D>(de: D) -> Result<PhantomData<T>, D::Error>
        where
            T: ?Sized + ActionType,
            D: Deserializer<'de>,
        {
            let action_type = de.deserialize_str(StrVisitor)?;
            if action_type == T::TYPE {
                Ok(PhantomData::default())
            } else {
                return Err(D::Error::custom(format!(
                    "expected action type `{}`, found `{action_type}`",
                    T::TYPE
                )));
            }
        }

        struct StrVisitor;

        impl<'de> Visitor<'de> for StrVisitor {
            type Value = &'de str;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string")
            }

            fn visit_borrowed_str<E>(self, value: &'de str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(value)
            }
        }
    }

    pub mod object {
        use super::*;
        use ::serde::de::{Error, MapAccess, Visitor};
        use ::serde::ser::SerializeMap;
        use ::serde::{Deserializer, Serializer};
        use std::borrow::Cow;
        use std::fmt;

        pub fn serialize<T, S>(_: &PhantomData<T>, ser: S) -> Result<S::Ok, S::Error>
        where
            T: ?Sized + ObjectType,
            S: Serializer,
        {
            let mut map = ser.serialize_map(Some(2))?;
            map.serialize_entry("service", T::SERVICE)?;
            map.serialize_entry("type", T::TYPE)?;
            map.end()
        }

        #[derive(Debug, Deserialize)]
        struct ObjectTypeMap<'a> {
            service: Cow<'a, str>,
            #[serde(rename = "type")]
            ty: Cow<'a, str>,
        }

        pub fn deserialize<'de, T, D>(de: D) -> Result<PhantomData<T>, D::Error>
        where
            T: ?Sized + ObjectType,
            D: Deserializer<'de>,
        {
            let ObjectTypeMap { service, ty } = de.deserialize_struct("", &["service", "type"], StructVisitor)?;

            let service_err_msg = || format!("expected object service `{}`, found `{service}`", T::SERVICE);
            let ty_err_msg = || format!("expected object type `{}`, found `{ty}`", T::TYPE);

            match (service == T::SERVICE, ty == T::TYPE) {
                (true, true) => Ok(PhantomData::default()),
                (true, false) => Err(D::Error::custom(ty_err_msg())),
                (false, true) => Err(D::Error::custom(service_err_msg())),
                (false, false) => Err(D::Error::custom(format!("{}; {}", service_err_msg(), ty_err_msg()))),
            }
        }

        struct StructVisitor;

        impl<'de> Visitor<'de> for StructVisitor {
            type Value = ObjectTypeMap<'de>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a map with values `service` and `type`")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut service = None::<Cow<'de, str>>;
                let mut ty = None::<Cow<'de, str>>;

                // While there are entries remaining in the input, add them
                // into our map.
                while let Some((key, value)) = access.next_entry::<&'de str, Cow<'de, str>>()? {
                    match key {
                        "service" => service = Some(value),
                        "type" => ty = Some(value),
                        _ => {}
                    };
                }
                match (service, ty) {
                    (Some(service), Some(ty)) => Ok(ObjectTypeMap { service, ty }),
                    _ => Err(A::Error::custom("could not deserialize object details")),
                }
            }
        }
    }
}

#[doc(hidden)]
pub mod fmt {
    use super::*;

    #[derive(Clone, Copy, From, Into)]
    pub struct DebugObject<O: ?Sized>(pub PhantomData<O>);

    impl<O: ?Sized + ObjectType> Debug for DebugObject<O> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_map()
                .entry(&"service", &O::SERVICE)
                .entry(&"type", &O::TYPE)
                .finish()
        }
    }
}
