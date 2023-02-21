#![cfg_attr(all(doc, CHANNEL_NIGHTLY), feature(doc_auto_cfg))]

#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde_with;

mod decision_makers;
mod storage_backends;

#[cfg(feature = "extra-traits")]
mod extra_traits;

use ::serde::{Deserialize, Serialize};
use derive_getters::{Dissolve, DissolveMut, DissolveRef, Getters};
use std::fmt::Debug;
use std::marker::PhantomData;
use typed_builder::TypedBuilder;

/// Compile time information about an action.
pub trait ActionType {
    /// The type name of this action.
    const TYPE: &'static str;
}

/// An action which requires authorization.
#[doc(hidden)]
#[async_trait]
pub trait TryAct<C, DM, Subject, Object, Input, Context>:
    Into<Event<Subject, Self::Action, Object, Input, Context>>
where
    C: ?Sized + StorageClient + Send + Sync,
    DM: ?Sized + DecisionMaker<Subject, Self::Action, Object, Input, Context> + Sync,
    Subject: Send + Sync,
    Object: ?Sized + Send + ObjectType + Sync,
    Input: Send + Sync,
    Context: Send + Sync,
{
    /// Action to be authorized and performed.
    type Action: ActionType + StorageAction<C, Input> + Send + Sync;

    async fn try_act(
        self,
        decision_maker: &DM,
        storage_client: &C,
    ) -> Result<
        <Self::Action as StorageAction<C, Input>>::Ok,
        ActionError<
            <DM as DecisionMaker<Subject, Self::Action, Object, Input, Context>>::Error,
            <Self::Action as StorageAction<C, Input>>::Error,
        >,
    >
    where
        C: 'async_trait,
        Input: 'async_trait,
    {
        let event = self.into();
        decision_maker.can_act(&event).await.map_err(ActionError::authz)?;
        Ok(Self::Action::act(storage_client, event.input)
            .await
            .map_err(ActionError::storage)?)
    }
}

#[async_trait]
impl<C, DM, Subject, A, Object, Input, Context> TryAct<C, DM, Subject, Object, Input, Context>
    for Event<Subject, A, Object, Input, Context>
where
    C: ?Sized + StorageClient + Send + Sync,
    DM: ?Sized + DecisionMaker<Subject, A, Object, Input, Context> + Sync,
    Subject: Send + Sync,
    Object: ?Sized + ObjectType + Send + Sync,
    Input: Send + Sync,
    Context: Send + Sync,

    A: ActionType + StorageAction<C, Input> + Send + Sync,
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

/// The unit of work in an authorization query, which will either be accepted or rejected by a decision maker.
#[skip_serializing_none]
#[derive(Clone, Deserialize, Dissolve, DissolveMut, DissolveRef, Eq, Getters, PartialEq, Serialize, TypedBuilder)]
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

/// Represents a unique backend on which actions are performed. An example would be postgres,
/// mysql, or your own custom implementation of an API.
pub trait StorageBackend {}

/// A client for communicating with a storage backend. Typically this should be implemented for
/// connection or client implementations for that backend, e.g. [`diesel_async::AsyncPgConnection`](https://docs.rs/diesel-async/latest/diesel_async/pg/struct.AsyncPgConnection.html).
pub trait StorageClient {
    /// The backend this client will act upon.
    type Backend: StorageBackend;
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
    type StorageObject: StorageObject<Backend> + Send + Sync;
}

/// An object's representation specific to a particular backend. Often this will only be
/// implemented once unless an object is stored in multipled different backends.
pub trait StorageObject<Backend> {}

/// Encapsulates the actual performance of a specific action given a suitable client and input.
#[async_trait]
pub trait StorageAction<Client: ?Sized, Input>
where
    Client: StorageClient + Send,
{
    type Ok: Send;
    type Error: Debug + Send;

    /// Carries out the intended action in the storage backend of `Client`.
    async fn act(client: &Client, input: Input) -> Result<Self::Ok, Self::Error>
    where
        Client: 'async_trait,
        Input: 'async_trait;
}

/// Represents a policy decision point (could be astracted over an in-process memory, a remote api,
/// etc.) which is capable of making authorization decisions using the provided [`Event`].
#[async_trait]
pub trait DecisionMaker<Subject, Action, Object, Input, Context>
where
    Event<Subject, Action, Object, Input, Context>: Send + Sync,
    Action: ?Sized,
    Object: ?Sized,
{
    type Ok: Debug + Send;
    type Error: Debug + Send;
    async fn can_act(&self, event: &Event<Subject, Action, Object, Input, Context>) -> Result<Self::Ok, Self::Error>;
}

#[async_trait]
impl<Subject, Action, Object, Input, Context, T> DecisionMaker<Subject, Action, Object, Input, Context> for &T
where
    Event<Subject, Action, Object, Input, Context>: Send + Sync,
    Subject: Serialize,
    Action: ?Sized + ActionType,
    Object: ?Sized + ObjectType,
    Input: Serialize,
    Context: Serialize,
    T: ?Sized + DecisionMaker<Subject, Action, Object, Input, Context> + Send + Sync,
{
    type Ok = T::Ok;
    type Error = T::Error;
    async fn can_act(&self, event: &Event<Subject, Action, Object, Input, Context>) -> Result<Self::Ok, Self::Error> {
        (**self).can_act(event).await
    }
}

/// Wraps only the authorization decision making
/// component of an action requiring authorization.
pub trait AuthorizationContext<DM> {
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
    fn decision_maker(&self) -> &DM;
}

/// Wraps all components of an action requiring authorization
/// which can be expected to be carried around in an
/// application context.
pub trait TryActionContext<DM, SC>: AuthorizationContext<DM> {
    fn storage_client(&self) -> &SC;
}

/// Represents the possible sources of error when performing
/// an action which requires authorization.
#[derive(AsVariant, AsVariantMut, Clone, Copy, Debug, Error, IsVariant, Unwrap)]
pub enum ActionError<E1, E2> {
    /// Wraps an error returned from a [`DecisionMaker`] when the subject is either not authorized to
    /// perform an action or some other issue occurs while communicating with the
    /// [`DecisionMaker`].
    Authz(E1),
    /// Wraps an error returned from a [`StorageClient`] when the subject has been authorized to
    /// perform an action but there an error occurs while actually performing the error. Examples
    /// of this include network errors while communicating with an api or database, unique
    /// constraint violations raised by a database, etc.
    Storage(E2),
}

impl<E1, E2> ActionError<E1, E2> {
    pub fn authz(err: E1) -> Self {
        Self::Authz(err)
    }
    pub fn storage(err: E2) -> Self {
        Self::Storage(err)
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
pub mod serde {
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
