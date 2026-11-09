# isola’s permissions

What the plugin can do, what it cannot, and what Escouade does for it.

## What it is

A WASM module (`run.wasm`), compiled from its folder, that runs isolated inside Escouade: it has no process of its own, and reaches neither the disk, nor the network, nor your secrets. To act, it asks Escouade to run a program.

## What it runs

Exactly three commands, those of its manifest’s `permissions.commands`, shown in the **Permissions** tab of its page:

| Command | When |
| --- | --- |
| `isola ls --json` | to list the worktree’s services, during a test |
| `isola down` | to stop the worktree’s services |
| `isola destroy` | to tear the worktree’s services down before it is removed |

Escouade holds these rules, not the plugin:

- the command and its arguments must be one of these three lines, word for word: `isola ls`, `isola down now` or `isola logs` are refused;
- `isola` is looked for on the PATH, never a path; no shell; on Windows, no `.cmd` nor `.bat`;
- the program runs in the worktree Escouade lends the plugin for the time of the call, and nowhere else: the plugin gets no path, only a name given for that call;
- its environment is an agent’s, without Escouade’s own variables, plus `NO_COLOR=1`;
- its input is closed at once; it has 60 seconds, and is killed past them; it dies with the plugin;
- each start is written in the plugin’s journal.

**The program itself is not isolated**: `isola` runs with your user’s rights. Escouade bounds what the plugin runs, not what isola does.

## What it does not run

`isola up` is not a command of the plugin: Escouade runs it, in a terminal of the project, once you have read and approved the `.isola.toml`. `up` is what runs the commands this file declares (its services, their `setup`).

`ls`, `down` and `destroy` run no command of the `.isola.toml`. That holds for isola as it is today, and is to be checked again when isola changes.

## What it reads and writes

- It reads no file. Escouade reads the `.isola.toml` to show it to you, whole (256 KiB at most), and compares it with the one you approved when the services start.
- It gets what `isola ls --json` writes, and keeps only the services of the worktree’s branch: their name, status, address and port.
- It writes nothing. What the services write, isola does.

## Facts and decisions

It gets no event from Escouade and decides none of your agents’ permissions.

## Secrets

None: it asks for none and gets none.

## What leaves your machine

Nothing. The plugin has no network. The services isola starts are your programs: what they reach does not depend on the plugin.

## What stays in your hands

- Nothing runs before **Run**, and a changed `.isola.toml` is shown to you again.
- You can turn the plugin off for the whole app or for a project; a private project only has it once you turn it on there.
- Turned off, it runs nothing more, and tickets start with a recipe on the reserved ports.
