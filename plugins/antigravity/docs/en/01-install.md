# Install

Three steps: Antigravity CLI on your machine, the plugin in Escouade, then an account.

## 1. Install Antigravity CLI

Install it with its script:

- Windows: `irm https://antigravity.google/cli/install.ps1 | iex`
- macOS and Linux: `curl -fsSL https://antigravity.google/cli/install.sh | bash`

Escouade finds `agy` in the `PATH`, or where its script puts it. Settings › Accounts and harnesses shows the executable found and its version. The plugin was tried with Antigravity CLI 1.4.9. `agy update` updates it; Escouade turns off its automatic updates for its agents.

## 2. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “Antigravity CLI”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated” and “Reads your agents’ account folder”. Confirm.

Like any plugin, it is switched off by its toggle in the Plugins view and uninstalled from its page. Off or uninstalled, no Antigravity agent starts; their conversations stay.

## 3. Add an account

There is no account at first: an agent starts only on an account you added.

1. Open Settings › Accounts and harnesses, then click **Add an account…**; once two harnesses are installed, choose the harness “Antigravity CLI”.
2. Choose:
   - **This machine’s account**: your Google sign-in, offered signed in or not. Not yet signed in, **Add this account** adds it then runs `agy` in a terminal of the window, which opens your browser. “Sign in…”, on its card, does the same again.
   - **A new account**: a Gemini API key (on aistudio.google.com, “Get API key”). Its agents go through that key’s Gemini API, with its models alone. “Change the key…” gives it again.
3. On the account’s card, set the model, effort and mode of new agents.

A second Google account isn’t possible: `agy`’s sign-in is the machine’s, whatever the folder.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **The Google sign-in** stays in the system keychain, where `agy` keeps it. Escouade never sees your credentials, only that the sign-in is there; the plugin never reads the keychain.
- **An API key** is kept in the system keychain, and given to its account’s agents as `GEMINI_API_KEY`: never in a file nor on a command line. A `GEMINI_API_KEY` of your machine isn’t given to an agent that doesn’t go by it.
- **Each agent** has a folder of its own, removed with it, which stands for `agy`’s home: its rules, its conversations and Escouade’s MCP server are kept there, never in your `~/.gemini`, which Gemini CLI shares. Escouade takes Escouade’s token out of that folder when the agent’s process stops.
