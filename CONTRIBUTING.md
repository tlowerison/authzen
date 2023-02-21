# Contributing
I just made this a separate doc to document local development.

## Documentation
Documentation hot reloading on file changes can be run in the background using cargo watch.

### Setup
```sh
cargo install cargo-watch
```

### Run
```sh
cargo watch \
  -w authzen \
  -w core \
  -w decision-makers \
  -w proc-macro-util \
  -w proc-macros \
  -w service-util \
  -w storage-backends \
  -w Cargo.toml \
  -- \
  cargo +nightly doc --workspace --no-deps --all-features
```
