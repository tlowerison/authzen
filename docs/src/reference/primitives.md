# Authorization Primitives
Authzen provides the following core abstractions to be used when describing a policy and its components
- [ActionType](https://docs.rs/authzen/latest/authzen/trait.ActionType.html): denote the type of an action, will be used to identify the action in decision makers
- [ObjectType](https://docs.rs/authzen/latest/authzen/trait.ObjectType.html): denote the type and originating service of an object, will be used to identify the object in decision makers
- [AuthzObject](https://docs.rs/authzen/latest/authzen/derive.AuthzObject.html):
  - derive macro used to implement `ObjectType` for a wrapper struct which should contain
  a representation of the object which can be persisted to a specific storage backend
  - for example, if you have a struct `DbFoo` which can be persisted to a database, then `AuthzObject` should be derived on some other struct `pub struct Foo<'a>(pub Cow<'a, DbFoo>);`. The use of a newtype with Cow
    is actually necessary to derive `AuthzObject` (the compiler will let you know if you forget), because there are certain cases where we want to construct an `ObjectType` with a reference and not an owned value
- [ActionError](https://docs.rs/authzen/latest/authzen/enum.ActionError.html): an error type encapsulating the different ways an action authorization+performance can fail
- [Event](https://docs.rs/authzen/latest/authzen/struct.Event.html): collection of all identifying information which will be used as input for an authorization decision; it is generic over the following parameters
  - Subject: who is performing the action; can be any type
  - Action: what the action is; must implement `ActionType`
  - Object:
    - the object being acted upon; must implement `ObjectType`, which should typically be derived using [AuthzObject](https://docs.rs/authzen/latest/authzen/derive.AuthzObject.html)
    - see here for an [example](https://github.com/tlowerison/authzen/blob/main/examples/cart/app/src/authz/account.rs#L5-L7) usage
    - note that this parameter *only* represents the information about the object
    which can be derived from `ObjectType`, i.e. object type and object service
  - Input:
    - the actual data representing the object being acted upon, this can take many different forms and is dependent on which storage backend(s) this object lives in
    - for example, if trying to create a `Foo`, an expected input could be a vec of `Foo`s which the decision maker can then use to determine if they the action is acceptable or not
    - as another example, if trying to read a `Foo`, an expected could be a vec of `Foo` ids
  - Context: any additional information which may be needed by the decision maker to make an unambiguous decision; typically the type of the Context provided should be the same
    across all events since the policy enforcer (the server/application) shouldn't need to know what context a specific action requires, that is up to the decision maker
- `Try*` traits:
  - this is a class of traits which are automatically derived for valid `ObjectType` types (see the section on StorageAction for more details)
  - `*` here can be replaced with the name of an action, for example
  [TryCreate](https://docs.rs/authzen/latest/authzen/actions/trait.TryCreate.html),
  [TryDelete](https://docs.rs/authzen/latest/authzen/actions/trait.TryDelete.html),
  [TryRead](https://docs.rs/authzen/latest/authzen/actions/trait.TryRead.html), and
  [TryUpdate](https://docs.rs/authzen/latest/authzen/actions/trait.TryUpdate.html)
  - each `Try*` trait contains two methods: `can_*` and `try_*`, the former only authorizes an action, while the latter both authorizes and then, if allowed, performs an action
    - these two methods are the primary export of authzen, meaning that they are the points of authorization enforcement and provide considerable value and code
  - the `Try*` traits are generated using the [action](https://docs.rs/authzen/latest/authzen/macro.action.html) macro
- [action](https://docs.rs/authzen/latest/authzen/macro.action.html): given an action name (and optionally an action type string if one wants to explicitly set it), will produce:
  - a type which implements `ActionType`; it is generic over the object type it is acting upon
  - the `Try*` traits mentioned above and implementations of them for any type `O` implementing `ObjectType` for which the action implements `StorageAction<O>`
