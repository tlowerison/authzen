# Storage Backends
A storage client is an abstraction representing the place where objects which require authorization to act upon are stored. A storage action is a representation of an
[ActionType](https://docs.rs/authzen/latest/authzen/trait.ActionType.html) in the context of a specific storage client. For example, the create action has an
implementation as a storage action for any type which implements [DbInsert](https://docs.rs/authzen-diesel/latest/authzen_diesel/operations/trait.DbInsert.html) -- its
storage client is an async diesel connection. Essentially storage actions are a way to abstract over the actual performance of an action using a storage client.

Why do these abstractions exist? Because then we can call methods like [try_create](https://docs.rs/authzen/latest/authzen/actions/trait.TryCreate.html#method.try_create)
for an object rather than having to call [can_create](https://docs.rs/authzen/latest/authzen/actions/trait.TryCreate.html#method.can_create) and then perform the subsequent
action after it has been authorized. Wrapping the authorization and performance of an action is *particularly* useful when the storage backend where the objects are stored is
transactional in nature, see the section on [transaction caches](#transaction-caches) for why that is the case.
