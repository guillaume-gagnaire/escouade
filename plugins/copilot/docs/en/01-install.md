# Install

Three steps: Copilot CLI on your machine, the plugin in Escouade, then an account.

## 1. Install Copilot CLI

```sh
npm install -g @github/copilot
```

Settings › Accounts and harnesses shows the executable found, its version and its state. The plugin was tried with Copilot CLI 1.0.98. `npm install -g @github/copilot@latest` updates it.

## 2. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “Copilot CLI”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated” and “Reads your agents’ files” (their repository). Confirm.

Like any plugin, it is switched off by its toggle in the Plugins view and uninstalled from its page. Off or uninstalled, no Copilot agent starts; their conversations stay.

## 3. Add an account

There is no account at first: an agent starts only on an account you added.

1. Open Settings › Accounts and harnesses, then click **Add an account…**; once two harnesses are installed, choose the harness “Copilot CLI”.
2. Choose:
   - **This machine’s account**, your usual Copilot (`~/.copilot`). **Add this account** adds it as it is.
   - **A new account**, with a folder of its own. Name it, choose what it takes from this machine’s account (`mcp-config.json`, `copilot-instructions.md`, `agents`, `skills`), linked or copied, then **Create and sign in**.
3. For a new account, a terminal opens with `copilot login` in the account’s folder: it gives you a code to type on github.com/login/device. The window says so, with `@` and your GitHub user name.
4. Click **Done**.

On the account’s card, set the model, effort and mode of new agents.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **The account’s folder**: `~/.copilot` for this machine’s account, launched without touching `COPILOT_HOME`; for a new account, a folder of its own under `~/.escouade/copilot/`, given to Copilot as `COPILOT_HOME`.
- **The sign-in** stays Copilot’s: its token is in the system keychain. `config.json` and `settings.json` are never copied to another account. Escouade takes and copies no token, not even the GitHub CLI’s.
- An account other than this machine’s never gets the environment’s tokens (`GH_TOKEN`, `GITHUB_TOKEN`, `COPILOT_GITHUB_TOKEN`): Copilot would prefer them to its own sign-in.

The month’s AI credits come from each call of its agents, else from a `copilot` of the account started at most every 5 minutes, long enough to read them.
