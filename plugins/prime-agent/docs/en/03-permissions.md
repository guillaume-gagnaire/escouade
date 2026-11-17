# Permissions

This page says what the plugin makes happen on your machine, and what it cannot do. It says this first: **Prime Agent never asks anything, and there is no sandbox.**

## What a Prime Agent agent does

The model writes Python code, and Prime Agent runs it in its kernel, with your user’s rights, asking neither you nor Escouade. That code can read, write and delete any file you can touch, on the whole disk and not only in the agent’s folder; run any command; go on the network; read your environment variables and your keys. Prime Agent’s README says so itself: its kernel and worker processes “are not a security sandbox”.

Escouade sees nothing go by before: no permission card shows, no rule of Escouade applies. It sees the code of a cell only as it runs.

## How to guard against it

- Let a Prime Agent agent work only on a repository and instructions you trust.
- Let it work in a **disposable worktree** (a ticket’s worktree), which you review before merging.
- For code or instructions you are not sure of, run Escouade and Prime Agent in a **container** or a **virtual machine**, without your keys or your personal folders.
- Keep “Allow Bypass mode” off if you want no agent of this kind: a Prime Agent agent in Bypass does not start without it.

## Bypass mode in Escouade

Prime Agent has one mode, Bypass, which says what it is. Without “Allow Bypass mode” (Settings › Agents, the policy `permissions.allowBypass` or `.escouade.yml`), an agent does not start and says so: “Prime Agent only runs in Bypass (it never asks before it acts): turn on “Allow Bypass mode” in Settings › Agents, or choose another harness.”

**Why Escouade 2.2.2 at least**: up to 2.2.1, without Bypass allowed, a new agent got the “Ask” mode, which Prime Agent does not have, and nothing refused it: the agent started, and Prime Agent ran as always, asking nothing, while the agent was shown in “Ask” (found for Unharnessed by ESC-484, and the same here). 2.2.2 fixes it (ESC-489); the plugin does not install on an older version.

## What it starts

This plugin has no program: nothing of it runs, its manifest is read. It declares no permission: no network, no secret, no file.

Its agents start `prime-agent --mode acp --no-session`, the program of your machine, with your rights. That is what the installation window shows. That program starts Prime Agent’s **daemon** when needed (`prime-agent --mode daemon`), one for all of the user’s sessions, which outlives it: see the Troubleshooting tab.

## What is read, and the secrets

The plugin itself reads nothing and receives nothing. The ACP plugin that serves Prime Agent receives each agent’s MCP token and gives it to its session in a header, on its input, never in its arguments. The code of its model, though, can read what it likes, that token included: it runs with your rights.

## The fence of a private project

It cannot hold: Prime Agent reads and writes files by itself (`"fs": false`; tried, it never asked the client for files). Escouade therefore refuses to start a Prime Agent agent as soon as it has a fence, that is as soon as one of your projects is private, whatever the agent’s project.

## What it cannot do

- run anything itself: it has no program;
- start anything other than what its manifest describes, which the installation window showed you;
- hold a read-only discussion, or run beside a private project.
