# A first plugin

A working plugin in ten minutes: first a Discord webhook, without a line of code, then a program in TypeScript or in Rust that reacts to the same events. Each step is checked before the next.

## What you need

- Escouade, which ships the `escouade-plugin` command beside its program (see [testing a plugin](plugins/testing));
- for Discord, a channel and the URL of one of its webhooks (the channel’s settings › Integrations › Webhooks);
- for a program, Node 22.18 or later in TypeScript, Rust in Rust, to write and test the plugin. Whoever installs it needs neither.

## A Discord webhook, without code

**1. The manifest.** Make a `my-discord` folder and copy into it the manifest of the Discord plugin, one of the [example plugins](plugins/examples#discord). Change its `id` to one of your own (`io.github.<your-login>.discord`): `dev.escouade.*` is kept for Escouade’s own plugins. The heart of the plugin is here:

`plugins/discord/escouade-plugin.json` (excerpt)

```json
  "permissions": {
    "events": ["ticket.column", "agent.status"],
    "network": ["discord.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook Discord", "en": "Discord webhook URL" } }],
    "webhooks": [
      {
        "url": "{{secrets.webhook}}",
        "events": ["ticket.column"],
        "when": { "data.to": "review" },
        "body": {
          "content": "🧪 **{{ticket.key}}** to test: {{ticket.title}} ({{project.name}})",
          "allowed_mentions": { "parse": [] }
        }
      },
```

It asks for two kinds of facts (`permissions.events`), one host (`permissions.network`) and one secret, the webhook’s URL, which it never sees written in a file. The webhook goes out when a ticket moves to the `review` column, “To review”.

**2. Check it.**

```sh
escouade-plugin validate my-discord
```

An error is told field by field, with its path (“`escouade.min` is missing.”).

**3. Load it.** In the Plugins view, “In development”, then “Load a folder…” and your folder. The window says what it will be able to do: get these events, connect to `discord.com`, keep a secret. It is read again whenever one of its files changes.

**4. Its secret.** On its page, the “Settings” tab: paste the webhook’s URL. It goes to the system’s keychain.

**5. Try it.** Move a ticket to “To review”: the message reaches the channel. The “Journal” tab of its page notes each call, by its host and its answer, never the whole URL.

## A program, in TypeScript or in Rust

Both give a WASM module that Escouade runs inside the app, on the same methods. The template of each language says when a ticket is ready to test and refuses force pushes:

| | TypeScript | Rust |
| --- | --- | --- |
| You need | Node 22.18 | Rust, with `rustup target add wasm32-wasip1`, and a C compiler for `cargo test` |
| Create it | `npm create escouade-plugin@latest my-plugin -- --kind program` | `npm create escouade-plugin@latest my-plugin -- --kind rust` |
| The manifest says | `"run": { "js": "dist/plugin.mjs" }` | `"run": { "wasm": "Cargo.toml" }` |
| Its tests | `npm test` | `cargo test` |
| Check its manifest | `npm run validate` | `escouade-plugin validate .` |
| The bench | `npm run bench` | `escouade-plugin test .` |
| The module | `npm run build`, Escouade compiles the rest | `escouade-plugin build .` |
| What you commit | `dist/plugin.mjs` and its sources | `Cargo.lock` and its sources |

Pick TypeScript to go fast. Pick Rust for an API client that already exists, for work on bytes, or for [the kit](plugins/rust), which writes a source of the Kanban or a code host for you.

### In TypeScript

**1. The template.**

```sh
npm create escouade-plugin@latest my-plugin -- --kind program
cd my-plugin
npm install
npm test
```

The template says when a ticket is ready to test and refuses force pushes; its code and its tests are in [the SDK](plugins/sdk#the-client). Its manifest names `"run": { "js": "dist/plugin.mjs" }`: Escouade runs it inside the app, in WASM, with no Node for whoever installs it.

**2. Send a message.** A program can do more than a webhook: pick the channel, format the text, keep a state. The Slack plugin, for example, asks for its secret on each fact and posts with `fetch`, which goes through Escouade, to the hosts of `permissions.network` only:

`plugins/slack/src/plugin.ts` (excerpt)

```ts
  return createPlugin().onEvent(['ticket.column', 'agent.status'], async (fact, p) => {
    const text = textOf(fact);
    if (!text || !fact.project) return;
    const token = await p.secret('token');
    if (!token) return p.log.warn('no Slack token kept: nothing sent (Plugins › Slack › Settings)');
    // A bot token, never anything else, goes to Slack (and never into the journal).
    if (!token.startsWith('xoxb-')) return p.log.warn('the token kept is not a Slack bot token (xoxb-…): nothing sent');
    const channel = channelOf(fact.project.name, channels);
    try {
      const res = await fetch('https://slack.com/api/chat.postMessage', {
        method: 'POST',
        headers: { authorization: `Bearer ${token}`, 'content-type': 'application/json; charset=utf-8' },
        body: JSON.stringify({ channel, text }),
      });
```

Add to your manifest what your code asks for: the fact in `permissions.events`, the host in `permissions.network`, the secret in `permissions.secrets` and `contributes.secrets`.

**3. The bench.**

```sh
npm run bench
```

The [conformance bench](plugins/testing) plays your program as Escouade does: the handshake, the facts, the decisions, the delays, the shutdown. It must pass before any publishing.

**4. Load it**: `npm run build` bundles your code into `dist/plugin.mjs`, then load the folder like the first one, from “In development”. Its journal shows its lines live.

### In Rust

**1. The template.**

```sh
npm create escouade-plugin@latest my-plugin -- --kind rust
cd my-plugin
cargo test
```

`npm` only writes the folder: the plugin has no `package.json` and no `node_modules`. It depends on [the kit](plugins/rust) by version, with its `Cargo.lock`, and its code fits in one file:
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

What each call of the host does is in [the calls](plugins/calls), in TypeScript and in Rust.

**2. The bench.**

```sh
escouade-plugin validate .
escouade-plugin test .
```

`escouade-plugin` compiles the module as the store does, with a compiler it downloads itself the first time (650 MB, a minute), then plays it as Escouade does.

**3. Load it.** `escouade-plugin build .` writes the module into `packs/`: copy it beside the manifest as `plugin.wasm`, which the template keeps out of git, then load the folder from “In development”. Copy it again after each change: the store, for its part, compiles the sources.

```sh
escouade-plugin build .
cp packs/*.wasm plugin.wasm
```

## Next

- [The manifest](plugins/manifest), field by field, and [the events](plugins/events) a plugin can receive.
- [The host’s capabilities](plugins/capabilities) and [the calls](plugins/calls), with an example in TS and in Rust for each.
- [Permissions](plugins/permissions): what each one opens.
- [Publishing on the store](plugins/publishing), when it is ready.
