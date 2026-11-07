# Testing a plugin

Escouade gives plugin authors a command, `escouade-plugin`, that runs a plugin without the app: the **conformance bench** checks it against what Escouade expects of it, and the **fake Escouade** sends it facts and requests by hand. The bench is the one Escouade plays on its own plugins.

## The command

`escouade-plugin` ships with Escouade, beside its program: `%LOCALAPPDATA%\Escouade\escouade-plugin.exe` on Windows, `Escouade.app/Contents/MacOS/escouade-plugin` on macOS, `/usr/bin/escouade-plugin` with the `.deb` package. For a `run.js` or `run.wasm` plugin, it needs neither Node nor Rust: it compiles its module as the store does and plays it in WASM, in the app’s host. For a plugin started by a command, it needs what that program needs to start.

```sh
escouade-plugin validate my-plugin   # the manifest, field by field
escouade-plugin test my-plugin       # the conformance bench
escouade-plugin dev my-plugin        # the fake Escouade
```

`--lang fr` or `--lang en` picks the language of its messages; without it, the system’s.

`--escouade <version>` plays the plugin as that version of Escouade, with `validate`, `test` and `dev`: the manifest’s `escouade` range is checked against it, and it is what the plugin sees in `initialize`. Without it, the command’s own version. Use it to write for a version to come:

```sh
escouade-plugin test my-plugin --escouade 2.2.0
```

## The conformance bench

`escouade-plugin test <folder>` reads the manifest, starts the program of `run` the way Escouade starts it (in its folder, with its three variables, with everything it starts), and plays up to eleven scenarios. Each says what happened; a scenario that fails also says **what is missing**.

| Scenario | What the bench does | What it expects |
| --- | --- | --- |
| Manifest | Reads `escouade-plugin.json`. | A manifest that holds, for this version of Escouade. |
| Initialization | Sends `initialize`. | `{"protocolVersion": 1, "capabilities": {…}}` within 10 s. |
| Version refused | Starts the plugin again and offers it version 999 of the protocol. | Its own version, or an error: never the version offered. |
| Delays | Times every call of the bench, and calls `bench/unknown`, which no plugin serves. | Each answer within its method’s delay; the error `-32601` at once to an unknown method. |
| A fact received | Sends a fact of `permissions.events` (the notification `events/fact`), then a call. | No answer to the notification, no crash, and the answer to the next call. |
| A decision | Sends `permissions/decide` for an `npm test` command. | `{"decision": "allow" \| "deny" \| "ask"}`, with a `message` for a denial. |
| A secret | Serves `secrets/get` throughout the bench, with a test value. | Only the keys of `permissions.secrets`, and never their value written on standard error or in a `log`. |
| Crash | Kills the plugin and everything it started in the middle of a call, then starts it again. | A new process that initializes like the first. |
| Shutdown | Sends `shutdown`. | An answer within a second, then the end of the process. |
| Ticket source | Only if `contributes.ticketSources` is declared: `account/me` and `tickets/containers`. | An answer within the delay, or an error the contract names: `-(32000 + HTTP status)` (`-32401`: token refused), `-32010` (a lasting refusal), `-32601` (a call it does not serve). |
| Code host | Only if `contributes.forges` is declared: `account/me` and `forge/pullRequestsOfBranch`. | The same. |

A scenario that does not concern the plugin is not played, and says so: no fact without `permissions.events`, no decision without `permissions.decide`, no source or host without `contributes.ticketSources` or `contributes.forges`, and nothing of a program for a declarative plugin, of which only the manifest is read.

```text
Conformance bench: dev.exemple.slow-rules 1.0.0

✓ Manifest         holds, with a program
✓ Initialization   answers in 61 ms, protocol 1
✓ Version refused  offered version 999, it answers its own (1): Escouade stops it
✗ Delays           permissions/decide: answered in 812 ms, past its delay of 500 ms
                   What is missing: answer within each method's delay (500 ms for a decision, 5 s in general); past it, Escouade no longer waits and its default answer applies.
– A fact received  it asks for no fact (`permissions.events`)
✓ A decision       "allow" in 812 ms
– A secret         it asks for no secret (`permissions.secrets`)
✓ Crash            killed in the middle of a call, it starts again: answers in 58 ms, protocol 1
✓ Shutdown         answers in 2 ms, then ends

6 passed, 1 failed, 2 not played
```

The exit code is `1` when a scenario fails, `0` otherwise: the command works as is in CI.

- `--json` gives the report in JSON: each scenario, its verdict (`pass`, `fail`, `skip`), what happened, what is missing, how long each call took, and what the plugin did on its own (its standard error, its logs, the secrets it asked for; the last 200). That is what the “Verified by Escouade” review reads, which plays the bench in a container without network: a connection or a write refused shows there.
- `--measure [n]` then measures `n` round trips (1,000 by default) and gives their median and 99th percentile, in microseconds.

### The bench’s facts and decisions

The bench sends facts and decisions in the shape Escouade will give them: the project, the ticket and the agent by their ids and names, never the conversation.

```json
{"jsonrpc":"2.0","method":"events/fact","params":{"name":"ticket.column","version":1,"at":"2026-10-29T10:00:00Z","project":{"id":"bench-project","name":"Banc","private":false},"ticket":{"id":"bench-ticket","key":"BEN-1","title":"Un ticket du banc","from":"doing","column":"test"},"agent":{"id":"bench-agent","name":"Agent du banc"}}}
{"jsonrpc":"2.0","id":3,"method":"permissions/decide","params":{"request":{"id":"bench-request","kind":"command","tool":"Bash","input":{"command":"npm test"}},"project":{"id":"bench-project","name":"Banc","private":false},"ticket":{"id":"bench-ticket","key":"BEN-1","title":"Un ticket du banc"},"agent":{"id":"bench-agent","name":"Agent du banc"}}}
```

A fact is a notification: it has no `id`, and the plugin does not answer it. The facts a plugin receives in Escouade, and their fields, come with the events, in an upcoming version; ignore the fields you do not know.

## The fake Escouade

`escouade-plugin dev <folder>` starts the plugin, initializes it, then sends what you type, line by line, and shows what comes back: its answers and how long they took, the secrets it asks for, its `log` lines, its standard error.

| Line | What is sent |
| --- | --- |
| `fact <name> [json]` | The fact `<name>` (`events/fact`); the JSON replaces its fields, for instance `fact ticket.column {"project":{"id":"p","name":"Private","private":true}}`. |
| `decide [json]` | A permission request (`permissions/decide`), and its answer; an answer past 500 ms is pointed out. |
| `call <method> [json]` | A call, and its answer. |
| `notify <method> [json]` | A notification. |
| `restart` | Kills the plugin and starts it again. |
| `quit` | `shutdown`, then the end. |

`--secret <key>=<value>` gives a secret’s value (repeatable); a key of the plugin without a value gets `null`, as in Escouade when nothing is kept, and a key outside its `permissions.secrets` is refused with `-32001`.

```text
$ escouade-plugin dev slack --secret webhook=https://example.invalid
slack/slack.cmd (pid 30112)
→ initialize
← {"capabilities":{},"protocolVersion":1}  (47 ms)
> fact ticket.column
→ events/fact {"agent":{…},"name":"ticket.column","project":{…},"ticket":{…},"version":1}
→ secrets/get "webhook"
→ log [info] no Slack webhook kept: nothing sent
> call nope {"a":1}
← error -32601 unknown method nope  (0 ms)
> quit
→ shutdown
← null  (0 ms)
```

## A plugin in Rust

Three levels, from the quickest to the truest:

```sh
cargo test                                 # the plugin, natively, on your machine
cargo check --target wasm32-wasip1         # it compiles as a module (rustup target add wasm32-wasip1)
escouade-plugin test my-plugin             # the bench, on the module
```

- **`cargo test`** runs natively, with tokio and reqwest: the same source as the module, without its host. The kit’s `serve_on` plays the plugin on lines in memory, the ones Escouade sends. It needs a C compiler, for `ring`. The template’s tests:
`sdk/create-escouade-plugin/templates/rust/src/tests.rs` (excerpt)

```rust
/// What the plugin writes for `messages`, one message a line.
fn play(messages: &[Value]) -> Vec<Value> {
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (output, written) = tokio::io::duplex(64 * 1024);
        serve_on(Arc::new(Template), input.as_bytes(), output)
            .await
            .unwrap();
        let mut lines = BufReader::new(written).lines();
        let mut read = Vec::new();
        while let Some(line) = lines.next_line().await.unwrap() {
            read.push(serde_json::from_str(&line).unwrap());
        }
        read
    })
}
```

- **`wasm32-wasip1`** is the module’s target. `cargo check --target wasm32-wasip1` says fast what does not compile there: `tokio`, `std::process`, a dependency that assumes a system. It is not the published module: that one comes out of `escouade-plugin build`.
- **The bench and the fake Escouade** compile the module as the store does, then play it in the app’s host. A store refusal shows here: no `Cargo.lock`, a crate off the list. The toolchain, rustc 1.98.0 and `wasm32-wasip1`, is downloaded the first time it is needed into `~/.escouade/tools` (650 MB, a minute) and checked at each use, without rustup. `ESCOUADE_RUST_TOOLCHAIN` names another one.

`--wasm <module>` plays a given module instead of compiling; `--offline` refuses every network call, as the review does; `--account <address>` gives `{account}` an address. The bench serves `fs/*` and `proc/*` around an agent of its own, and `secrets/get`. It does not serve `settings/get`, which answers `-32601`: a plugin must survive that.

## In CI

The bench fits in one step: install Escouade, then

```sh
escouade-plugin validate .
escouade-plugin test .
```

Escouade’s tests play this bench on its fake plugin and on its own plugins (`plugins/` of its repository), with `--first-party`: their ids `dev.escouade.*` are reserved, the bench refuses them to any other plugin. The [TypeScript SDK](plugins/sdk) (`@escouade-dev/plugin`) installs the same command in a plugin’s `node_modules`, and its CI plays this bench on the templates of `npm create escouade-plugin`.
