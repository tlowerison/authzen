# cart

## Setup

### Diesel
```
# for macos, set correct C++ and ld flags, see https://diesel.rs/guides/getting-started if you run into issues
# export CPPFLAGS="-I/opt/homebrew/opt/llvm/include,-I/usr/local/opt/libpq/include"
# export LDFLAGS="-L/opt/homebrew/opt/llvm/lib,-L/usr/local/opt/libpq/lib"

cargo install diesel_cli --no-default-features postgres
cd app
diesel setup
```

## Running
- app server: `cd app && cargo run`
- rego policy engine: `cd policies && sh ./local.sh`
- policy information sever: `cd policy-information-point && cargo run`
- policy information transaction cache: `docker run --rm --name mongodb -p 27017:27017 mongo:6.0.4-focal`
- session storage backend: `cd /tmp && redis-server`

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
then navigate to the local [http://localhost:16686](Jaeger UI) in your browser

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
