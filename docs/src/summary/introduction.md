# Introduction
Authzen is a framework for easily integrating authorization into backend services.

Policy based authorization is great but can be really complex to integrate into an application.
This project exists to help remove a lot of the up front cost that's required to get authorization working in backend rust services.

Authzen can mostly be thought of as a "frontend" gluing together multiple different backends.
It orchestrates the enforcement of authorization policies and then the
performance of those authorized actions. The enforced policies live external to authzen and are managed
by a "decision maker" such as [Open Policy Agent](https://www.openpolicyagent.org). Authzen wraps around that decision maker, and when
an action needs to be authorized, authzen relays the policy query to the decision maker. Then if the action is allowed,
authzen can either stop there or perform the action, depending on where the "action" takes place.

For example, say we want to authorize whether a user can tag another user in a post. The authorization engine is running
in a separate process and can be reached by a network connection; when provided a query of the format `(subject, action, object, input, context)`
the authorization engine will return some form of a binary decision indicating whether the user (*subject*) can create (*action*) a `PostTag` (*object* as well as *input*).
Assuming `PostTag`s live in a database that the backend service has a connection to, and that the backend uses a database interface supported by authzen (diesel, etc.),
the order of operations looks something like this:
- backend application calls `PostTag::try_create`, providing the `PostTag` to be created and a relevant context object containing all the http clients/connections needed
  to reach the authorization engine, database, etc.
  - note that `try_create` is a method provided by authzen while `PostTag` is a user defined type
- authzen relays the policy query to the authorization engine specified in the context above
  - if the policy query is rejected or fails, `try_create` returns an error
- authzen then inserts the `PostTag` into the database using adaptor code provided by authzen
  - for example, if `PostTag` has a "connected" struct `DbPostTag` which implements `diesel::Insertable`, then authzen automatically derives
    the adaptor code to insert the `PostTag` inside of `PostTag::try_create`
  - if the insertion of the `PostTag` into the database fails, `try_create` returns an error
- authzen then places the inserted `PostTag` into a "transaction cache" which stores all of the objects inserted/updated/deleted as a part of
  this database transaction as a json blob

The transaction cache step is the *crucial use case* for authzen: if your authorization engine requires up to date information to make accurate and unambiguous
policy decisions, then changes to your database over the course of a transaction must be visible to your authorization engine. Because the
component of the authorization engine which retrieves data information typically runs in a different process with different
database connections from the transaction, it may not be able to see the changes from the transaction! The transaction cache is a
workaround for this problem, which when integrated into your authorization engine, allows it to fetch the most up-to-date versions of your data.

