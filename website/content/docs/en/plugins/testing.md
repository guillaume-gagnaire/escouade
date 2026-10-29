# Testing a plugin

Escouade gives plugin authors a command, `escouade-plugin`, that runs a plugin without the app: the **conformance bench** checks it against what Escouade expects of it, and the **fake Escouade** sends it facts and requests by hand. The bench is the one Escouade plays on its own plugins.

## The command

`escouade-plugin` ships with Escouade, beside its program: `%LOCALAPPDATA%\Escouade\escouade-plugin.exe` on Windows, `Escouade.app/Contents/MacOS/escouade-plugin` on macOS, `/usr/bin/escouade-plugin` with the `.deb` package. It only needs what the plugin’s program needs to start (Node for a plugin in JavaScript).

```sh
escouade-plugin validate my-plugin   # the manifest, field by field
escouade-plugin test my-plugin       # the conformance bench
escouade-plugin dev my-plugin        # the fake Escouade
```

`--lang fr` or `--lang en` picks the language of its messages; without it, the system’s.

## The conformance bench

`escouade-plugin test <folder>` reads the manifest, starts the program of `run` the way Escouade starts it (in its folder, with its three variables, with everything it starts), and plays nine scenarios. Each says what happened; a scenario that fails also says **what is missing**.

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

A scenario that does not concern the plugin is not played, and says so: no fact without `permissions.events`, no decision without `permissions.decide`, and nothing of a program for a declarative plugin, of which only the manifest is read.

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

- `--json` gives the report in JSON: each scenario, its verdict (`pass`, `fail`, `skip`), what happened, what is missing, and how long each call took.
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

## In CI

The bench fits in one step: install Escouade, then

```sh
escouade-plugin validate .
escouade-plugin test .
```

Escouade’s tests play this bench on its fake plugin and on its example plugins. The TypeScript SDK (`@escouade/plugin`), coming with the rest of 2.1, will take up the same command and its scenarios.
