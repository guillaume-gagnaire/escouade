# Install

Three steps: OpenCode on your machine, the plugin in Escouade, then the account.

## 1. Install OpenCode

```sh
npm install -g opencode-ai
```

That is OpenCode 1, the only one Escouade drives: OpenCode 2 (`@opencode/cli`) is recognized and says “Not supported”. Settings › Accounts and harnesses shows the executable found, its version and its state. The plugin was tried with OpenCode 1.19.3. `npm install -g opencode-ai@latest` updates it.

Then connect a provider: `opencode auth login`, or “Sign in…” in Settings › Accounts and harnesses, which runs it. Ollama’s local models work too, declared in OpenCode’s config.

## 2. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “OpenCode”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated” and “Talks to the harness’s local server, started by Escouade”. Confirm.

Like any plugin, it is switched off by its toggle in the Plugins view and uninstalled from its page. Off or uninstalled, no OpenCode agent starts; their conversations stay.

## 3. Add the account

OpenCode has a single account in Escouade: your usual OpenCode config and the providers you connected to it.

1. Open Settings › Accounts and harnesses, then click **Add an account…**; once two harnesses are installed, choose the harness “OpenCode”.
2. **This machine’s account** is offered as soon as OpenCode is installed. **Add this account** adds it.
3. On its card, set the model, effort and mode of new agents, and if you like a **daily budget ($)**.

## What Escouade keeps of the account

- **The list of accounts** (name, order, settings, budget) is in Escouade’s settings, `~/.escouade/settings.json`.
- **The providers and their keys** stay OpenCode’s, in its config and its folder. Neither Escouade nor the plugin reads them.

OpenCode has no quota: a turn’s cost is what your providers give, 0 with a local model. The status bar shows the day’s cost; once the day’s budget is reached, the autopilot pauses until midnight.
