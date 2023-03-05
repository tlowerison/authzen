# Transaction Caches
Transaction caches are transient json blob storages (i.e. every object inserted only lives for a short bit before being removed) which contain objects which have been mutated in the course of a transaction (only objects which we are concerned with authorizing).
They are essential in ensuring that an authorization engine has accurate information in the case where it would not be able to view data which is specific to an ongoing transaction.

For example, say we have the following architecture:
- a backend api using authzen for authorization enforcement
- a postgres database
- OPA as the authorization engine
- a policy information point which is essentially another api which OPA talks to in order to retrieve information about objects it is trying to make policy decisions on
- a transaction cache

Then let's look at the following operations taking place in the backend api wrapped in a database transaction:

1. Authorize then create an object `Foo { id: "1", approved: true }`.
2. Authorize then create two child objects `[Bar { id: "1", foo_id: "1" }, Bar { id: "1", foo_id: "2" }]`.

Say our policies living in OPA look something like this:
```rego
import future.keywords.every

allow {
  input.action == "create"
  input.object.type == "foo"
}

allow {
  input.action == "create"
  input.object.type == "bar"
  every post in input.input {
    allow_create_bar[post.id]
  }
}

allow_create_bar[id] {
  post := input.input[_]
  id := post.id

  # retrieve the Foos these Bars belong to
  foos := http.send({
    "headers": {
		  "accept": "application/json",
		  "content-type": "application/json",
		  "x-transaction-id": input.transaction_id,
    },
		"method": "POST",
		"url": "http://localhost:9191", # policy information point url
		"body": {
      "service": "my_service",
      "type": "foo",
      "ids": {id | id := input.input[_].foo_id},
    },
  }).body

  # policy will automatically fail if the parent foo does not exist
  foo := foos[post.foo_id]

  foo.approved == true
}
```

Without a transaction cache to store transaction specific changes, the policy information point would
have no clue that `Foo { id: "1" }` exists in the database and therefore this whole operation would fail.
If we integrate the transaction cache into our policy information point to pull objects matching the
given query (in this case, `{"service":"my_service","type":"foo","ids":["1"]}`) from both the database *and*
the transaction cache, then the correct information will be returned for `Foo` with id `1` and the policy
will correctly return that the action is acceptable.

Integration of a transaction cache into a policy information point is very straightforward using authzen, see section on [policy information points](#policy-information-points).
