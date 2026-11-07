# {{name}}

A plugin of [Escouade](https://escouade.dev), in Rust with [`escouade-plugin-kit`](https://docs.rs/escouade-plugin-kit): a WASM module, compiled by the store from these sources, that runs inside the app.

- `src/lib.rs`: what it does (a fact received, a permission decided), and the line that makes it a module;
- `src/tests.rs`: the plugin on the lines Escouade sends, in memory;
- `Cargo.lock`: part of the plugin: the store compiles what it says, and refuses a plugin without it.

```sh
cargo test                          # the plugin, on your machine
cargo check --target wasm32-wasip1  # it compiles as a module (rustup target add wasm32-wasip1)
escouade-plugin validate .          # the manifest, and what the store refuses of the crate
escouade-plugin test .              # the conformance bench of Escouade, on the module
escouade-plugin dev .               # the fake Escouade: type facts, decisions and calls
escouade-plugin build .             # the module as the store makes it, and its SHA-256, in packs/
```

`escouade-plugin` ships with Escouade (https://escouade.dev/en/docs/plugins/testing). It downloads its own compiler the first time: `validate`, `test`, `dev` and `build` need no Rust on your machine.

To try it in Escouade, copy the module next to the manifest as `plugin.wasm`, then load this folder in Plugins › In development. Do not commit `plugin.wasm`: the store writes its own.

A plugin compiles alone in its folder: no `build.rs`, no macro of its own, only crates from crates.io. What the store refuses, and why: https://escouade.dev/en/docs/plugins/rust.
