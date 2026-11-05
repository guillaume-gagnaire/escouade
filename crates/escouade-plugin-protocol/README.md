# escouade-plugin-protocol

Le protocole qu'[Escouade](https://escouade.dev) parle à ses plugins : JSON-RPC 2.0, un message par ligne. Ce crate tient ce que
les deux côtés partagent : la version, les méthodes, les messages de la poignée de main et des points d'extension.

Un plugin qui répond tout de suite, une machine à états, n'a besoin que de lui, et la même source compile en process et pour
`wasm32-wasip1` :

```rust
use escouade_plugin_protocol::{turn, Refusal};
use serde_json::{json, Value};

pub fn run() -> i32 {
    turn::run("exemple", json!({}), |method, _params: Value| {
        Err(Refusal::not_found(method))
    })
}

escouade_plugin_protocol::module!(run);
```

`turn` donne aussi ce que `std` ne peut pas donner à un module : `turn::instance_id()` à la place de `std::process::id()`, et
`turn::read_file()` à la place de `std::fs`.

Un plugin dont les appels attendent le réseau prendra plutôt [`escouade-plugin-kit`](https://crates.io/crates/escouade-plugin-kit).

La référence des plugins : <https://escouade.dev/docs/plugins/api>. Le code est dans [le dépôt d'Escouade](https://github.com/guillaume-gagnaire/escouade),
sous licence GPL-3.0-or-later.
