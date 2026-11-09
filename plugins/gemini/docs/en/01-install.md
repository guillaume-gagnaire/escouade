# Install

Three steps: Gemini CLI on your machine, the plugin in Escouade, then an account.

## 1. Install Gemini CLI

With npm (Node 20 or later) or Homebrew:

```sh
npm install -g @google/gemini-cli
brew install gemini-cli
```

Escouade finds `gemini` in the `PATH`, or in npm’s global folder. Settings › Accounts and harnesses shows the executable found and its version. The plugin was tried with Gemini CLI 0.66.0, against a fake server: not yet with a real account. `npm install -g @google/gemini-cli` updates it.

## 2. Install the plugin

1. Open the Plugins view: the puzzle-piece icon in the projects bar, or Settings › Plugins.
2. If it is not already under **Installed**, open **Store** and look for “Gemini CLI”.
3. On its page, click **Install**. The install window says what it will be able to do: “Runs inside Escouade, isolated”, “Runs this program of your computer for its agents, with your rights”, with `GEMINI_CLI_TRUST_WORKSPACE=true gemini --acp`, and “Reads your agents’ account folder”. Confirm.

Like any plugin, it is switched off by its toggle in the Plugins view and uninstalled from its page. Off or uninstalled, no Gemini agent starts; their conversations stay.

## 3. Add an account

There is no account at first: an agent starts only on an account you added.

1. Open Settings › Accounts and harnesses, then click **Add an account…**; once two harnesses are installed, choose the harness “Gemini CLI”.
2. Choose **A new account**, then one or the other:
   - **A Google account**: Escouade runs `gemini` in a terminal of the window, with the account’s folder as `GEMINI_CLI_HOME`. It opens your browser on your Google account and keeps its sign-in in that folder. “Sign in again…” does it again.
   - **A Gemini API key** (on aistudio.google.com, “Get API key”). “Change the key…” gives it again.
3. On the account’s card, set the model and mode of new agents.

**This machine’s account** is offered only when the machine has a key (`GEMINI_API_KEY`, `GOOGLE_API_KEY`, Vertex AI): the Google sign-in of your `~/.gemini` is never given to agents. A key or Vertex AI of the machine never goes to an added account.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **A Google account** has its folder under `~/.escouade/gemini/`, where Gemini keeps its sign-in. Escouade never reads it: it only sees that it is there, with the account’s address. Its agents get it by `GOOGLE_APPLICATION_CREDENTIALS`.
- **An API key** is kept in the system keychain, and given to its agents as `GEMINI_API_KEY`: never in a file nor on a command line.
- **Each agent** has its own configuration folder (`GEMINI_CLI_HOME`), removed with it: its settings, its sessions and its rules are kept there, never in your `~/.gemini`, which Antigravity CLI shares. Escouade turns off its updates and its usage statistics there.

The plugin sees neither an account’s key nor its sign-in.
