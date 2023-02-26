# policies

## Running locally
### Setup
```sh
cargo install cargo-watch
opa_version=v0.49.1
[[ "$OSTYPE" == "linux-gnu"*  ]] && [[ "$(uname -m)" == "x86_64" ]] && target="opa_linux_amd64_static"
[[ "$OSTYPE" == "linux-gnu"*  ]] && [[ "$(uname -m)" == "arm6" ]] && target="opa_linux_arm64_static"
[[ "$OSTYPE" == "darwin"*  ]] && [[ "$(uname -m)" == "x86_64" ]] && target="opa_darwin_amd64"
[[ "$OSTYPE" == "darwin"*  ]] && [[ "$(uname -m)" == "arm64" ]] && target="opa_darwin_arm64_static"
[[ "$OSTYPE" == "win32"  ]] && target="opa_windows_amd64.exe"

curl -L -o opa https://openpolicyagent.org/downloads/$opa_version/$target
chmod 755 ./opa

# place opa binary somewhere in your path
```

### Run
```sh
./local.sh
```
