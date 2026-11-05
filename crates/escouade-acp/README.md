# escouade-acp

The ACP (Agent Client Protocol, version 1) client of [Escouade](https://escouade.dev): a state machine that reads an agent's lines
and answers the items of the conversation and the lines to write back to it, with no process or runtime of its own. It compiles for
`wasm32-wasip1` too.

```rust
use escouade_acp::Agent;
use escouade_plugin_protocol::engine::{Attach, HarnessDecl};
use serde_json::json;

let harness: HarnessDecl = serde_json::from_value(json!({ "id": "example", "name": "Example", "acp": {} })).unwrap();
let mut agent = Agent::new(&Attach::default(), &harness, "2.2.0");

// What the program wrote, line by line: the items of the conversation, and those that say what to write back.
let items = agent.frames(&[json!({ "jsonrpc": "2.0", "method": "session/update", "params": {} }).to_string()]);
assert!(items.len() <= 1);
```

The reference of the plugins: <https://escouade.dev/en/docs/plugins/api>. The code is in
[Escouade's repository](https://github.com/guillaume-gagnaire/escouade). This crate is under the MIT licence; the app is under
GPL-3.0-or-later.
