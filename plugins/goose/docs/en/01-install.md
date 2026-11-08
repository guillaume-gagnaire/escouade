# Install

Four steps: Goose on your machine, the ACP plugin, this plugin, then an account.

## 1. Install Goose

- Windows: `Invoke-WebRequest -Uri https://raw.githubusercontent.com/aaif-goose/goose/main/download_cli.ps1 -OutFile download_cli.ps1; ./download_cli.ps1`
- macOS and Linux: `curl -fsSL https://github.com/aaif-goose/goose/releases/download/stable/download_cli.sh | CONFIGURE=false bash`

Then configure its provider in a terminal: `goose configure`. Escouade finds `goose` in the `PATH`. The plugin was tried with Goose 1.57.0. `goose update` updates it.

## 2. The ACP plugin

Goose is served by the ACP plugin (`dev.escouade.acp`). In the Plugins view, check that it is under **Installed** and on; otherwise install it from **Store**. Off, Goose agents stay, marked “harness absent”, with their conversations, and come back with it.

## 3. Install the plugin

1. In the Plugins view, open **Store** and look for “Goose” (or, to try a folder, **From a folder…**).
2. On its page, click **Install**. Before installing, the window shows the program its agents will run: “Runs this program of your computer for its agents, with your rights”, `goose acp`. Confirm.
3. Pick “Goose” in the “Harness” menu beside “New agent”.

Like any plugin, it is switched off by its toggle and uninstalled from its page. Off or uninstalled, its agents stay, marked “harness absent”, with their conversations.

## 4. Add an account

There is no account at first: an agent starts only on an account you added. Open Settings › Accounts and harnesses, **Add an account…**, harness “Goose”:

- **This machine’s account**: Goose’s usual configuration, as it is.
- **A new account**: a configuration folder of its own, under `~/.escouade/goose/`, given to Goose as `GOOSE_PATH_ROOT`. Configure its provider for that folder.

On the account’s card, set the mode of new agents.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **Goose’s configuration and keys** stay its own, in its folder. Escouade reads none of them.
