# A first plugin

A working plugin in ten minutes: first a Discord webhook, without a line of code, then a TypeScript program that reacts to the same events. Each step is checked before the next.

## What you need

- Escouade, which ships the `escouade-plugin` command beside its program (see [testing a plugin](plugins/testing));
- for Discord, a channel and the URL of one of its webhooks (the channel’s settings › Integrations › Webhooks);
- for the TypeScript part, Node 22.18 or later, to write and test the plugin. Whoever installs it does not need it.

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

## The same, in TypeScript

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

## Next

- [The manifest](plugins/manifest), field by field, and [the events](plugins/events) a plugin can receive.
- [Permissions](plugins/permissions): what each one opens.
- [Publishing on the store](plugins/publishing), when it is ready.
