# The protocol

A plugin with a program talks to Escouade in JSON-RPC 2.0, on its standard input and output. This page describes that transport: when the program starts, the handshake, the delays, the restarts, the memory, the secrets and the journal.

## When the program starts

- **At its first activation, never before.** Escouade starts the program of `run` when a fact of its `activation` list happens (`event:ticket.column`), or at the first call it makes to it. A plugin without `activation` does not run; a plugin without `run` is declarative: nothing runs.
- **One process per plugin.** It serves every project and every agent where the plugin is enabled: Escouade tells its calls apart by their `id`.
- **In the plugin’s folder**, with three variables: `ESCOUADE_PLUGIN_ID` (its id), `ESCOUADE_PLUGIN_API` (the protocol’s version, `1`) and `ESCOUADE_VERSION` (Escouade’s).
- **With everything it starts.** On Windows, the process is in a Job object; on macOS and Linux, it leads its process group. When Escouade stops it, what it started stops with it; on Windows, even if Escouade crashes.

## The messages

- JSON-RPC 2.0, **one message per line**: JSON in UTF-8, without a line break, ended by `\n`. The standard output is for the protocol only; write your traces on the standard error, which goes to the [journal](#the-journal).
- **Batches both ways**: a JSON array of messages, on one line. Answer a batch with a batch; the answers can come in any order, each with the `id` of its request.
- A notification has no `id` and expects no answer.

```json
{"jsonrpc":"2.0","id":7,"method":"echo","params":{"a":1}}
[{"jsonrpc":"2.0","id":8,"method":"echo","params":1},{"jsonrpc":"2.0","method":"log","params":{"level":"info","message":"ready"}}]
```

## The handshake

Escouade’s first message is `initialize`. It gives the version of the protocol it speaks and what it can do:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "escouade": { "version": "2.1.1" },
    "plugin": { "id": "dev.exemple.slack", "version": "1.2.0" },
    "capabilities": { "batch": true, "secrets": true, "log": true, "attach": true }
  }
}
```

The plugin answers the version it speaks and its own capabilities:

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"capabilities":{"batch":true}}}
```

- Another version, or an answer without a version, stops the plugin, with the reason: “it speaks version 2 of the protocol; this Escouade speaks version 1”.
- Without an answer within 10 seconds, the plugin is stopped too.
- Nothing else is sent to it before its answer: what comes meanwhile waits in a queue.

## The delays

Every call of Escouade has a delay. Past it, Escouade stops waiting: **its default answer applies** (a permission request goes to the user, as without a plugin), and a late answer is ignored.

| Method | Delay |
| --- | --- |
| `initialize` | 10 s |
| `shutdown` | 1 s |
| `permissions/…` (a decision an agent waits for) | 500 ms |
| `tickets/…`, `forge/…`, `engine/oneShot` (an import, a call to a service) | 60 s |
| The others | 5 s |

Every delay that runs out is noted in the journal: “test/do: no answer within 300 ms”.

## What Escouade serves the plugin

| Method | Parameters | Answer |
| --- | --- | --- |
| `secrets/get` | `{ "key": "webhook" }` | The secret, or `null` if none is kept under that key. |
| `log` (notification) | `{ "level": "info", "message": "…" }` | None: the line goes to the plugin’s journal. |

Any other method answers the error `-32601` and notes it in the journal. Escouade’s own errors: `-32001` for a secret that is not the plugin’s, `-32002` for a keychain out of reach, `-32602` for parameters that do not hold.

## The secrets

A plugin’s secrets are in the system’s keychain, under a name of its own: `plugin:<id>:<key>`. It asks for them with `secrets/get`, when it needs them.

- **Only its own.** The key must be one of its `permissions.secrets`; a pattern ending with `*` takes in several (`account:*`). Any other key is refused with `-32001` and noted in the journal, even another plugin’s written in full: `plugin:dev.other:token` is not a key of its permissions.
- A plugin never gets the secret of an account, a harness or an integration of Escouade.
- A secret given to the plugin is masked (`••••`) everywhere in its journal, its standard error included.

## Crashes and restarts

- **A crash restarts the plugin**, three times within five minutes. At the fourth, it is stopped: “stopped after 4 crashes within 5 minutes”. It no longer starts by itself; the “Restart” button of Settings › Plugins will start it again, its crashes forgotten.
- **The calls under way** when it crashes fail: their default answer applies.
- **While it restarts**, what Escouade sends it waits in a queue of 256 messages, then goes in order once `initialize` is answered. A full queue loses its oldest message, and the journal says so.
- **What each agent had attached to it** (for a harness, `agent/attach`: its session, its mode, its model) is given again to the new process before any other message.

## The memory

Escouade reads the memory of the plugin and of everything it started every two seconds, as it does the agents’. Past 1 GB, the plugin is stopped, without a restart: “stopped: 1100 MB of memory, past the limit of 1024 MB”.

## When Escouade closes

When Escouade closes, it sends `shutdown` to each running plugin, waits for its answer a second at most for all of them, then stops its process and everything it started. Answer `shutdown`, then exit: what you have not written before is lost.

## The journal

Each plugin has its journal, in `~/.escouade/plugin-logs/<id>.log`; past 1 MB, it becomes `<id>.log.1` and starts over. Its lines are in English, like the app’s log, and note:

- its starts, its answer to `initialize`, its stops and their reason;
- its crashes, with their exit code and its last lines of standard error;
- each line of its standard error;
- its messages: its `log`s, the secrets it asked for (never their value);
- the calls left without an answer, what it asked for and was refused, the messages lost from a full queue.

The last 500 lines serve the Settings › Plugins tab, which comes in a later version.
