# Prime Agent

This plugin makes Prime Agent, Prime Intellect’s open source coding and research agent, a harness of Escouade. It holds no code: its manifest describes Prime Agent over ACP (`prime-agent --mode acp --no-session`), and Escouade’s ACP plugin (`dev.escouade.acp`) starts it, speaks to it and shows its conversation: live answers, and each cell of its one tool, a Python kernel, with its code and its output.

It is for whoever already has Prime Agent, connected to a model provider.

**Prime Agent never asks anything.** The Python code its model writes and the commands it runs go with your rights, on the whole disk and the network, with no sandbox. Its agents have one mode, Bypass: Bypass mode must be allowed in Escouade, and they should work in a disposable worktree, a container or a virtual machine.

## What it needs

- Prime Agent installed on the machine (`prime-agent --version` answers 0.10.3 or later), connected to a provider (`/login` in its interface, or a `models.json` of its own).
- The ACP plugin (`dev.escouade.acp`) installed and on: it is what runs the described harnesses.
- “Allow Bypass mode” on in Settings › Agents.
- A Prime Agent account added in Settings › Accounts and harnesses.

## What it can do

- Nothing of it runs: it is a plugin without a program, which the installation window calls “No program”.
- Its agents start the program `prime-agent --mode acp --no-session` of your machine, with your rights. That program starts Prime Agent’s daemon when needed, which outlives it (see the Troubleshooting tab).
- It asks for no network, no secret, no file, no account of its own. Escouade reads none of Prime Agent’s keys.

Prime Agent writes files by itself, without passing them through Escouade: as soon as one of your projects is private, no Prime Agent agent starts. It has no mode that changes nothing: it holds no discussion of the Docs tab.

## The tabs

- **Install**: Prime Agent, the plugin, Bypass mode, an account.
- **Use**: its Python tool, its model, what it does not do in Escouade yet.
- **Permissions**: what it does without asking, and how to guard against it.
- **Troubleshooting**: the messages, and its daemon.
- **Develop**: the manifest, its tests, what the study found.
