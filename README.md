# authzen
A framework for easily integrating authorization into backend services.

Authzen provides primitives for combining the enforcement of authorization policies and the actions those policies govern.
For example, in an endpoint which creates a `Foo` for a user but needs to be certain the user is authorized to create the
Foo provided, using authzen this would simply look like
```rust
Foo::try_create(ctx, [my_foo]).await
```
The method `try_create` combines both the authorization enforcement with the actual creation of the Foo.
If you need to authorize the action separately from the performance of the action, which can happens often, you can instead call
```rust
Foo::can_create(ctx, &[&my_foo]).await
```

The design philosophy of authzen was heavily influenced by [hexagonal architecture](https://netflixtechblog.com/ready-for-changes-with-hexagonal-architecture-b315ec967749)
and aims to have pluggable backends in multiple different areas.

## Components of authzen
The main components of the authzen framework are:
- authorization primitives
- storage clients
- decision makers
- transaction caches (optional)

Each component is discussed in its own section.

### Authorization Primitives
authzen provides the following core abstractions to be used when describing a policy and its components
