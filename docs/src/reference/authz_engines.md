# Authorization Engines
An [Authorization Engine](https://docs.rs/authzen/latest/authzen/trait.AuthzEngine.html) is an abstraction over a [policy decision point](https://docs.aws.amazon.com/prescriptive-guidance/latest/saas-multitenant-api-access-authorization/pdp.html).
It's main priority is to provide binary decisions on whether actions are allowed and, in the future, to support partial evaluation of policies which can then be adapted
to queries on different data sources (OPA and Oso both support partial evaluation).
