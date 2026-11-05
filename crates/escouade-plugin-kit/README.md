# escouade-plugin-kit

What the Rust plugins of [Escouade](https://escouade.dev) share, so that a plugin is nothing but its client. The same source
compiles in process and for `wasm32-wasip1`, without tokio or reqwest in the second case.

- `serve`: the loop of a plugin. Calls answer as they end, several at once, and the plugin can ask Escouade for its secrets and
  its settings while it answers (`Cx`).
- `hosts`: the service of code hosts and of the Kanban's sources, and of their account. A plugin gives its kinds (`ForgeKind`,
  `SourceKind`); the account, its token, one client per account, the typed errors and the budget of calls are here.
- `forge`: the contract of a code host's client.
- `errors`, `text`: an HTTP error and what it says, the criteria of a ticket's description.
- `http`: the calls, and `http::reqwest`, a plugin's client. It is reqwest in process and, in a module, the part of reqwest
  plugins use, over `net/fetch`.
- `time`: `sleep` and `timeout`, on tokio in process and on the host's timers in a module.
- `i18n`: the language of the interface, as `initialize` says it.

```rust
use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::serve::{self, Cx, Plugin, Refusal};
use serde_json::Value;

struct Example;

impl Plugin for Example {
    fn call<'a>(&'a self, _cx: &'a Cx, method: &'a str, _params: Value) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(async move { Err(Refusal::not_found(method)) })
    }
}

pub fn run() -> i32 {
    serve::run("example", Example)
}

escouade_plugin_kit::protocol::module!(run);
```

The reference of the plugins: <https://escouade.dev/en/docs/plugins/api>. The code is in
[Escouade's repository](https://github.com/guillaume-gagnaire/escouade). This crate is under the MIT licence; the app is under
GPL-3.0-or-later.
