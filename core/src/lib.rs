#[macro_use]
extern crate async_trait;
#[macro_use]
extern crate derivative;
#[macro_use]
extern crate derive_more;
#[macro_use]
extern crate serde_with;

pub mod storage;

use ::serde::{Deserialize, Serialize};
use derive_getters::{Dissolve, DissolveMut, DissolveRef, Getters};
use futures::future::{BoxFuture, FutureExt};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use typed_builder::TypedBuilder;

pub trait Id: Debug + for<'de> Deserialize<'de> + Hash + Eq + Serialize {}

impl<T> Id for T where T: Debug + for<'de> Deserialize<'de> + Hash + Eq + Serialize {}

pub trait Subject {
    type Id: Id;
}

pub trait ActionType {
    const TYPE: &'static str;
}

#[async_trait]
pub trait Action<C: ?Sized, DM: ?Sized, Subject, Object: ?Sized, Input, Context>:
    Into<Event<Subject, Self::Action, Object, Input, Context>>
where
    C: StorageClient + Send + Sync,
    DM: DecisionMaker + Sync,
    Subject: Send + Sync,
    Object: Send + Sync,
    Input: Send + Sync,
    Context: Send + Sync,
{
    type Action: StorageAction<C, Input> + Send + Sync;

    async fn try_act(
        self,
        decision_maker: &DM,
        storage_client: &C,
    ) -> Result<
        <Self::Action as StorageAction<C, Input>>::Ok,
        Error<<DM as DecisionMaker>::Error, <Self::Action as StorageAction<C, Input>>::Error>,
    >
    where
        C: 'async_trait,
        Input: 'async_trait,
    {
        let event = self.into();
        decision_maker.can_act(&event).await.map_err(Error::authz)?;
        Ok(Self::Action::act(storage_client, event.input)
            .await
            .map_err(Error::storage)?)
    }
}

#[async_trait]
impl<C: ?Sized, DM: ?Sized, Subject, A, Object: ?Sized, Input, Context> Action<C, DM, Subject, Object, Input, Context>
    for Event<Subject, A, Object, Input, Context>
where
    C: StorageClient + Send + Sync,
    DM: DecisionMaker + Sync,
    Subject: Send + Sync,
    Object: Send + Sync,
    Input: Send + Sync,
    Context: Send + Sync,

    A: StorageAction<C, Input> + Send + Sync,
{
    type Action = A;
}

pub trait ObjectType {
    const SERVICE: &'static str;
    const TYPE: &'static str;
    type Id: Id;
}

#[skip_serializing_none]
#[derive(
    Clone, Debug, Deserialize, Dissolve, DissolveMut, DissolveRef, Eq, Getters, PartialEq, Serialize, TypedBuilder,
)]
#[serde(bound(
    serialize = "Subject: Serialize, Action: ActionType, Object: ObjectType, Input: Serialize, Context: Serialize",
    deserialize = "Subject: Deserialize<'de>, Action: ActionType, Object: ObjectType, Input: Deserialize<'de>, Context: Deserialize<'de>",
))]
pub struct Event<Subject, Action: ?Sized, Object: ?Sized, Input, Context = ()> {
    pub subject: Subject,
    #[serde(with = "serde::action")]
    pub action: PhantomData<Action>,
    #[serde(with = "serde::object")]
    pub object: PhantomData<Object>,
    pub input: Input,
    pub context: Context,
}

impl<Subject, A, Objects, Context> ActionType for Event<Subject, A, Objects, Context>
where
    A: ActionType,
{
    const TYPE: &'static str = A::TYPE;
}

pub trait StorageBackend {}

pub trait StorageClient {
    type Backend: StorageBackend;
}

pub trait HasStorageObject<Backend>: ObjectType + Send + Sync {
    type Constructor<'a>: HasStorageObject<Backend>;
    type StorageObject: StorageObject<Backend> + Send + Sync;
}

pub trait StorageObject<Backend> {}

#[async_trait]
pub trait StorageAction<Client: ?Sized, Input>
where
    Client: StorageClient + Send,
{
    type Ok: Send;
    type Error: Debug + Send;

    async fn act(client: &Client, input: Input) -> Result<Self::Ok, Self::Error>
    where
        Client: 'async_trait,
        Input: 'async_trait;
}

#[async_trait]
pub trait DecisionMaker {
    type Ok: Debug + Send;
    type Error: Debug + Send;
    async fn can_act<Subject, Action, Object: ?Sized, Input, Context>(
        &self,
        event: &Event<Subject, Action, Object, Input, Context>,
    ) -> Result<Self::Ok, Self::Error>;
}

/// wraps all components of an authorization decision
/// which can be expected to be carried around in an
/// application context
pub trait Context<DM, SC> {
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
    fn storage_client(&self) -> &SC;
}

#[derive(AsVariant, AsVariantMut, Clone, Copy, Debug, Error, IsVariant, Unwrap)]
pub enum Error<E1, E2> {
    Authz(E1),
    Storage(E2),
}

impl<E1, E2> Error<E1, E2> {
    pub fn authz(err: E1) -> Self {
        Self::Authz(err)
    }
    pub fn storage(err: E2) -> Self {
        Self::Storage(err)
    }
}

pub mod action {
    use super::*;

    #[derive(Derivative)]
    #[derivative(Clone(bound = ""), Copy(bound = ""), Debug, Default(bound = ""))]
    pub struct Create<O: ?Sized>(#[derivative(Debug = "ignore")] PhantomData<O>);

    #[derive(Derivative)]
    #[derivative(Clone(bound = ""), Copy(bound = ""), Debug, Default(bound = ""))]
    pub struct Delete<O: ?Sized>(#[derivative(Debug = "ignore")] PhantomData<O>);

    #[derive(Derivative)]
    #[derivative(Clone(bound = ""), Copy(bound = ""), Debug, Default(bound = ""))]
    pub struct Read<O: ?Sized>(#[derivative(Debug = "ignore")] PhantomData<O>);

    #[derive(Derivative)]
    #[derivative(Clone(bound = ""), Copy(bound = ""), Debug, Default(bound = ""))]
    pub struct Update<O: ?Sized>(#[derivative(Debug = "ignore")] PhantomData<O>);

    impl<O: ?Sized> ActionType for Create<O> {
        const TYPE: &'static str = "create";
    }

    impl<O: ?Sized> ActionType for Delete<O> {
        const TYPE: &'static str = "delete";
    }

    impl<O: ?Sized> ActionType for Read<O> {
        const TYPE: &'static str = "read";
    }

    impl<O: ?Sized> ActionType for Update<O> {
        const TYPE: &'static str = "update";
    }

    pub trait TryCreate<Ctx, DM, SC, I>: HasStorageObject<<SC as StorageClient>::Backend> + Sync
    where
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Create<Self>: StorageAction<SC, I>,
    {
        fn try_create<'life0, 'async_trait>(
            ctx: &'life0 Ctx,
            input: I,
        ) -> BoxFuture<
            'async_trait,
            Result<
                <Create<Self> as StorageAction<SC, I>>::Ok,
                Error<DM::Error, <Create<Self> as StorageAction<SC, I>>::Error>,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
            SC: 'async_trait,
            DM: 'async_trait,
            I: 'async_trait,
        {
            let event = Event {
                context: ctx.context(),
                subject: ctx.subject(),
                action: PhantomData::<Create<Self>>::default(),
                object: PhantomData::<Self>::default(),
                input,
            };
            event.try_act(ctx.decision_maker(), ctx.storage_client()).boxed()
        }
    }

    pub trait TryDelete<Ctx, DM, SC, I>: HasStorageObject<<SC as StorageClient>::Backend> + Sync
    where
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Delete<Self>: StorageAction<SC, I>,
    {
        fn try_delete<'life0, 'async_trait>(
            ctx: &'life0 Ctx,
            input: I,
        ) -> BoxFuture<
            'async_trait,
            Result<
                <Delete<Self> as StorageAction<SC, I>>::Ok,
                Error<DM::Error, <Delete<Self> as StorageAction<SC, I>>::Error>,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
            SC: 'async_trait,
            DM: 'async_trait,
            I: 'async_trait,
        {
            let event = Event {
                context: ctx.context(),
                subject: ctx.subject(),
                action: PhantomData::<Delete<Self>>::default(),
                object: PhantomData::<Self>::default(),
                input,
            };
            event.try_act(ctx.decision_maker(), ctx.storage_client()).boxed()
        }
    }

    pub trait TryRead<Ctx, DM, SC, I>: HasStorageObject<<SC as StorageClient>::Backend> + Sync
    where
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Read<Self>: StorageAction<SC, I>,
    {
        fn try_read<'life0, 'async_trait>(
            ctx: &'life0 Ctx,
            input: I,
        ) -> BoxFuture<
            'async_trait,
            Result<
                <Read<Self> as StorageAction<SC, I>>::Ok,
                Error<DM::Error, <Read<Self> as StorageAction<SC, I>>::Error>,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
            SC: 'async_trait,
            DM: 'async_trait,
            I: 'async_trait,
        {
            let event = Event {
                context: ctx.context(),
                subject: ctx.subject(),
                action: PhantomData::<Read<Self>>::default(),
                object: PhantomData::<Self>::default(),
                input,
            };
            event.try_act(ctx.decision_maker(), ctx.storage_client()).boxed()
        }
    }

    pub trait TryUpdate<Ctx, DM, SC, I>: HasStorageObject<<SC as StorageClient>::Backend> + Sync
    where
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Update<Self>: StorageAction<SC, I>,
    {
        fn try_read<'life0, 'async_trait>(
            ctx: &'life0 Ctx,
            input: I,
        ) -> BoxFuture<
            'async_trait,
            Result<
                <Update<Self> as StorageAction<SC, I>>::Ok,
                Error<DM::Error, <Update<Self> as StorageAction<SC, I>>::Error>,
            >,
        >
        where
            'life0: 'async_trait,
            Self: 'async_trait,
            SC: 'async_trait,
            DM: 'async_trait,
            I: 'async_trait,
        {
            let event = Event {
                context: ctx.context(),
                subject: ctx.subject(),
                action: PhantomData::<Update<Self>>::default(),
                object: PhantomData::<Self>::default(),
                input,
            };
            event.try_act(ctx.decision_maker(), ctx.storage_client()).boxed()
        }
    }

    impl<Ctx, DM, SC, T, I> TryCreate<Ctx, DM, SC, I> for T
    where
        Self: HasStorageObject<<SC as StorageClient>::Backend> + Sync,
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Create<Self>: StorageAction<SC, I>,
    {
    }

    impl<Ctx, DM, SC, T, I> TryDelete<Ctx, DM, SC, I> for T
    where
        Self: HasStorageObject<<SC as StorageClient>::Backend> + Sync,
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Delete<Self>: StorageAction<SC, I>,
    {
    }

    impl<Ctx, DM, SC, T, I> TryRead<Ctx, DM, SC, I> for T
    where
        Self: HasStorageObject<<SC as StorageClient>::Backend> + Sync,
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Read<Self>: StorageAction<SC, I>,
    {
    }

    impl<Ctx, DM, SC, T, I> TryUpdate<Ctx, DM, SC, I> for T
    where
        Self: HasStorageObject<<SC as StorageClient>::Backend> + Sync,
        DM: DecisionMaker + Sync,
        SC: StorageClient + Send + Sync,
        Ctx: Context<DM, SC> + Sync,
        I: Send + Sync,
        Update<Self>: StorageAction<SC, I>,
    {
    }
}

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
