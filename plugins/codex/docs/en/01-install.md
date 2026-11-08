# Install

Three steps: Codex on your machine, the plugin in Escouade, then an account.

## 1. Install Codex

```sh
npm install -g @openai/codex
```

Escouade launches the native binary of the npm package (on Windows, the package’s `codex.exe`, not its launcher). Codex’s desktop app often installs an older version, which the `PATH` may find first: Settings › Accounts and harnesses shows the executable found and its version. The plugin was tried with Codex 0.163.0. `npm install -g @openai/codex@latest` updates it.

## 2. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “Codex”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated”, “Reads your agents’ files” (their repository) and “Reads your agents’ account folder”. Confirm.

Like any plugin, it is switched off by its toggle in the Plugins view and uninstalled from its page. Off or uninstalled, no Codex agent starts; their conversations stay.

## 3. Add an account

There is no account at first: an agent starts only on an account you added.

1. Open Settings › Accounts and harnesses, then click **Add an account…**; once two harnesses are installed, choose the harness “Codex”.
2. Choose:
   - **This machine’s account**, your usual Codex (`~/.codex`), offered when Codex is signed in there. **Add this account** adds it as it is.
   - **A new account**, with a folder of its own. Name it, choose what it takes from this machine’s account (`config.toml`, `AGENTS.md`, `skills`, `prompts`, `rules`), linked or copied, then **Create and sign in**.
3. For a new account, a terminal opens with `codex login` in the account’s folder: your browser opens on ChatGPT’s sign-in. The window says so, with the address, as soon as `auth.json` is written.
4. Click **Done**.

On the account’s card, set the model, effort and mode of new agents.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **The account’s folder**: `~/.codex` for this machine’s account, launched without touching `CODEX_HOME`; for a new account, a folder of its own under `~/.escouade/codex/`, given to Codex as `CODEX_HOME`.
- **The sign-in** stays Codex’s: `auth.json` in that folder. It is never copied to another account: its token changes at each renewal, and two folders using it would sign each other out.
- The plugin never reads `auth.json`. Of that folder it reads only `config.toml`, and only for an agent Escouade’s server is refused to.

The account’s quota comes from Codex itself: from its agents while they run, else from a `codex app-server` of the account started every 5 minutes, long enough to read it.
