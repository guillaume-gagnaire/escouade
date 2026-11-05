# The Rust kit

A plugin in Rust is a WASM module, compiled by the store from its sources, on the same methods as a plugin in TypeScript. This page says which crates to use, how a source, a code host or a harness is written, and what the compilation refuses.

## The three crates

They are on crates.io in `0.1.0`, under the MIT license: a plugin keeps the choice of its own. Their documentation is on docs.rs, for native and for `wasm32-wasip1`.

| Crate | What it gives | When to use it |
| --- | --- | --- |
| [`escouade-plugin-kit`](https://docs.rs/escouade-plugin-kit) | The loop (`serve`), the http client, `sleep` and `timeout`, the service of code hosts and sources (`hosts`), the typed errors, the language. | Nearly always. |
| [`escouade-plugin-protocol`](https://docs.rs/escouade-plugin-protocol) | The protocol: its types, `turn` and `module!`, the contract of harnesses (`engine`). The kit reexports it (`escouade_plugin_kit::protocol`). | For a harness, alone. |
| [`escouade-acp`](https://docs.rs/escouade-acp) | The ACP client, a state machine without a process. | For a harness that speaks ACP and that [the manifest alone](plugins/harnesses) cannot describe. |

A plugin depends on them by version, with its `Cargo.lock`:

`sdk/create-escouade-plugin/templates/rust/Cargo.toml` (excerpt)

```toml
[lib]
name = "plugin"
crate-type = ["cdylib", "rlib"]

[dependencies]
escouade-plugin-kit = "0.1.0"
serde_json = "1"

# Only for `cargo test`, on your machine: the module has no tokio.
[target.'cfg(not(target_family = "wasm"))'.dev-dependencies]
tokio = { version = "1", features = ["io-util", "rt"] }

# The plugin stands alone: a repository with several plugins does not make it a member of anything.
[workspace]
```

The same source compiles twice: in process on your machine for `cargo test`, with tokio and reqwest, and for `wasm32-wasip1` as a module, without them.

## A plugin

`Plugin` has one method, `call`, which the kit calls for everything that comes, but `initialize` and `shutdown`, which it answers itself:

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (excerpt)

```rust
struct Template;

impl Plugin for Template {
    // Every method of Escouade comes here but `initialize` and `shutdown`, which the kit answers. A notification too: what
    // it returns goes nowhere. Nothing may block here: an `.await` gives the turn back to Escouade.
    fn call<'a>(
        &'a self,
        cx: &'a Cx,
        method: &'a str,
        params: Value,
    ) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(async move {
            match method {
                "events/fact" => {
                    fact(cx, &params);
                    Ok(Value::Null)
                }
                "permissions/decide" => Ok(decide(&params)),
                other => Err(Refusal::not_found(other)),
            }
        })
    }
}
```

`Refusal::not_found` answers `-32601` to what the plugin does not serve. What remains is to say what runs, and to make it a module, in one line that produces nothing natively:

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (excerpt)

```rust
/// What the module runs at its first turn: the plugin, kept for the turns to come. Natively, the loop on stdin and stdout.
pub fn run() -> i32 {
    serve::run("plugin", Template)
}

// The one line that makes this crate a module of Escouade. Natively, it is nothing.
escouade_plugin_kit::protocol::module!(run);
```

`npm create escouade-plugin@latest my-plugin -- --kind rust` writes all that, with its tests: see [a first plugin](plugins/quick-start). Each call of the host has its example in [the calls](plugins/calls).

## The model of turns

One instance per plugin, on one thread, woken by what comes: a line of Escouade, the end of a `net/fetch`, a piece of a stream, a socket’s message, a program’s output, a timer. Escouade writes all that came on the module’s input, runs **one** turn, then serves what the module wrote.

**Nothing waits in a turn.** A request to the host is a line, and its answer comes at a later turn.

- An `.await` gives the hand back. The kit keeps the task and resumes it at the turn its answer comes: several calls go on at once, and a slow call does not hold the others back.
- What blocks the thread blocks the whole plugin: a loop, a long computation, `std::thread::sleep`. There is no thread, and `std::process` fails. `escouade_plugin_kit::time::sleep` replaces the wait.
- There is no environment: the language comes from `initialize`, read by `escouade_plugin_kit::i18n`, and an agent’s folder from `agent/attach`.
- The network goes through `http::reqwest`, the kit’s facade: the part of reqwest plugins use, over `net/fetch`. The proxy, the certificates and the delay stay with the host, and what the plugin says of them has no effect.

A turn keeps nothing for itself: the state lives in the plugin, between two turns, as in a process. It is not kept from one instance to the next.

### The synchronous door

A state machine that must read a file before it answers, like a harness, has one more call: `escouade.call`, an import of the module, served on the plugin’s thread. `turn::read_file` reads through it, and `turn::host_call` sends `fs/*` or `proc/*`. It counts in the turn’s time.

## The limits

| Limit | Value |
| --- | --- |
| Memory | 64 MiB |
| A turn | 2 s, 500 ms when it holds a decision |
| `net/fetch` | 16 at once |
| `net/stream` | 8 at once |
| WebSocket | 4 at once |
| Timers | 64 at once |
| Programs | 4 at once |

The time spent waiting for the network does not count: it is not in a turn. An instance that passes a limit, or panics, is ended and made again, with the same three restarts in five minutes as a process. The journal says why, and what it held is given back by `agent/attach`.

## Writing a source

A source of the Kanban serves `tickets/*`. The kit does nearly everything: `Hosts` answers the protocol, asks for the account’s token, keeps one client per account and puts the errors in the expected form. A plugin gives its kind (`SourceKind`) and a client (`TicketSource`):

`plugins/linear/src/lib.rs` (excerpt)

```rust
impl SourceKind for LinearSource {
    fn id(&self) -> &str {
        ID
    }
    fn name(&self) -> &str {
        S
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        let base = account.api.clone().filter(|a| !a.trim().is_empty());
        Box::new(Linear::new(http, account, base.as_deref().unwrap_or(API)))
    }
}
```

and plugs them in, in `run`:

`plugins/linear/src/lib.rs` (excerpt)

```rust
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "linear",
        escouade_plugin_kit::hosts::Hosts::new(vec![], vec![Box::new(LinearSource)]),
    )
}

escouade_plugin_kit::protocol::module!(run);
```

The manifest declares the account and the source, with `network` and `account:<name>` in `permissions.secrets`:

`plugins/linear/escouade-plugin.json` (excerpt)

```json
    "ticketSources": [{ "id": "linear", "name": "Linear", "account": "linear" }]
```

An http error stays an error: `HttpError` becomes `-(32000 + status)`, `-32401` for a refused token, and `Refused` becomes `-32010`, a lasting refusal. The whole code is in [`plugins/linear`](https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/linear), with Jira, Trello and ClickUp beside it.

## Writing a code host

A code host serves `forge/*`, the same way: `ForgeKind` and the kit’s `Forge` client, each of whose methods answers `Unsupported` until you write it. The Gitea plugin, which serves every instance of a self-hosted service, fits in a few lines:

`plugins/gitea/src/lib.rs` (excerpt)

```rust
impl ForgeKind for GiteaForge {
    fn id(&self) -> &str {
        "gitea"
    }
    fn name(&self) -> &str {
        "Gitea / Forgejo"
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        let base = account
            .api
            .clone()
            .filter(|a| !a.trim().is_empty())
            .unwrap_or_else(|| api_base(&account.url));
        Box::new(pulls::Gitea::new(http, base, account.token.clone()))
    }
}
```

`account.url` is the instance the user typed, hence `{account}` in `permissions.network`. A refused token, a limit reached and the budget of calls are dealt with by `Hosts`. The bench plays `account/me` and `forge/pullRequestsOfBranch` as soon as `contributes.forges` is declared.

## Writing a harness

For an agent that speaks ACP, [the manifest](plugins/harnesses) is enough, without a line of code. Otherwise, a harness is a state machine that never waits, and depends on the protocol alone: `turn::run` holds the loop, on both targets.

`plugins/codex/src/lib.rs` (excerpt)

```rust
pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    turn::run(
        "codex",
        json!({ "harnesses": ["codex"] }),
        move |m, params| plugin.handle(m, params),
    )
}

escouade_plugin_protocol::module!(run);
```

The contract is in `escouade_plugin_protocol::engine`. The core starts the agent’s program and reads its output; the plugin says how to start it, then turns its lines into items of the conversation. `std::process::id` is replaced by `turn::instance_id`, and `std::fs` by `turn::read_file`, under a root of `permissions.files`. An ACP client plugs in through `escouade_acp::Agent`, whose `frames` reads the agent’s lines.

## What the compilation refuses

`escouade-plugin build` compiles as the store does, and `escouade-plugin validate` says the same without running anything. A plugin compiles **alone in its folder**, on a copy, with rustc 1.98.0 and `wasm32-wasip1`, which the command downloads itself, an imposed profile and `--locked`: for the same sources and the same `Cargo.lock`, the module is the same to the byte under Windows and under Linux.

| Refused | Why | What to do |
| --- | --- | --- |
| No `Cargo.lock` | The store compiles what it says, and nothing else. | Commit it. |
| A `build.rs` in the plugin | It would run natively on the store. | Generate the code beforehand and commit it. |
| A procedural macro of your own | The same reason. | A `macro_rules!` macro, or a crate of the list. |
| A crate off the closed list that runs at compile time | A script, a macro or what they depend on. What only runs in WASM need not be there. | Remove it, or stay on the versions of the list. |
| A `git` dependency, or one from another registry | The lock must come from crates.io alone. | Publish it on crates.io. |
| A `path` that leaves the folder | The store reads the plugin’s folder alone. | Keep your subcrates in the folder. |
| `[patch]`, `[replace]` | The module would no longer be what the lock says. | Remove them. |
| `.cargo/config.toml`, `rust-toolchain`, `[profile.*.package]`, `build-override` | The toolchain and the profile are imposed. | Remove them. A plain `[profile]` is allowed, and ignored. |
| Anything but a `cdylib` (an `rlib` beside it is allowed); a `[[bin]]`, `src/main.rs`, `src/bin` | A plugin is a module, without `main`. | `crate-type = ["cdylib"]`. |
| `module!` missing | The module has no `escouade-turn` export. | The `module!` line. |

> [!WARNING]
> The closed list names versions: `serde` 1.0.229, `syn` 3.0.9… A `cargo update` that raises one of these crates can get the plugin refused (“… is not on the reviewed list”). Run `escouade-plugin validate` and `build` after each update of the lock. The template writes a `Cargo.lock` that passes.

## Next

- [Testing a plugin](plugins/testing): `cargo test`, the bench and the fake Escouade.
- [Publishing on the store](plugins/publishing): the tag, the checks, the review.
