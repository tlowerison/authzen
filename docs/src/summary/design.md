# Design
The design philosophy of authzen was heavily influenced by [hexagonal architecture](https://netflixtechblog.com/ready-for-changes-with-hexagonal-architecture-b315ec967749).
Particularly, authzen is designed with the goal of supporting not only swappable data sources but also swappable authorization engines.
The core exports of authzen land squarely in the *Interactors* category of the above article: utilities which facilitate interacting with the underlying authorization engine
and data sources while not exposing their internals.
Applications should be able to use a call to `PostTag::try_create` in their business logic and not need to change that code if they want to swap out where `PostTag`s are stored
or which authorization engine authorizes their creation.

