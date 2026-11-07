# Les appels, en TS et en Rust

Chaque méthode de l’hôte, avec un exemple court dans les deux langages : le réseau, les sockets, les minuteurs, les fichiers, les programmes, les secrets, les réglages et le journal. Pour la forme des messages, voir le protocole.

## Comment lire cette page

Les exemples viennent de deux plugins du dépôt d’Escouade qui font la même chose, `sdk/doc-examples/ts` et `sdk/doc-examples/rust`. Ils sont compilés et passent le [banc](plugins/testing) à chaque changement.

- **En TypeScript**, `p` est le `Plugin` du SDK : chaque fonction reçoit celui de `createPlugin()`.
- **En Rust**, `cx` est le `Cx` du kit, que reçoit chaque appel. `cx.request` envoie une méthode et attend sa réponse, `cx.notify` envoie une notification. Ce que l’hôte envoie de lui-même, comme `net/chunk`, arrive dans `Plugin::call`, comme tout appel :

`sdk/doc-examples/rust/src/lib.rs` (extrait)

```rust
                // What the host sends by itself comes here, like any call.
                "net/chunk" => net::chunk(cx, &params),
                "net/end" => net::end(cx, &params),
                "ws/message" => socket::message(cx, &params),
                "ws/closed" => socket::closed(cx, &params),
                "proc/out" => procs::out(cx, &params),
                "proc/exit" => procs::exit(cx, &params),
```

Un appel refusé répond une erreur : `RpcError` en TypeScript, `Refusal` en Rust, avec le même code (`-32003` pour ce qui est refusé, `-32004` pour ce qui échoue en route, `-32005` pour un fichier absent, `-32602` pour des paramètres qui ne tiennent pas). Ce que chaque capacité permet est dans [les capacités de l’hôte](plugins/capabilities).

## Les événements

Le plugin reçoit `events/fact` pour chaque fait de `permissions.events`, et `permissions/decide` s’il a `permissions.decide`. La liste des faits est dans [les événements](plugins/events), les règles d’une décision dans [les règles d’autorisation](plugins/permission-rules). Le plugin des deux modèles de `npm create escouade-plugin` fait les deux :

`sdk/create-escouade-plugin/templates/program/src/plugin.ts` (extrait)

```ts
// A fact of permissions.events: a ticket changed columns (its data says from and to).
plugin.onEvent('ticket.column', (fact, p) => {
  // A ticket's fact says its ticket; the fact's type, one for every fact, leaves it optional.
  if (fact.data.to === 'review') p.log.info(`${fact.ticket?.key ?? ''} is ready to test (${fact.project?.name ?? ''})`);
});
```

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (extrait)

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

Une décision répond en 500 ms au plus :

`sdk/create-escouade-plugin/templates/program/src/plugin.ts` (extrait)

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

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (extrait)

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

## Le réseau

### `net/fetch`

`{ url, method?, headers?, body? | bodyBase64? }` → `{ status, headers, body | bodyBase64 }`. Une réponse UTF-8 vient dans `body`, une autre, entière, dans `bodyBase64`. 16 appels à la fois par plugin : les suivants attendent leur tour.

`sdk/doc-examples/ts/src/net.ts` (extrait)

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

`sdk/doc-examples/rust/src/net.rs` (extrait)

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

Une réponse lue au fil de l’eau. `net/stream { url, method?, headers?, body?, sse? }` → `{ stream, status, headers }`, puis les notifications `net/chunk { stream, data | dataBase64 }` et `net/end { stream, error? }`. Le plugin rend ce qu’il a lu par `net/credit { stream, bytes }`, ou lâche le flux par `net/cancel { stream }`. Le découpage du SSE reste au plugin.

`openStream` du SDK fait tout cela :

`sdk/doc-examples/ts/src/net.ts` (extrait)

```ts
// Each piece taken gives its bytes back to Escouade, which reads no further than 256 KiB ahead of the plugin.
export async function follow(p: Plugin, url: string) {
  const stream = await openStream(p, url, { sse: true });
  for await (const piece of stream) p.log.debug(`${piece.data?.length ?? 0} characters`);
}
```

En Rust, on l’écrit à la main. Sans crédit rendu, l’hôte s’arrête de lire à 256 Kio :

`sdk/doc-examples/rust/src/net.rs` (extrait)

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

## Les sockets : `ws/*`

`ws/open { url, headers? }` → `{ socket }`, `ws/send { socket, text | dataBase64 }`, `ws/close { socket }`, puis les notifications `ws/message { socket, text | dataBase64 }` et `ws/closed { socket, code, reason }`.

`sdk/doc-examples/ts/src/socket.ts` (extrait)

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

`sdk/doc-examples/rust/src/socket.rs` (extrait)

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

## Les minuteurs : `timer/*`

`timer/set { ms }` → `{ timer }`, `timer/cancel { timer }`, puis la notification `timer/fired { timer }`. On ne les appelle pas soi-même : `setTimeout` et `clearTimeout` existent en JS, `sleep` et `timeout` dans le kit. Attendre rend la main à Escouade, et les autres appels continuent.

`sdk/doc-examples/ts/src/timers.ts` (extrait)

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

`sdk/doc-examples/rust/src/timers.rs` (extrait)

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

## Les fichiers

### `fs/*`

Une racine de `permissions.files` et un chemin relatif, ou un chemin de l’hôte sous une racine d’un agent, avec `agent`. `.` nomme la racine.

| Méthode | Paramètres en plus | Réponse |
| --- | --- | --- |
| `fs/read` | `tail` | `{ text }`, ou `{ dataBase64 }` pour ce qui n’est pas de l’UTF-8. Avec `tail`, un fichier au-delà de la limite rend sa fin et `cut` au lieu d’échouer |
| `fs/write` | `text` ou `dataBase64`, `append` | `{ bytes, path }` |
| `fs/stat` | | `{ kind, bytes, modified }` |
| `fs/list` | | `{ entries, truncated }`, chaque entrée avec `name`, `kind` et `bytes` |
| `fs/canon` | | `{ path }`, le chemin de l’hôte |

`sdk/doc-examples/ts/src/files.ts` (extrait)

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

`sdk/doc-examples/rust/src/files.rs` (extrait)

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

Un fichier de son propre dossier, sans `permissions.files` : en texte UTF-8, 256 Kio au plus. Jamais son manifeste, un nom caché (`.env`) ou un chemin qui sort du dossier.

`sdk/doc-examples/ts/src/files.ts` (extrait)

```ts
// A file of the plugin's own folder.
export function words(p: Plugin) {
  return p.file('data/words.txt');
}
```

`sdk/doc-examples/rust/src/files.rs` (extrait)

```rust
/// A file of the plugin's own folder.
pub async fn words(cx: &Cx) -> Result<Value, Refusal> {
    cx.request("files/read", json!({ "path": "data/words.txt" }))
        .await
}
```

## Les programmes : `proc/*`

`proc/spawn { command, args?, dir?, agent?, project?, env? }` → `{ proc }`, `proc/write { proc, data, close? }`, `proc/kill { proc }`, puis les notifications `proc/out { proc, stream, data }` et `proc/exit { proc, code }`. `proc/out` peut arriver avant la réponse à `proc/spawn`.

`command` et `args` doivent être une ligne de `permissions.commands` :

`sdk/doc-examples/rust/escouade-plugin.json` (extrait)

```json
    "commands": ["git status --short", "git log *"]
```

`sdk/doc-examples/ts/src/procs.ts` (extrait)

```ts
// Runs a line of `permissions.commands` in a project: what it writes comes to the function, `exited` says how it ended.
export function status(p: Plugin, project: string) {
  return p.runProgram({ command: 'git', args: ['status', '--short'], dir: 'project', project }, (data, stream) =>
    p.log.info(`${stream}: ${data}`),
  );
}
```

`sdk/doc-examples/rust/src/procs.rs` (extrait)

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

`proc/write` écrit sur son entrée, et `proc/kill` l’arrête avec ce qu’il a lancé :

`sdk/doc-examples/ts/src/procs.ts` (extrait)

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

`sdk/doc-examples/rust/src/procs.rs` (extrait)

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

Lancer un programme prend du temps : ne le fais pas dans une décision, qui a 500 ms.

## Les secrets et les réglages

### `secrets/get`

`{ key }` → le secret, ou `null` si aucun n’est gardé. La clé doit être dans `permissions.secrets`, et déclarée dans `contributes.secrets`. Une autre répond `-32001`. Pour un compte : `account:<nom>`, avec `account:*` dans `permissions.secrets`.

`sdk/doc-examples/ts/src/settings.ts` (extrait)

```ts
// One of `permissions.secrets`; null while the user has not given it.
export function token(p: Plugin) {
  return p.secret('token');
}
```

`sdk/doc-examples/rust/src/settings.rs` (extrait)

```rust
/// One of `permissions.secrets`; none while the user has not given it.
pub async fn token(cx: &Cx) -> Result<Option<String>, Refusal> {
    cx.secret("token").await
}
```

### `settings/get`

`{}` ou `{ project }` → un objet par clé de `contributes.settings`, sans celles laissées à leur valeur par défaut. Avec un projet, ce que son `.escouade.yml` lui donne sous `plugins.<id>` passe devant.

`sdk/doc-examples/ts/src/settings.ts` (extrait)

```ts
// Its settings: with a project, what its `.escouade.yml` gives over what the user chose.
export async function channel(p: Plugin, project: string) {
  const settings = await p.settings(project);
  return settings.channel;
}
```

`sdk/doc-examples/rust/src/settings.rs` (extrait)

```rust
/// Its settings: with a project, what its `.escouade.yml` gives over what the user chose.
pub async fn channel(cx: &Cx, project: &Value) -> Result<Value, Refusal> {
    let settings = cx
        .request("settings/get", json!({ "project": project }))
        .await?;
    Ok(settings["channel"].clone())
}
```

## Le journal

`log { level, message }`, une notification : `debug`, `info`, `warn` ou `error`. Le journal est en anglais, comme celui de l’app, et chaque secret donné au plugin y est masqué.

`sdk/doc-examples/ts/src/settings.ts` (extrait)

```ts
// A line of its journal: `debug`, `info`, `warn` or `error`.
export function journal(fact: Fact, p: Plugin) {
  p.log.info(`${fact.name} received`);
}
```

`sdk/doc-examples/rust/src/settings.rs` (extrait)

```rust
/// A line of its journal: `debug`, `info`, `warn` or `error`.
pub fn journal(cx: &Cx, fact: &Value) {
    cx.log("info", &format!("{} received", fact["name"]));
}
```

Ce que le plugin écrit sur sa sortie d’erreur y va aussi : voir [le journal](plugins/api#le-journal).
