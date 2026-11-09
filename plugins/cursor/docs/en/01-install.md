# Install

Three steps: Cursor CLI on your machine, the plugin in Escouade, then an account.

## 1. Install Cursor CLI

Install it with its script:

- Windows: `irm 'https://cursor.com/install?win32=true' | iex`
- macOS and Linux: `curl https://cursor.com/install -fsS | bash`

Then sign it in, in a terminal: `agent login`. Escouade finds it in the `PATH`, and on Windows in `%LOCALAPPDATA%\cursor-agent` even when it isn’t in the `PATH`. Settings › Accounts and harnesses shows the executable found and its version. The plugin was tried with Cursor CLI 2026.10.22. `agent update` updates it.

## 2. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “Cursor CLI”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated” and “Runs this program of your computer for its agents, with your rights”, with `agent acp`. Confirm.

Like any plugin, it is switched off by its toggle in the Plugins view and uninstalled from its page. Off or uninstalled, no Cursor agent starts; their conversations stay.

## 3. Add an account

There is no account at first: an agent starts only on an account you added.

1. Open Settings › Accounts and harnesses, then click **Add an account…**; once two harnesses are installed, choose the harness “Cursor CLI”.
2. Choose:
   - **This machine’s account**: the sign-in of `agent login`. **Add this account** adds it as it is.
   - **A new account**: another Cursor account, by its API key (on cursor.com, Settings › API Keys). Name it and paste the key.
3. On the account’s card, set the model, effort and mode of new agents. “Change the key…” gives an account’s key again.

The sign-in of `agent login` is the machine’s: a configuration folder doesn’t move it. That is why a second account goes by its key.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **The API key** of an added account is kept in the system keychain, and given to its agents as `CURSOR_API_KEY`: never in a file nor on a command line. The plugin never sees it; it only knows the account goes by a key.
- **The machine’s sign-in** stays Cursor’s. Escouade doesn’t read it.
- **Each agent** has its own Cursor configuration folder (`CURSOR_CONFIG_DIR`), removed with it: its rules, its model and its sessions are kept there, never in your Cursor config.

Cursor doesn’t tell its month’s usage: the link to cursor.com, in Settings › Accounts and harnesses, and in the quota panel, opens its dashboard.
