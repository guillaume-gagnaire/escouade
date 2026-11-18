# Install

Five steps: Prime Agent on your machine, the ACP plugin, this plugin, Bypass mode, then an account.

The plugin needs **Escouade 2.2.2 or later**: before it, the app did not refuse a Prime Agent agent when Bypass mode is not allowed (see the Permissions tab). On an older version, it does not install.

## 1. Install Prime Agent

Its official installers, as its README gives them:

- macOS and Linux (GNU, glibc 2.35 or later; not musl): `curl -fsSL https://app.primeintellect.ai/prime-agent/install.sh | sh`
- Windows x64: `irm https://app.primeintellect.ai/prime-agent/install.ps1 | iex`

The macOS and Linux script puts its launcher in `~/.local/bin/prime-agent` and the program in `~/.local/share/prime-agent/`. It also installs `uv` (from astral.sh) and prepares Prime Agent’s Python kernel in `~/.prime/agent/kernel-venv`. It reads the version of the stable channel, downloads the archive and its `SHA256SUMS` from Prime Intellect’s download site, and checks the sum before extracting. The Windows script does the same in `%USERPROFILE%\.local\` and adds its `bin` folder to the user’s `PATH`. Read a script before running it.

Add `~/.local/bin` to the `PATH` if the script tells you to, then connect Prime Agent to a provider: run `prime-agent` in a terminal and type `/login` (a subscription or a key), or describe a provider of your own in `~/.prime/agent/models.json` (a local OpenAI-compatible model, for example). Escouade finds `prime-agent` on the `PATH`. The plugin was tried with Prime Agent 0.10.3. `prime-agent update` updates it.

Prime Agent sends usage statistics to Prime Intellect by default. `prime-agent telemetry` says where it stands, `prime-agent telemetry off` turns them off. Escouade does not touch it.

## 2. The ACP plugin

Prime Agent is served by the ACP plugin (`dev.escouade.acp`). In the Plugins view, check that it is in **Installed** and on; otherwise, install it from **Store**. Off, the Prime Agent agents stay, marked “harness missing”, with their conversations, and come back with it.

## 3. Install the plugin

1. In the Plugins view, open **Store** and look for “Prime Agent” (or, to try a folder, **From a folder…**).
2. On its page, click **Install**. Before installing, the window shows the program its agents will start: “Starts this program of your computer for its agents, with your rights”, `prime-agent --mode acp --no-session`. Confirm.
3. Pick “Prime Agent” in the “Harness” menu beside “New agent”.

Like any plugin, it turns off by its switch and uninstalls from its page. Off or uninstalled, its agents stay, marked “harness missing”, with their conversations.

## 4. Allow Bypass mode

Prime Agent has one mode, Bypass. Turn on “Allow Bypass mode” in Settings › Agents: without it, an agent in Bypass does not start. Read the Permissions tab first.

## 5. Connect an account

No account exists at first: an agent starts only on an account you added. Open Settings › Accounts and harnesses, **Add an account…**, harness “Prime Agent”. In the account’s sheet, set the mode of new agents to **Bypass**.

Prime Agent has one account in Escouade: the machine’s, `~/.prime/agent`. A second account would have no folder of its own. Prime Agent can move its folder (`PRIME_AGENT_CODING_AGENT_DIR`), but its daemon, one for all of the user’s sessions, keeps the folder it started with: two accounts would mix.

## What Escouade keeps of an account

- **The list of accounts** (name, order, settings) is in Escouade’s settings, `~/.escouade/settings.json`.
- **Prime Agent’s configuration, sessions and credentials** stay its own, in `~/.prime/agent`. Escouade reads none of them.
