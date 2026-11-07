# The calls, in TS and in Rust

Each method of the host, with a short example in both languages: the network, the sockets, the timers, the files, the programs, the secrets, the settings and the journal. For the form of the messages, see the protocol.

## How to read this page

The examples come from two plugins of Escouade’s repository that do the same thing, `sdk/doc-examples/ts` and `sdk/doc-examples/rust`. They are compiled and pass the [bench](plugins/testing) at each change.

- **In TypeScript**, `p` is the SDK’s `Plugin`: each function gets the one from `createPlugin()`.
- **In Rust**, `cx` is the kit’s `Cx`, which each call gets. `cx.request` sends a method and waits for its answer, `cx.notify` sends a notification. What the host sends by itself, like `net/chunk`, comes to `Plugin::call`, like any call:

`sdk/doc-examples/rust/src/lib.rs` (excerpt)

```rust
                // What the host sends by itself comes here, like any call.
                "net/chunk" => net::chunk(cx, &params),
                "net/end" => net::end(cx, &params),
                "ws/message" => socket::message(cx, &params),
                "ws/closed" => socket::closed(cx, &params),
                "proc/out" => procs::out(cx, &params),
                "proc/exit" => procs::exit(cx, &params),
```

A refused call answers an error: `RpcError` in TypeScript, `Refusal` in Rust, with the same code (`-32003` for what is refused, `-32004` for what fails on the way, `-32005` for a missing file, `-32602` for parameters that do not hold). What each capability allows is in [the host’s capabilities](plugins/capabilities).

## The events

The plugin gets `events/fact` for each fact of `permissions.events`, and `permissions/decide` if it has `permissions.decide`. The list of facts is in [the events](plugins/events), the rules of a decision in [the permission rules](plugins/permission-rules). The plugin of both templates of `npm create escouade-plugin` does both:

`sdk/create-escouade-plugin/templates/program/src/plugin.ts` (excerpt)

```ts
// A fact of permissions.events: a ticket changed columns (its data says from and to).
plugin.onEvent('ticket.column', (fact, p) => {
  // A ticket's fact says its ticket; the fact's type, one for every fact, leaves it optional.
  if (fact.data.to === 'review') p.log.info(`${fact.ticket?.key ?? ''} is ready to test (${fact.project?.name ?? ''})`);
});
```

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (excerpt)

```rust
/// A fact of `permissions.events`: a ticket changed columns (its data says from and to).
fn fact(cx: &Cx, fact: &Value) {
    if fact["name"] == "ticket.column" && fact["data"]["to"] == "review" {
        let key = fact["ticket"]["key"].as_str().unwrap_or_default();
        let project = fact["project"]["name"].as_str().unwrap_or_default();
        cx.log("info", &format!("{key} is ready to test ({project})"));
    }
}
```

A decision answers within 500 ms:

`sdk/create-escouade-plugin/templates/program/src/plugin.ts` (excerpt)

```ts
// A permission an agent asks for, before the user is asked (permissions.decide). Answer within 500 ms.
plugin.decidePermission(({ request }) => {
  const command = typeof request.input?.command === 'string' ? request.input.command : '';
  if (request.kind === 'command' && /\bgit\s+push\b.*(--force\b|\s-f\b)/.test(command)) {
    return { decision: 'deny', message: 'No force push in this team: push a new commit instead.' };
  }
  // Leaves it to the user, as without the plugin.
  return { decision: 'ask' };
});
```

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (excerpt)

```rust
/// A permission an agent asks for, before the user is asked (`permissions.decide`). Answer within 500 ms.
fn decide(params: &Value) -> Value {
    let request = &params["request"];
    let command = request["input"]["command"].as_str().unwrap_or_default();
    if request["kind"] == "command" && is_force_push(command) {
        return json!({
            "decision": "deny",
            "message": "No force push in this team: push a new commit instead.",
        });
    }
    // Leaves it to the user, as without the plugin.
    json!({ "decision": "ask" })
}
```

## The network

### `net/fetch`

`{ url, method?, headers?, body? | bodyBase64? }` → `{ status, headers, body | bodyBase64 }`. A UTF-8 response comes in `body`, any other, whole, in `bodyBase64`. 16 calls at once per plugin: the next ones wait for their turn.

`sdk/doc-examples/ts/src/net.ts` (excerpt)

```ts
// The standard fetch, which goes through Escouade in WASM.
export async function issue(p: Plugin, id: string) {
  const token = await p.secret('token');
  const res = await fetch(`https://api.example.dev/issues/${id}`, {
    headers: { authorization: `Bearer ${token ?? ''}` },
  });
  return { status: res.status, body: await res.text() };
}
```

`sdk/doc-examples/rust/src/net.rs` (excerpt)

```rust
/// reqwest on your machine, `net/fetch` in the module: same source.
pub async fn issue(cx: &Cx, id: &str) -> Result<Value, Refusal> {
    let refused = |e: reqwest::Error| Refusal {
        code: -32004,
        message: e.to_string(),
    };
    let token = cx.secret("token").await?.unwrap_or_default();
    let response = reqwest::Client::new()
        .get(format!("https://api.example.dev/issues/{id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(refused)?;
    let status = response.status().as_u16();
    let body = response.text().await.map_err(refused)?;
    Ok(json!({ "status": status, "body": body }))
}
```

### `net/stream`

A response read as it comes. `net/stream { url, method?, headers?, body?, sse? }` → `{ stream, status, headers }`, then the notifications `net/chunk { stream, data | dataBase64 }` and `net/end { stream, error? }`. The plugin gives back what it read by `net/credit { stream, bytes }`, or lets the stream go by `net/cancel { stream }`. Cutting SSE into events stays with the plugin.

The SDK’s `openStream` does all that:

`sdk/doc-examples/ts/src/net.ts` (excerpt)

```ts
// Each piece taken gives its bytes back to Escouade, which reads no further than 256 KiB ahead of the plugin.
export async function follow(p: Plugin, url: string) {
  const stream = await openStream(p, url, { sse: true });
  for await (const piece of stream) p.log.debug(`${piece.data?.length ?? 0} characters`);
}
```

In Rust, you write it by hand. Without credit given back, the host stops reading at 256 KiB:

`sdk/doc-examples/rust/src/net.rs` (excerpt)

```rust
/// Opens a stream: its pieces come as `net/chunk`, its end as `net/end`.
pub async fn follow(cx: &Cx, url: &str) -> Result<Value, Refusal> {
    cx.request("net/stream", json!({ "url": url, "sse": true }))
        .await
}

/// A piece, read: its bytes go back to Escouade, which reads no further than 256 KiB ahead of the plugin.
pub fn chunk(cx: &Cx, piece: &Value) {
    let text = piece["data"].as_str().map(str::len);
    let bytes = piece["dataBase64"].as_str().map(|b64| b64.len() / 4 * 3);
    let bytes = text.or(bytes).unwrap_or_default();
    cx.log("debug", &format!("{bytes} bytes"));
    cx.notify(
        "net/credit",
        json!({ "stream": piece["stream"], "bytes": bytes }),
    );
}

pub fn end(cx: &Cx, end: &Value) {
    if let Some(error) = end["error"].as_str() {
        cx.log("warn", error);
    }
}
```

## The sockets: `ws/*`

`ws/open { url, headers? }` → `{ socket }`, `ws/send { socket, text | dataBase64 }`, `ws/close { socket }`, then the notifications `ws/message { socket, text | dataBase64 }` and `ws/closed { socket, code, reason }`.

`sdk/doc-examples/ts/src/socket.ts` (excerpt)

```ts
// Its messages as they come, until it closes.
export async function live(p: Plugin) {
  const token = await p.secret('token');
  const socket = await openSocket(p, 'wss://api.example.dev/live', { authorization: `Bearer ${token ?? ''}` });
  await socket.send({ text: 'hello' });
  for await (const message of socket) {
    p.log.info(`the socket says ${message.text ?? ''}`);
    await socket.close();
  }
  return (await socket.closed).code;
}
```

`sdk/doc-examples/rust/src/socket.rs` (excerpt)

```rust
/// Opens a socket and says hello. What comes back arrives as `ws/message`, its end as `ws/closed`.
pub async fn open(cx: &Cx) -> Result<Value, Refusal> {
    let token = cx.secret("token").await?.unwrap_or_default();
    let opened = cx
        .request(
            "ws/open",
            json!({
                "url": "wss://api.example.dev/live",
                "headers": { "authorization": format!("Bearer {token}") },
            }),
        )
        .await?;
    let socket = &opened["socket"];
    cx.request("ws/send", json!({ "socket": socket, "text": "hello" }))
        .await?;
    Ok(socket.clone())
}

pub fn message(cx: &Cx, message: &Value) {
    let text = message["text"].as_str().unwrap_or_default();
    cx.log("info", &format!("the socket says {text}"));
}

pub fn closed(cx: &Cx, closed: &Value) {
    cx.log("info", &format!("closed: {}", closed["code"]));
}

pub async fn close(cx: &Cx, socket: &Value) -> Result<Value, Refusal> {
    cx.request("ws/close", json!({ "socket": socket })).await
}
```

## The timers: `timer/*`

`timer/set { ms }` → `{ timer }`, `timer/cancel { timer }`, then the notification `timer/fired { timer }`. You do not call them yourself: `setTimeout` and `clearTimeout` exist in JS, `sleep` and `timeout` in the kit. Waiting gives the hand back to Escouade, and the other calls go on.

`sdk/doc-examples/ts/src/timers.ts` (excerpt)

```ts
// Other calls go on while this one waits.
export async function later(p: Plugin) {
  await new Promise((resolve) => setTimeout(resolve, 1000));
  p.log.info('one second later');
  return 'rested';
}

// A timeout on anything that waits.
export async function patient(p: Plugin) {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const late = new Promise<never>((_, reject) => {
    timer = setTimeout(() => reject(new Error('timed out')), 2000);
  });
  try {
    return await Promise.race([p.secret('token'), late]);
  } finally {
    clearTimeout(timer);
  }
}
```

`sdk/doc-examples/rust/src/timers.rs` (excerpt)

```rust
/// Other calls go on while this one waits.
pub async fn later(cx: &Cx) -> Value {
    sleep(Duration::from_secs(1)).await;
    cx.log("info", "one second later");
    json!("rested")
}

/// A timeout on anything that waits.
pub async fn patient(cx: &Cx) -> Result<Option<String>, Refusal> {
    match timeout(Duration::from_secs(2), cx.secret("token")).await {
        Ok(secret) => secret,
        Err(elapsed) => Err(Refusal {
            code: -32004,
            message: elapsed.to_string(),
        }),
    }
}
```

## The files

### `fs/*`

A root of `permissions.files` and a relative path, or a path of the host under a root of an agent, with `agent`. `.` names the root.

| Method | More parameters | Answer |
| --- | --- | --- |
| `fs/read` | `tail` | `{ text }`, or `{ dataBase64 }` for what is not UTF-8. With `tail`, a file past the limit gives its end and `cut` instead of failing |
| `fs/write` | `text` or `dataBase64`, `append` | `{ bytes, path }` |
| `fs/stat` | | `{ kind, bytes, modified }` |
| `fs/list` | | `{ entries, truncated }`, each entry with `name`, `kind` and `bytes` |
| `fs/canon` | | `{ path }`, the path of the host |

`sdk/doc-examples/ts/src/files.ts` (excerpt)

```ts
// Writes in its own temporary folder (`permissions.files`, root `temp`).
export function keep(p: Plugin, text: string) {
  return p.fs.write({ root: 'temp', path: 'notes.txt', text, append: true });
}

// Reads it back, what is there and what the host calls it.
export async function notes(p: Plugin) {
  const file = { root: 'temp', path: 'notes.txt' } as const;
  return {
    text: await p.fs.text(file),
    stat: await p.fs.stat(file),
    canon: await p.fs.canon(file),
    list: await p.fs.list({ root: 'temp', path: '.' }),
  };
}
```

`sdk/doc-examples/rust/src/files.rs` (excerpt)

```rust
/// Writes in its own temporary folder (`permissions.files`, root `temp`).
pub async fn keep(cx: &Cx, text: &str) -> Result<Value, Refusal> {
    let note = json!({ "root": "temp", "path": "notes.txt", "text": text, "append": true });
    cx.request("fs/write", note).await
}

/// Reads it back, what is there and what the host calls it.
pub async fn notes(cx: &Cx) -> Result<Value, Refusal> {
    let file = json!({ "root": "temp", "path": "notes.txt" });
    let read = cx.request("fs/read", file.clone()).await?;
    let stat = cx.request("fs/stat", file.clone()).await?;
    let canon = cx.request("fs/canon", file).await?;
    let list = cx
        .request("fs/list", json!({ "root": "temp", "path": "." }))
        .await?;
    Ok(json!({ "read": read, "stat": stat, "canon": canon, "list": list }))
}
```

### `files/read`

A file of its own folder, without `permissions.files`: as UTF-8 text, 256 KiB at most. Never its manifest, a hidden name (`.env`) or a path that leaves the folder.

`sdk/doc-examples/ts/src/files.ts` (excerpt)

```ts
// A file of the plugin's own folder.
export function words(p: Plugin) {
  return p.file('data/words.txt');
}
```

`sdk/doc-examples/rust/src/files.rs` (excerpt)

```rust
/// A file of the plugin's own folder.
pub async fn words(cx: &Cx) -> Result<Value, Refusal> {
    cx.request("files/read", json!({ "path": "data/words.txt" }))
        .await
}
```

## The programs: `proc/*`

`proc/spawn { command, args?, dir?, agent?, project?, env? }` → `{ proc }`, `proc/write { proc, data, close? }`, `proc/kill { proc }`, then the notifications `proc/out { proc, stream, data }` and `proc/exit { proc, code }`. `proc/out` may come before the answer to `proc/spawn`.

`command` and `args` must be a line of `permissions.commands`:

`sdk/doc-examples/rust/escouade-plugin.json` (excerpt)

```json
    "commands": ["git status --short", "git log *"]
```

`sdk/doc-examples/ts/src/procs.ts` (excerpt)

```ts
// Runs a line of `permissions.commands` in a project: what it writes comes to the function, `exited` says how it ended.
export function status(p: Plugin, project: string) {
  return p.runProgram({ command: 'git', args: ['status', '--short'], dir: 'project', project }, (data, stream) =>
    p.log.info(`${stream}: ${data}`),
  );
}
```

`sdk/doc-examples/rust/src/procs.rs` (excerpt)

```rust
/// Runs a line of `permissions.commands` in a project: what it writes comes as `proc/out`, its end as `proc/exit`.
pub async fn status(cx: &Cx, project: &str) -> Result<Value, Refusal> {
    let spawned = cx
        .request(
            "proc/spawn",
            json!({
                "command": "git",
                "args": ["status", "--short"],
                "dir": "project",
                "project": project,
            }),
        )
        .await?;
    Ok(spawned["proc"].clone())
}

pub fn out(cx: &Cx, out: &Value) {
    let data = out["data"].as_str().unwrap_or_default();
    cx.log("info", &format!("{}: {data}", out["stream"]));
}

pub fn exit(cx: &Cx, exit: &Value) {
    cx.log("info", &format!("it ended: {}", exit["code"]));
}
```

`proc/write` writes on its input, and `proc/kill` stops it with what it started:

`sdk/doc-examples/ts/src/procs.ts` (excerpt)

```ts
// What it reads on its input; `true` ends that input.
export async function input(p: Plugin) {
  const program = await p.runProgram({ command: 'git', args: ['log', '--oneline'] });
  await program.write('yes\n', true);
  return program.exited;
}

export async function stop(p: Plugin) {
  const program = await p.runProgram({ command: 'git', args: ['log', '--oneline'] });
  await program.kill();
}
```

`sdk/doc-examples/rust/src/procs.rs` (excerpt)

```rust
/// What it reads on its input; `close` ends that input.
pub async fn feed(cx: &Cx, proc: &Value) -> Result<Value, Refusal> {
    let input = json!({ "proc": proc, "data": "yes\n", "close": true });
    cx.request("proc/write", input).await
}

pub async fn stop(cx: &Cx, proc: &Value) -> Result<Value, Refusal> {
    cx.request("proc/kill", json!({ "proc": proc })).await
}
```

Running a program takes time: do not do it in a decision, which has 500 ms.

## The secrets and the settings

### `secrets/get`

`{ key }` → the secret, or `null` when none is kept. The key must be in `permissions.secrets`, and declared in `contributes.secrets`. Another one answers `-32001`. For an account: `account:<name>`, with `account:*` in `permissions.secrets`.

`sdk/doc-examples/ts/src/settings.ts` (excerpt)

```ts
// One of `permissions.secrets`; null while the user has not given it.
export function token(p: Plugin) {
  return p.secret('token');
}
```

`sdk/doc-examples/rust/src/settings.rs` (excerpt)

```rust
/// One of `permissions.secrets`; none while the user has not given it.
pub async fn token(cx: &Cx) -> Result<Option<String>, Refusal> {
    cx.secret("token").await
}
```

### `settings/get`

`{}` or `{ project }` → an object by key of `contributes.settings`, without those left at their default. With a project, what its `.escouade.yml` gives it under `plugins.<id>` comes first.

`sdk/doc-examples/ts/src/settings.ts` (excerpt)

```ts
// Its settings: with a project, what its `.escouade.yml` gives over what the user chose.
export async function channel(p: Plugin, project: string) {
  const settings = await p.settings(project);
  return settings.channel;
}
```

`sdk/doc-examples/rust/src/settings.rs` (excerpt)

```rust
/// Its settings: with a project, what its `.escouade.yml` gives over what the user chose.
pub async fn channel(cx: &Cx, project: &Value) -> Result<Value, Refusal> {
    let settings = cx
        .request("settings/get", json!({ "project": project }))
        .await?;
    Ok(settings["channel"].clone())
}
```

## The journal

`log { level, message }`, a notification: `debug`, `info`, `warn` or `error`. The journal is in English, like the app’s, and each secret given to the plugin is hidden there.

`sdk/doc-examples/ts/src/settings.ts` (excerpt)

```ts
// A line of its journal: `debug`, `info`, `warn` or `error`.
export function journal(fact: Fact, p: Plugin) {
  p.log.info(`${fact.name} received`);
}
```

`sdk/doc-examples/rust/src/settings.rs` (excerpt)

```rust
/// A line of its journal: `debug`, `info`, `warn` or `error`.
pub fn journal(cx: &Cx, fact: &Value) {
    cx.log("info", &format!("{} received", fact["name"]));
}
```

What the plugin writes on its standard error goes there too: see [the journal](plugins/api#the-journal).
