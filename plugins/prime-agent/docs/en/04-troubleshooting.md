# Troubleshooting

The messages below are the ones Escouade and the ACP plugin show, word for word; `{…}` is what changes from one time to the next.

## The agent does not start

**“Prime Agent only runs in Bypass (it never asks before it acts): turn on “Allow Bypass mode” in Settings › Agents, or choose another harness.”**
Prime Agent has this mode alone. Turn on “Allow Bypass mode” after reading the Permissions tab, or do not use Prime Agent.

**“Prime Agent has no mode that changes nothing (its manifest names none): it doesn’t hold a read-only discussion.”**
A discussion of the Docs tab does not run on Prime Agent. Pick another harness for it.

**“Prime Agent reads and writes files by itself, without passing them through Escouade (its manifest says so): the fence of private projects wouldn’t hold, the agent doesn’t start in this private project.”**
One of your projects is private: each agent then has a fence, and Prime Agent cannot hold it. The message holds for any project as long as one project is private.

**“Prime Agent not found. Install it or give its path in the settings.”**
Escouade did not find `prime-agent` on the `PATH`. Install Prime Agent (Install tab), add `~/.local/bin` to the `PATH`, and start Escouade again.

**“No Prime Agent account: add one in Settings › Accounts and harnesses.”**
Add the machine’s account.

**“Prime Agent didn’t open the agent’s session: {reason}. If it asks for a sign-in, sign it in in a terminal.”**
Prime Agent refused the session, or its daemon did not start. Run `prime-agent` in a terminal: `/login` if it has no provider, and `prime-agent doctor` (below).

## During a turn

**“Prime Agent doesn’t resume its session: the agent starts again on a new session.”**
Prime Agent 0.10.3 resumes no session over ACP (`loadSession: false`), and its sessions go with the connection (`--no-session`). After a restart, the agent starts on a new session, without the context from before; Escouade 2.2.3 says so, and gives that session its instructions again (its ticket’s protocol) and a summary of what it did, with its next message. With Escouade 2.2.2, nothing is said nor given again: tell it what it must know.

**“Prime Agent: {error}”**
Prime Agent ended the turn on an error, in its own words (a model out of reach, a context too long, a key refused). Read them.

## Its daemon

`prime-agent --mode acp` does not work alone: it starts, unless one is there already, Prime Agent’s daemon (`prime-agent --mode daemon`), which holds the sessions, their workers and their Python kernels. That daemon is **one for the whole user**: the same one serves your own sessions in a terminal and every agent of Escouade, whatever their worktree. Its address: on Linux and macOS, `/tmp/prime-agent-rust-<uid>/daemon.sock` (`$TMPDIR` when set); on Windows, the named pipe `\\.\pipe\prime-agent-daemon`.

What is left when an agent stops or when Escouade closes:

- **The agent’s session, its worker and its kernel**: nothing. The plugin starts Prime Agent with `--no-session`: the session belongs to Escouade’s connection, and goes with it (at once when Escouade closes its input, 30 s later if the process is killed).
- **The daemon**: on Linux and macOS, it stays, empty (about 60 MB of memory), until you stop it: it leaves the process group Escouade stops. On Windows, it stays in the Job of the agent that started it and stops with it, taking along the sessions of the other agents that used it at the time (read in Prime Agent’s code, not tried on Windows).

The commands, in a terminal:

- `prime-agent status`: the daemons that answer, their sessions and their age.
- `prime-agent list`: Prime Agent’s agents alive (Escouade’s do not stay there).
- `prime-agent doctor`: what is wrong; `prime-agent doctor --fix` removes dead sockets and stops idle orphaned services.
- `prime-agent shutdown`: stops all of its agents and the daemon, after a confirmation; `--force` without confirmation, and kills what no longer answers. **It stops your own sessions too.**

`prime-agent update` stops the daemon to update: Escouade’s agents under way then lose their session. Update when none runs. A daemon keeps the environment it started with: after a change of `PATH`, of certificates (`NODE_EXTRA_CA_CERTS`) or of provider, `prime-agent shutdown` then start the agent again.

## The ACP plugin

The messages of the plugin that runs Prime Agent (“The plugin of Prime Agent restarted.”, “… stopped after {n} crashes…”) are in the documentation of the ACP plugin.
