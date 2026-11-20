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

`proc/spawn { command, args?, dir?, agent?, project?, env? }` → `{ proc, pid }`, `proc/write { proc, data, close? }`, `proc/kill { proc }`, puis les notifications `proc/out { proc, stream, data }` et `proc/exit { proc, code }`.

`command` et `args` doivent être une ligne de `permissions.commands` :

`sdk/doc-examples/rust/escouade-plugin.json` (extrait)

```json
    "commands": ["git status --short", "git log *", "git --version"]
```

Un joker de l’entrée (`{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`, `{path:plugin}`) prend un seul mot de sa forme ([les jokers](plugins/capabilities#les-jokers)). `pid` est celui du programme, que `{pid}` admet tant qu’il tourne ; une Escouade sans `capabilities.typedCommands` ne le rend pas.

**Ce qui revient.**

- `proc/out { proc, stream, data }` : `proc` est celui que `proc/spawn` a rendu ; `stream` vaut `stdout` ou `stderr` ; `data` est du **texte**, jamais du base64 : les octets lus en UTF-8, coupés entre deux caractères (un octet qui n’est pas de l’UTF-8 devient U+FFFD). Un `proc/out` porte 16 Kio au plus et n’est pas une ligne : une ligne peut arriver en deux. Chaque sortie arrive dans l’ordre où elle a été écrite ; `stdout` et `stderr` ne sont pas ordonnées entre elles.
- `proc/out` peut arriver **avant** la réponse à `proc/spawn` : garde la sortie par son `proc`.
- `proc/exit { proc, code }` arrive une fois, **après** tous ses `proc/out` (sauf si ce qu’il a laissé tourner garde ses sorties ouvertes plus d’une seconde). `code` est son code de sortie, `null` s’il a été tué (`proc/kill`, la fin de l’instance) ou arrêté par un signal.
- La sortie n’est plus lue au-delà de 256 Kio qui n’ont pas atteint le module : un programme qui écrit sans fin ralentit, la mémoire de l’app ne grossit pas. Quatre programmes à la fois.

Lancer une ligne dans un projet (`dir: "project"`, l’id du projet d’un fait) :

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
```

**Un exemple complet** : lancer `git --version` et répondre ce qu’il a écrit. En TypeScript, `runProgram` garde la sortie et `exited` attend la fin :

`sdk/doc-examples/ts/src/procs.ts` (extrait)

```ts
// Runs `git --version` and answers what it wrote and how it ended: `data` is text, a piece of the output, not a line.
export async function version(p: Plugin) {
  let stdout = '';
  let stderr = '';
  const program = await p.runProgram({ command: 'git', args: ['--version'] }, (data, stream) => {
    if (stream === 'stderr') stderr += data;
    else stdout += data;
  });
  const code = await program.exited;
  return { stdout: stdout.trimEnd(), stderr, code };
}
```

En Rust, le plugin garde la sortie de chaque programme par son `proc`, et la réponse attend son `proc/exit`. Les notifications arrivent comme tout appel :

`sdk/doc-examples/rust/src/lib.rs` (extrait)

```rust
                "proc/out" => procs::out(cx, &params),
                "proc/exit" => procs::exit(cx, &params),
```

`sdk/doc-examples/rust/src/procs.rs` (extrait)

```rust
/// What a program wrote, by its `proc`, until its end.
#[derive(Default)]
pub struct Said {
    pub stdout: String,
    pub stderr: String,
    /// Its exit code once it ended; `null` when it was killed.
    pub code: Option<Value>,
    waiting: Option<Waker>,
}
```

Les tests d’Escouade construisent cet exemple comme un auteur (`escouade-plugin build`), le chargent et appellent `example/version` : la réponse est `{ "stdout": "git version …", "stderr": "", "code": 0 }`.

`proc/write` écrit sur son entrée, et `proc/kill` l’arrête avec ce qu’il a lancé :

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

`{ key, profile }` (Escouade 2.2.5, `capabilities.profiles`) → le champ secret `key` du profil `profile`, ou `null`. `profile` est l’identifiant que donnent `settings/get` ou `profiles/list`, et `permissions.secrets` doit avoir `profile:<genre>`. Un identifiant qui n’en est pas un ou que le plugin n’a pas, une clé qui n’est pas un champ secret de ce genre : `-32602` ; sans `profile:<genre>` : `-32001`. Une clé qui n’est qu’un champ secret d’un profil, demandée **sans** `profile`, est refusée (`-32602`) : jamais un autre secret à sa place.

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

Un réglage de `"scope": "project"` vient par ses rangs : la valeur de cette machine pour le projet, puis celle du fichier (sauf `"repo": false` ou `import`), puis `default`. Un réglage `"type": "profile"` rend l’identifiant du profil choisi, ou `null` ; jamais celui que nomme le fichier. D’un projet où le plugin n’est pas actif, seuls les défauts ([les réglages d’un projet](plugins/manifest#les-reglages-d-un-projet)).

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

### `profiles/list` et `profiles/get`

Escouade 2.2.5, `capabilities.profiles`. `{ kind }` → les profils de ce genre, dans l’ordre où l’utilisateur les a créés : leur identifiant, tiré par Escouade, et leur nom (`nameField`). `{ kind, profile }` → les champs simples d’un profil, jamais un secret, ou `null` s’il n’en a pas de cet identifiant. Un genre que le plugin ne déclare pas : `-32602`.

```json
{"jsonrpc":"2.0","id":7,"method":"profiles/list","params":{"kind":"vpn"}}
{"jsonrpc":"2.0","id":7,"result":[{"id":"0f3c9a1b2d4e5f60","name":"Client A"}]}
{"jsonrpc":"2.0","id":8,"method":"profiles/get","params":{"kind":"vpn","profile":"0f3c9a1b2d4e5f60"}}
{"jsonrpc":"2.0","id":8,"result":{"name":"Client A","server":"vpn.client-a.example","proto":"udp"}}
```

Le SDK les appelle `p.profiles.list`, `p.profiles.get` et `p.profiles.of`, le kit `Cx::profiles`, `Cx::profile` et `Cx::profile_of` ; tous refusent avant l’appel une Escouade qui ne sert pas les profils. Le cas complet : [des identifiants par projet](plugins/project-credentials).

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
