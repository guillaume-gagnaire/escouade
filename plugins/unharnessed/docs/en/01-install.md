# Install

Five steps: Unharnessed on your machine, a model provider for it, the ACP plugin, this plugin, then an account. First of all, read the **Permissions** tab: Unharnessed never asks anything.

## 1. Get Unharnessed

Unharnessed has no official download outside its VS Code extension, and no official command installs it alone: so this plugin offers none. Two ways to get it:

**The VS Code extension (Apple Silicon macOS only).** As of November 16, 2026, the "Unharnessed Agent" extension (`Unharnessed.unharnessed-agent`, 0.0.4) is published on the VS Code Marketplace for Apple Silicon macOS only. It carries the program in its folder:

```text
~/.vscode/extensions/unharnessed.unharnessed-agent-<version>-darwin-arm64/bin/unharnessed
```

beside its Python kernel (`bin/unharnessed-runtime`) and its skills (`bin/skills`), which it finds next to itself. The folder's name changes with each version: Escouade does not find it alone. Give its path in Settings › Accounts and harnesses (the program's path), or put a link to it in a folder of the `PATH`, and do it again after each update of the extension. The extension's program was not tried outside VS Code.

**Building it from source (Windows, Linux, macOS).** It takes Rust 1.98.1 (the repository pins it), [uv](https://docs.astral.sh/uv/) and Git:

```sh
git clone https://github.com/unharnessed-agent/Unharnessed
cd Unharnessed
cargo build --release -p uh-cli --bin unharnessed
./target/release/unharnessed --unharnessed-bootstrap
```

Then put `target/release` in the `PATH`: Escouade looks for `unharnessed` in the `PATH`. `unharnessed --version` must answer `0.9.8` or later. The plugin was tried with Unharnessed 0.9.8, built this way on Linux (commit `fe2cf28` of November 15, 2026); the build on Windows and macOS was not tried.

Either way, `uv` must be in the `PATH`: Unharnessed builds its Python kernel (a venv, `~/.unharnessed/kernel-venv`) on its first start.

## 2. A model provider

Unharnessed talks to its providers itself: Escouade gives it no key. Set it up in a terminal, as its documentation says: a sign-in to a subscription, a key in a provider's usual variables, or a local model (Ollama, LM Studio, llama.cpp) declared in `~/.unharnessed/models.json`. For instance, for Ollama:

```json
{
  "providers": {
    "ollama": {
      "baseUrl": "http://localhost:11434/v1",
      "apiKey": "local",
      "api": "openai-completions",
      "models": [{ "id": "qwen3-coder:30b", "name": "Qwen3 Coder 30B", "contextWindow": 32768 }]
    }
  }
}
```

`unharnessed model list` lists the models it can take.

## 3. The ACP plugin

Unharnessed is served by the ACP plugin (`dev.escouade.acp`). In the Plugins view, check it is in **Installed** and on; otherwise, install it from **Store**. Off, the Unharnessed agents stay, marked "harness missing", with their conversations.

## 4. Install the plugin

1. In the Plugins view, open **Store** and search for "Unharnessed".
2. On its page, click **Install**. The window shows the program its agents will run, `unharnessed --mode acp`, with your rights. Confirm.
3. Allow Bypass mode: Settings › Agents, "Allow Bypass mode". It is Unharnessed's only mode.
4. Pick "Unharnessed" in the "Harness" menu next to "New agent".

## 5. Connect an account

Open Settings › Accounts and harnesses, **Add an account…**, harness "Unharnessed":

- **The machine's account**: Unharnessed's usual folder, `~/.unharnessed`, as it is.
- **A new account**: a folder of its own, given to Unharnessed by `UNHARNESSED_CODING_AGENT_DIR`: its credentials (`auth.json`), its settings (`settings.json`, `models.json`) and its sessions live there. Set up its provider for that folder. The Python kernel stays shared, in `~/.unharnessed/kernel-venv`.

In the account's card, set the mode of new agents to **Bypass** (see the Permissions tab).

## What Escouade keeps of an account

- **The list of accounts** is in Escouade's settings, `~/.escouade/settings.json`.
- **Unharnessed's credentials, settings and sessions** stay its own, in its folder. Escouade reads none of them.
