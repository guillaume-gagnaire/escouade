# Plugins FAQ

The questions that come back about plugins, for those who install them and for those who write them, with a pointer to the page that details each answer.

## For the user

### Do I need Node to run a plugin?

No: a plugin of the store runs inside the app, in WASM, whether it is written in TypeScript (`run.js`) or in Rust (`run.wasm`). Only a plugin installed from a folder or a git repository whose command names `node` needs it; the store refuses that form. Slack and Team rules, two of the [example plugins](plugins/examples), run as `run.js` since their 2.0.0.

### “Verified by Escouade”, “Unverified”: what is the difference?

A verified version was reviewed by Escouade and signed; the app checks the signature itself, without trusting the store. An unverified version passed the store’s automatic checks, but nobody read its code; a plugin installed from a folder or a git repository always is. See [plugin security](plugins/security).

### Can a plugin see my private projects?

Not from the start: a plugin you install is off in every private project, and nothing of that project is sent to it until you switch it on there. A `run.command` program still has your rights on the disk (see [permissions](plugins/permissions)). The plugins shipped with Escouade are on there, like the rest of the app.

### How do I stop a plugin that does something I do not want?

Its switch, in the Plugins view, stops it for the whole app; its Projects tab, for one project; its Permissions tab takes one permission back. Its journal says what it did. A plugin shipped with Escouade is switched off the same way.

### Can a project install a plugin for me?

No. Its `.escouade.yml` can propose some (`plugins.recommended`): Settings › Projects shows them, and the install always goes through the window that says their permissions (see [`.escouade.yml`](escouade-yml#plugins)).

### Where is its data?

Its copy in `~/.escouade/plugins/<id>/`, the list of plugins in `~/.escouade/plugins.json`, its journal in `~/.escouade/plugin-logs/<id>.log`, its secrets in the system’s keychain (see [local data](data)).

## For the author

### Which language should I write a plugin in?

None, when the manifest is enough: a theme, webhooks, an ACP harness. JS or TypeScript with [the SDK](plugins/sdk), published as `run.js`. Rust with [the kit](plugins/rust), published as `run.wasm`. The store takes no other language: the protocol stays open to any program, but it does not get published.

### How do I test it without starting Escouade?

`escouade-plugin test` plays the conformance bench, `escouade-plugin dev` the fake Escouade, by hand; the SDK gives an in-memory fake for unit tests, and the Rust kit `serve_on` (see [testing a plugin](plugins/testing)).

### Can I use `dev.escouade.*` as an id?

No: it is kept for Escouade’s own plugins. Take a domain name of your own, reversed (`io.github.<your-login>.my-plugin`).

### Can it add a panel or a page to the interface?

Not yet: a plugin adds what its manifest declares (themes, harnesses, settings…), drawn by Escouade.

### Do I need an account to publish?

A GitHub account and a public repository: the store reads the version at its tag (see [publishing on the store](plugins/publishing)). An agent can also do it for you, through [the store’s MCP server](plugins/store-mcp).
