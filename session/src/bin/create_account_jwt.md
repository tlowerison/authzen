# create_account_jwt
Use this binary to create service account jwts to be provided as environment variables to services so that they can securely communicate with each other.

## Usage
```sh
export JWT_ACCOUNT_ID="<account id>"
export JWT_ALGORITHM="<algorithm>"
export JWT_DURATION="<duration>"
export JWT_ISSUER="<issuer>"
export JWT_PRIVATE_CERTIFICATE="<private certificate>"
export JWT_PUBLIC_CERTIFICATE="<public certificate>"

cargo run --bin create_account_jwt --features create_account_jwt
```

### Example
```sh
openssl genrsa -out private-key.pem 3072
openssl rsa -in private-key.pem -pubout -out public-key.pem

export JWT_ACCOUNT_ID="\"e3cb4265-b2a1-4e42-8422-b3c720e83a20\""
export JWT_ALGORITHM="RS512"
export JWT_DURATION="720" # == 12 hours
export JWT_ISSUER="accounts"
export JWT_PRIVATE_CERTIFICATE="$(cat private-key.pem | awk '{printf "%s_", $0}')"
export JWT_PUBLIC_CERTIFICATE="$(cat public-key.pem | awk '{printf "%s_", $0}')"

# create jwt with environment variables
cargo run --bin create_account_jwt --features account,cli

# create jwt using command line args
cargo run \
  --bin create_account_jwt \
  --features account,cli \
  -- \
  --account-id "\"e3cb4265-b2a1-4e42-8422-b3c720e83a20\"" \
  --alg RS512 \
  --dur 720 \
  --iss accounts \
  --private-key private-key.pem \
  --public-key public-key.pem
```

### Example
```sh
openssl genrsa -out private-key.pem 3072
openssl rsa -in private-key.pem -pubout -out public-key.pem

export JWT_ACCOUNT_ID="123"
export JWT_ALGORITHM="RS512"
export JWT_DURATION="720" # == 12 hours
export JWT_ISSUER="accounts"
export JWT_PRIVATE_CERTIFICATE="$(cat private-key.pem | awk '{printf "%s_", $0}')"
export JWT_PUBLIC_CERTIFICATE="$(cat public-key.pem | awk '{printf "%s_", $0}')"

# create jwt with additional state fields
cargo run \
  --bin create_account_jwt \
  --features account,cli \
  -- \
  --field $'role_ids=["3d972425-b3c9-4ded-8c66-404434ec4773"]'
```
