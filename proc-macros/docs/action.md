Generate a custom action which can be queried for authorization and performed on a data source.

The output produced by this macro essentially consists of two traits:
- a trait encapsulating a query to some authorization engine of whether the action is allowed;
this trait is generally referred to as a "Can" trait (e.g. [`CanCreate`](actions/trait.CanCreate.html))
- a trait encapsulating a query which performs the authorization engine, and if allowed, performs the action;
this trait is generally referred to as a "Try" trait (e.g. [`TryCreate`](actions/trait.TryCreate.html))

# Example
This will show a couple of ways to configuring a custom action we'll call `Replace`.
```rs
action!(Replace);
```
This will produce two traits `CanReplace` and `TryReplace`, with methods `can_replace` and `try_replace` respectively.
There is a struct generated under the hood as well called `Replace` which implements [`ActionType`](trait.ActionType.html) with [`ActionType::TYPE`](trait.ActionType.html#associatedconstant.TYPE)
equal to `"replace"`. The default generated value of [`ActionType::TYPE`](trait.ActionType.html#associatedconstant.TYPE) when none is specified is the action name converted
to lower snake case.

We can explicitly set the value of the [`ActionType::TYPE`](trait.ActionType.html#associatedconstant.TYPE) using the syntax below.
```rs
action!(Replace = "my.actions.replace");
```

Note that [`ActionType::TYPE`](trait.ActionType.html#associatedconstant.TYPE) is the name of the action that authorization engines are expected to recognize on authorization queries.
So if we call `CanReplace::can_replace`, the input data to the authorization engine would look something like the below (omitting other fields)
```json
{"action": "my.actions.replace"}
```
