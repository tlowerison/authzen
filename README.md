# authzen
A framework for easily integrating authorization into backend services. The design philosophy of authzen was heavily influenced by [hexagonal architecture](https://netflixtechblog.com/ready-for-changes-with-hexagonal-architecture-b315ec967749)
and aims to provide authorization primitives with the support for many different "backends".

## Motivation and Objectives
Policy based authorization is great but can be really complex to integrate into an application. This project exists to help remove a lot of the up front cost
that's required to get authorization working in backend rust services. The goals of this project include:
- annotation of object metadata (i.e. what's this object's type and originating service) which will be used by authorization engines
- easy authorization enforcement (should be able to query whether a requestor is able to perform an action against some objects, e.g. can this requestor create these objects)
- integration with different storage backends so that actions can be authorized and then, if allowed, performed as atomic operations

## Example

Authzen provides primitives for combining the enforcement of authorization policies and the actions those policies govern.
For example, in an endpoint which creates a `Foo` for a user but needs to be certain the user is authorized to create the
Foo provided, using authzen this would look something like
```rust
#[derive(Clone, Debug, diesel::Insertable, serde::Deserialize, serde::Serialize)]
#[diesel(table_name = foo)] // `foo` is an in-scope struct produced by the diesel::table macro somewhere
pub struct DbFoo {
    pub id: uuid::Uuid,
    pub bar: String,
    pub baz: Option<String>,
}

#[derive(authzen::AuthzObject, Clone, Debug, serde::Deserialize, serde::Serialize)]
#[authzen(service = "my_backend_service_name", ty = "foo")]
pub struct Foo<'a>(pub std::borrow::Cow<'a, DbFoo>);

pub async fn create_foo<D: authzen::storage_backends::diesel::connection::Db>(ctx: Ctx<'_, D>, foos: Vec<Foo>) -> Result<(), anyhow::Error> {
    use authzen::actions::TryCreate;

    let db_foos = Foo::try_create(ctx, foos).await?;

    // ...

    Ok(())
}

```
The method `try_create` combines both the authorization enforcement with the actual creation of the Foo.
If you need to authorize the action separately from the performance of the action, which can happens often, you can instead call
```rust
pub async fn create_foo<D: authzen::storage_backends::diesel::connection::Db>(ctx: Ctx<'_, D>, foos: Vec<Foo>) -> Result<(), anyhow::Error> {
    use authzen::actions::TryCreate;
    use authzen::storage_backends::diesel::operations::DbInsert;

    Foo::can_create(ctx, &foos).await?;
    // ...
    let db_foos = DbFoo::insert(ctx, foos).await?; // note, DbFoo automatically implements the trait DbInsert, giving it the method `DbInsert::insert`
    // ...
    Ok(())
}
```

There is a working example in the [examples](https://github.com/tlowerison/authzen/tree/main/examples/cart) directory which uses
postgres as a database,
[diesel](https://diesel.rs) as its rust-sql interface (aka its [storage client](#storage-clients)),
[Open Policy Agent](https://www.openpolicyagent.org) as its policy decision point (in authzen, this is referred to as a [decision maker](#decision-makers)),
and the [Mongodb](https://hub.docker.com/_/mongo) container as its [transaction cache](#transaction-caches).

It's highly recommended to give this a look to get an idea of what authzen can do and how to use it.

## Components of authzen
The main components of the authzen framework are:
- [authorization primitives](#authorization-primitives)
- [storage clients](#storage-clients)
- [decision makers](#decision-makers)
- [transaction caches](#transaction-caches) (optional)

Each component is discussed in its own section.

### <a id="authorization-primitives"></a> Authorization Primitives
authzen provides the following core abstractions to be used when describing a policy and its components
- [ActionType](https://tlowerison.github.io/authzen/authzen/trait.ActionType.html): denote the type of an action, will be used to identify the action in decision makers
- [ObjectType](https://tlowerison.github.io/authzen/authzen/trait.ObjectType.html): denote the type and originating service of an object, will be used to identify the object in decision makers
- [Event](https://tlowerison.github.io/authzen/authzen/struct.Event.html): collection of all identifying information which will be used as input for an authorization decision; it is generic over the following parameters
  - Subject: who is performing the action; can be any type
  - Action: what the action is; must implement `ActionType`
  - Object:
    - the object being acted upon; must implement `ObjectType`, which should typically be derived using [AuthzObject](https://tlowerison.github.io/authzen/authzen/derive.AuthzObject.html)
    - see here for an [example](https://github.com/tlowerison/authzen/blob/main/examples/cart/app/src/authz/account.rs#L5-L7) usage
    - note that this parameter *only* represents the information about the object
    which can be derived from `ObjectType`, i.e. object type and object service
  - Input:
    - the actual data representing the object being acted upon, this can take many different forms and is dependent on which storage backend(s) this object lives in
    - for example, if trying to create a `Foo`, an expected input could be a vec of `Foo`s which the decision maker can then use to determine if they the action is acceptable or not
    - as another example, if trying to read a `Foo`, an expected could be a vec of `Foo` ids
  - Context: any additional information which may be needed by the decision maker to make an unambiguous decision; typically the type of the Context provided should be the same
    across all events since the policy enforcer (the server/application) shouldn't need to know what context a specific action requires, that is up to the decision maker
- [AuthzObject](https://tlowerison.github.io/authzen/authzen/derive.AuthzObject.html):
  - derive macro used to implement `ObjectType` for a wrapper struct which should contain
  a representation of the object which can be persisted to a specific storage backend
  - for example, if you have a struct `DbFoo` which can be persisted to a database, then `AuthzObject` should be derived on some other struct `pub struct Foo<'a>(pub Cow<'a, DbFoo>);`. The use of a newtype with Cow
    is actually necessary to derive `AuthzObject` (the compiler will let you know if you forget), because there are certain cases where we want to construct an `ObjectType` with a reference and not an owned value
- [ActionError](https://tlowerison.github.io/authzen/authzen/enum.ActionError.html): an error type encapsulating the different ways an action authorization+performance can fail
- `Try*` traits:
  - this is a class of traits which are automatically derived for valid `ObjectType` types (see the section on StorageAction for more details)
  - `*` here can be replaced with the name of an action, for example
  [TryCreate](https://tlowerison.github.io/authzen/authzen/actions/trait.TryCreate.html),
  [TryDelete](https://tlowerison.github.io/authzen/authzen/actions/trait.TryDelete.html),
  [TryRead](https://tlowerison.github.io/authzen/authzen/actions/trait.TryRead.html), and
  [TryUpdate](https://tlowerison.github.io/authzen/authzen/actions/trait.TryUpdate.html)
  - each `Try*` trait contains two methods: `can_*` and `try_*`, the former only authorizes an action, while the latter both authorizes and then, if allowed, performs an action
    - these two methods are the primary export of authzen, meaning that they are the points of authorization enforcement and provide considerable value and code
  - the `Try*` traits are generated using the [action](https://tlowerison.github.io/authzen/authzen/macro.action.html) macro
- [action](https://tlowerison.github.io/authzen/authzen/macro.action.html): given an action name (and optionally an action type string if one wants to explicitly set it), will produce:
  - a type which implements `ActionType`; it is generic over the object type it is acting upon
  - the `Try*` traits mentioned above and implementations of them for any type `O` implementing `ObjectType` for which the action implements `StorageAction<O>`

### <a name="storage-clients"></a> Storage Clients

### <a name="decision-makers"></a> Decision Makers

### <a name="transaction-caches"></a> Transaction Caches
