# Install

Three steps: Claude Code on your machine, the plugin in Escouade, then an account.

## 1. Install Claude Code

Install it with its script, as you would to use it in a terminal:

- Windows: `irm https://claude.ai/install.ps1 | iex`
- macOS and Linux: `curl -fsSL https://claude.ai/install.sh | bash`

Escouade finds `claude` in the `PATH` or where its script puts it, in WSL too. Settings › Accounts and harnesses shows the executable found, its version and its state. The plugin was tried from Claude Code 2.1.283 to 2.1.301. `claude update` updates it.

## 2. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “Claude Code”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated”. Confirm.

Like any plugin, it is switched off by its toggle in the Plugins view and uninstalled from its page. Off or uninstalled, no Claude Code agent starts; their conversations stay.

## 3. Add an account

There is no account at first: an agent starts only on an account you added.

1. Open Settings › Accounts and harnesses, then click **Add an account…** (or the button in the Claude Code group).
2. Choose:
   - **This machine’s account**, offered when Claude Code is already signed in in its usual folder. **Add this account** adds it as it is.
   - **A new account**, with a folder of its own. Name it, choose what it takes from this machine’s account (`settings.json`, `CLAUDE.md`, `skills`, `agents`, `commands`, `plugins`, `hooks`, `output-styles`), linked or copied, then **Create and sign in**.
3. For a new account, a terminal opens with the account’s `claude`: sign in, with `/login` if Claude Code doesn’t offer it. The window says so, with the address, as soon as the sign-in is done.
4. Click **Done**.

On the account’s card, set the model, effort and mode of new agents.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **The account’s folder**: for this machine’s account, Claude Code’s own (`~/.claude`, or the one `CLAUDE_CONFIG_DIR` names); for a new account, a folder of its own under `~/.escouade/claude/`, given to `claude` as `CLAUDE_CONFIG_DIR`.
- **The sign-in** stays Claude Code’s: `.credentials.json` in the account’s folder, or the keychain on macOS. Escouade reads it to know whether the account is signed in, and for its quota. The plugin never sees it.
- The sign-in and `.claude.json` are never shared between accounts.

Removing an account takes it off the list: its folder stays on the disk, with its sign-in and its conversations.
