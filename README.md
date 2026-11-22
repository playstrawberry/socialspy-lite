# socialspy-lite

Social spy: staff read the private messages of players.

A script for Play Strawberry servers, by Strawberry Interactive LLC. Its texts are in English, French, Spanish and Turkish.

## Install on a Play Strawberry server

Add it to your server in the Play Strawberry panel and restart the server. Play Strawberry servers load only scripts
delivered by Play Strawberry, so a file you build yourself from this source loads only on your own Play Strawberry
developer server.

## Build from source

You need Rust with the `wasm32-wasip2` target and the Strawberry plugin SDK (`strawberry-plugin-api`), which Play
Strawberry gives to its script developers. `Cargo.toml` takes it from the SDK registry named `strawberry`; add that
registry to your Cargo configuration (for example `~/.cargo/config.toml`):

To get the SDK, accept its licence at https://panel.playstrawberry.net/strawberry/sdk; the page gives you a
token for `cargo login --registry strawberry`.

```toml
[registries.strawberry]
index = "sparse+https://sdk.playstrawberry.net/index/"
```

```text
cargo test
cargo build --release --target wasm32-wasip2
```

The script file is `target/wasm32-wasip2/release/socialspy_lite.wasm`.

## License

Source available under the Strawberry Source-Available License 1.0 ([LICENSE](LICENSE)). In short: you may read, build and
change this code for your own use, and run what you build only on Play Strawberry, without making money from it unless we
approve it in writing. The build Play Strawberry offers may be used on any of your Play Strawberry servers, also ones that
earn money. Selling the code or anything built from it is not allowed. Forks are welcome for sending us changes (pull
requests and issues).
