# cart

## Setup

### Postgres
This example uses postgres as its data data source, you'll need it installed prior to installing diesel in the next step.

### Diesel
Diesel is used as the database interface in this example.
```sh
# if you run into issues installing diesel on macos, try setting these C++ and ld flags; see https://diesel.rs/guides/getting-started
# export CPPFLAGS="-I/opt/homebrew/opt/llvm/include,-I/usr/local/opt/libpq/include"
# export LDFLAGS="-L/opt/homebrew/opt/llvm/lib,-L/usr/local/opt/libpq/lib"
cargo install diesel_cli --no-default-features postgres
cd app
diesel setup
```

### Redis
Redis is used for account session management in this example.
Installation releases can be found [here](https://redis.io/docs/getting-started/installation).

### Open Policy Agent
Install the `opa` binary which runs an opa server for policy decisions.
Releases can be found [here](https://github.com/open-policy-agent/opa/releases).

### Cargo Watch
In case you want to mess around with hot reloading changes to the authz [policies](https://github.com/tlowerison/authzen/tree/main/examples/cart/policies/rego),
the `policies/local.sh` script watches with the file with `cargo watch`. This binary can be installed with
```sh
cargo install cargo-watch
```

### Clone repo
```sh
git clone https://github.com/tlowerison/authzen
cd authzen
```

## Running
```sh
# app server
cd examples/cart/app && cargo run

# open policy agent (with hot-reloading)
sh examples/cart/policies/local.sh

# policy information point
cd examples/cart/policy-information-point && cargo run

# transaction cache
docker run --rm --name mongodb -p 27017:27017 mongo:6.0.4-focal

# session storage
cd /tmp && redis-server
```

If you want to be explore how this example works with distributed tracing, run
```sh
docker run \
  -e COLLECTOR_OTLP_ENABLED=true \
  -p 6831:6831/udp \
  -p 6832:6832/udp \
  -p 5778:5778 \
  -p 16686:16686 \
  -p 4317:4317 \
  -p 4318:4318 \
  -p 14250:14250 \
  -p 14268:14268 \
  -p 14269:14269 \
  -p 9411:9411 \
  --rm \
  jaegertracing/all-in-one:latest
```
then navigate to the [local Jaeger UI](http://localhost:16686) in your browser to see request traces.

### Example Usage
```sh
# this username is actually a special value,
# see the policy in `policies/rego/app/examples_cart/item/create.rego`
username="super_special_admin_username"

# create new account with the above username and capture the session cookie created
cookie="$(curl localhost:8000/api/sign-up --data "{\"username\":\"$username\"}" -H "Content-Type: application/json" -v 2>&1 | grep set-cookie | sed 's/< set-cookie: //g')"

# create a new item and capture its id
item_id="$(curl localhost:8000/api/item --data "{\"name\":\"lambo\",\"description\":\"momma\!\"}" -H "Content-Type: application/json" -H "Cookie: $cookie" -s | jq -r '.id')"

# add an item to our cart (will automatically create a cart for this account since none exists yet)
curl localhost:8000/api/add-cart-item --data "{\"itemId\":\"$item_id\"}" -H "Content-Type: application/json" -H "Cookie: $cookie" -s | jq .

# retrieve our cart
curl localhost:8000/api/cart -H "Cookie: $cookie" -s | jq .
```
