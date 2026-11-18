# Permissions

Unharnessed never asks anything, and its kernel is not a sandbox. This page says it plainly.

## What an Unharnessed agent does

- **It never asks.** No permission card ever comes from an Unharnessed agent: not before a command, not before a write, not before a network access. Escouade sees nothing go by before it is done.
- **Everything runs with your rights.** The model's Python code and the commands it runs (`bash(...)`) run as your account, on the whole disk, with the network. Nothing holds it to the agent's folder.
- **The kernel is not a sandbox.** It is an ordinary Python process. Unharnessed's README says it itself: use it in a container, a virtual machine or a disposable checkout.

That is why its one mode in Escouade is **Bypass**: "Unharnessed never asks: its Python code and its commands run with your rights, on the whole disk and the network, with no sandbox."

## What to do

- Only run an Unharnessed agent in a **disposable worktree**, a **container** or a **virtual machine**, on a repository you can lose.
- Do not run it on a machine that keeps secrets or credentials the model could read: it can read all you can read.
- Prefer a separate Unharnessed account (a folder of its own, see Install) and a provider whose key can be lost.

## What Escouade holds

- **The "Allow Bypass mode" setting** (Settings › Agents), off by default. Off, an Unharnessed agent does not start and says so: “Unharnessed only runs in Bypass (it never asks before it acts): turn on “Allow Bypass mode” in Settings › Agents, or choose another harness.”
- **Discussions of the Docs tab**: refused, Unharnessed has no mode that changes nothing.
- **Private projects**: as soon as one of your projects is private, no Unharnessed agent starts, since the fence would not hold.
- **Escouade's token**: it goes to the session over ACP, in a header, never among the arguments.

## Why Escouade 2.2.2 at least

Up to 2.2.1, without Bypass allowed, a new agent got the “Ask” mode, which Unharnessed does not have, and nothing refused it: the agent started, and ran as Unharnessed always runs, asking nothing (found by ESC-484). 2.2.2 judges the mode the harness really runs in, its one mode, Bypass (ESC-489): its agents are shown in Bypass, and start only with “Allow Bypass mode”. The plugin does not install on an older version.

## What the plugin itself runs

This plugin has no program: nothing of its own runs, its manifest is read. It declares no permission: no network, no secret, no file. Its agents run `unharnessed --mode acp`, your machine's program, with your rights; that is what the installation window shows. It offers no install command.
