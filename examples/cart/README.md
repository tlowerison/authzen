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
