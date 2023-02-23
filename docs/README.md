# docs
To generate the docs, cd into the root of the repository and run:
```sh
cargo +nightly doc --workspace --no-deps --verbose --color always
echo "<meta http-equiv=\"refresh\" content=\"0; url=authzen\">" > target/doc/index.html
rm -rf docs/src
mkdir docs/src
find target/doc -mindepth 1 -maxdepth 1 | sed "s:target/doc/::g" | xargs -n 1 -I {} sh -c "mv target/doc/{} docs/src/{}"
```
