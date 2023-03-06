# Open Policy Agent
Open Policy Agent is supported for use as an authorization engine with authzen.
You can either use the provided [client](https://docs.rs/authzen-opa/latest/authzen_opa/struct.OPAClient.html)
which depends on the following assumptions about the structure of your rego policies:
- input is expected to have the structure:
```
{
  "subject": {
    "value": {
      "token": "<subject-jwt>"
    }
  },
  "action": "<action-type>",
  "object": "<object-type>",
  "input": # json blob,
  "context": # json blob,
  "transaction_id": # string or null,
}
```
- the output has structure `{"response":bool}`
where the details you choose to use about the subject live inside the encoded jwt token like so
```rego
token := io.jwt.decode_verify(
  input.subject.value.token,
  {"alg": "<jwt-alg>", "cert": "<jwt-cert>"},
)
is_verified := token[0]
subject := token[2].state
```

### Policy Information Point and Transaction Cache
If your policies are not governing live data, there's no need for either a policy information point nor a transaction cache.

Otherwise, it's highly suggested! Taking the leap to implementing a policy information point can seem like the design is getting out of hand but
in reality it's the final link to make your authorization system as flexible as needed! Integrating a transaction cache into your policy information
point will also ensure that the information used by OPA will be fresh and valid within transactions as well as outside of them.

Authzen provides an easy way to run a policy information point server with transaction cache integration through the use of the [server](https://docs.rs/authzen/latest/authzen/macro.server.html)
macro. Using a trait based handler system, the server fetches objects based off of your own custom defined PIP query type. For an example of this in action,
see the [main.rs](https://github.com/tlowerison/authzen/blob/main/examples/cart/policy-information-point/src/main.rs) in the example and check out
the example [context and query definitions](https://github.com/tlowerison/authzen/blob/main/examples/cart/policy-information-point/src/lib.rs) as
well as the custom data type [handler implementations](https://github.com/tlowerison/authzen/blob/main/examples/cart/policy-information-point/src/examples_cart.rs).

### Rego Template
If you want to use a working rego policy template out of the box,
check out the rego [package entry](https://github.com/tlowerison/authzen/blob/main/examples/cart/policies/rego/app/main.rego) in the example.
For interaction with your policy information point, [util.rego](https://github.com/tlowerison/authzen/blob/main/examples/cart/policies/rego/util.rego) in the example
provides a useful function `data.util.fetch` which when provided input with structure
```
{
  "service": "<object's-service-name>",
  "type": "<object-type>",
  # query fields here
}
```
