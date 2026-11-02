# Manage your plugins from an agent

The store has an MCP server, `https://store.escouade.dev/mcp`: an agent (Claude Code, or any MCP client) submits your versions there, follows their checks, publishes them and manages your plugins, connected to your account by OAuth. Signing and revocation are not there.

## Connect it

In Claude Code:

```sh
claude mcp add --transport http escouade-store https://store.escouade.dev/mcp
```

Then `/mcp` in Claude Code, and “Authenticate” on `escouade-store`. Another MCP client takes the same address, over Streamable HTTP: it finds the authorization server on its own (RFC 9728, RFC 8414), registers (RFC 7591) and connects with PKCE.

## The first connection

1. The browser opens on the store, then on GitHub: the same sign-in as on the site, through the “Escouade Store” App.
2. A consent page, in French or English after your browser, says the client (the name it gives itself, which the store does not check), your account and what it will be able to do. You allow or refuse.
3. The client gets a token of one hour, good for `https://store.escouade.dev/mcp` alone, and a refresh token of 30 days that changes at each use. The store keeps their hash only.

## What it allows

With the `author` scope, every account’s:

| Tool | What it does |
| --- | --- |
| `list_repositories` | your repositories where the App is installed, and your last submissions |
| `list_folders`, `list_tags` | the folders of a repository that hold a manifest, the tags of a folder |
| `submit_version`, `get_version`, `retry_checks` | submit a tag of a folder, follow its eleven checks, run them again when the store or GitHub failed |
| `publish_version` | publish: the categories of a first version, “Ask for the review” (yes by default), new tags published on their own |
| `list_plugins`, `get_plugin` | your plugins, their versions and their state, the versions waiting for a click |
| `yank_version`, `dismiss_version` | yank a published version, set aside a waiting one |
| `set_listed`, `set_auto_publish` | unlist a plugin (or list it again), publish its new tags on their own |
| `change_source` | move a plugin to another repository of yours or another folder, by submitting its next version there |

Each tool goes through the same code as the site: the same checks, the same limits, the same refusals. A refusal comes back as an error of the tool, with its code (`not-yours`, `path-parent`, `categories`, `id-differs`…).

The `review` scope (read the review queue, a version and its report, post a report) is given only to the store’s administrators, asked apart on the consent page. Another account that asks for it gets `author` alone.

## What it never does

- **Sign, revoke, touch a key.** A version’s signature and the revocation list are made on the administrator’s machine, with the key that never leaves it (`npm run plugin-review -- sign`).
- Give a GitHub token, a secret or an internal address; write to a repository; act for another account than the one that consented.
- Pass off what a plugin wrote as an instruction. A README, a manifest, a folder’s name or a report a tool returns comes between `<third_party_data>` and `</third_party_data>`, announced as data to read: the agent does not follow an instruction found there.

## Disconnect a client

In your account on the store, “Connected apps” shows each client you allowed, its scopes, the date of its last use and its last actions. “Disconnect” stops its tokens at once. Each action of a tool is noted in your journal, “by an MCP client”, with the client’s name. Deleting your account erases all its authorizations.

See also [Publishing on the store](/en/docs/plugins/publishing/) for the eleven checks and their refusals.
