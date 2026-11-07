# escouade-plugin-protocol

The protocol [Escouade](https://escouade.dev) speaks to its plugins: JSON-RPC 2.0, one message a line. This crate holds what both
sides share: the version, the methods, the messages of the handshake and of the extension points.

A plugin that answers at once, a state machine, needs nothing else, and the same source compiles in process and for
`wasm32-wasip1`:

```rust
use escouade_plugin_protocol::{turn, Refusal};
use serde_json::{json, Value};

pub fn run() -> i32 {
    turn::run("example", json!({}), |method, _params: Value| {
        Err(Refusal::not_found(method))
    })
}

escouade_plugin_protocol::module!(run);
```

`turn` also gives what `std` cannot give a module: `turn::instance_id()` in place of `std::process::id()`, and
`turn::read_file()` in place of `std::fs`.

A plugin whose calls wait on the network will rather take [`escouade-plugin-kit`](https://crates.io/crates/escouade-plugin-kit).

The reference of the plugins: <https://escouade.dev/en/docs/plugins/api>. The code is in
[Escouade's repository](https://github.com/guillaume-gagnaire/escouade). This crate is under the MIT licence; the app is under
GPL-3.0-or-later.
