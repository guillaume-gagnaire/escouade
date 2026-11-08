# Install

The ACP plugin does nothing alone: it serves the harnesses other plugins describe.

## 1. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “ACP”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated”, “Writes in your agents’ folders” and “Reads your agents’ files” (their repository). Confirm.

## 2. Install a described harness

1. Install the harness’s program on your machine, as its page says (for Goose: its script, then `goose configure`).
2. Install the plugin that describes it, from the **Store**, or from a folder for a plugin in development (**From a folder…**). Before installing, the window shows the program its agents will run, with its arguments: “Runs this program of your computer for its agents, with your rights”.
3. Pick its harness in the “Harness” menu beside “New agent”.

## 3. Add an account

The ACP plugin has no account. Each described harness has its own: Settings › Accounts and harnesses, **Add an account…**, its harness.

- **This machine’s account**: the harness’s usual configuration, as it is.
- **A new account**: a configuration folder of its own, under `~/.escouade/<harness>/`, given to the harness by the variable its manifest names (`accountVar`, `GOOSE_PATH_ROOT` for Goose).

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **The harness’s configuration and keys** stay its own, in its folder. Neither Escouade nor the plugin reads them. ACP tells neither an agent’s quota, nor its cost, nor its accounts.
