# Contributing

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
  -w session \
  -w storage-backends \
  -w Cargo.toml \
  -- \
  cargo +nightly doc --workspace --no-deps --all-features
```

### Build
To generate the docs, cd into the root of the repository and run:
```sh
cargo +nightly doc --workspace --no-deps --all-features --verbose --color always
echo "<meta http-equiv=\"refresh\" content=\"0; url=authzen\">" > target/doc/index.html
rm -rf docs
mkdir docs
find target/doc -mindepth 1 -maxdepth 1 | sed "s:target/doc/::g" | xargs -n 1 -I {} sh -c "mv target/doc/{} docs/{}"
```
