# Troubleshooting

The messages below are those Escouade and the ACP plugin show; `{…}` is what changes from one time to the next.

## The agent does not start

**"Unharnessed only runs in Bypass (it never asks before it acts): turn on “Allow Bypass mode” in Settings › Agents, or choose another harness."**
Unharnessed has Bypass only. Turn the setting on, after reading the Permissions tab.

**"Unharnessed reads and writes files by itself, without passing them through Escouade (its manifest says so): the fence of private projects wouldn’t hold, the agent doesn’t start in this private project."**
One of your projects is private: each agent then has a fence, which Unharnessed cannot hold. The message holds for any project while one project is private.

**"Unharnessed has no mode that changes nothing (its manifest names none): it doesn’t hold a read-only discussion."**
A discussion of the Docs tab does not run on Unharnessed. Take another harness for it.

**"Unharnessed not found. Install it or give its path in the settings."**
Escouade did not find `unharnessed` in the `PATH`. The VS Code extension's program is in a folder that changes with each version: give its path in Settings › Accounts and harnesses (see Install).

**"Unharnessed is older than version 0.9.8, the oldest its plugin holds."**
Build a newer version, or update the extension.

**"No Unharnessed account: add one in Settings › Accounts and harnesses."**
Add the machine's account or a new account.

**"Unharnessed didn’t open the agent’s session: {reason}. If it asks for a sign-in, sign it in in a terminal."**
Most often, it has no model: no provider set up, or a local model that does not answer. Check `unharnessed model list` in a terminal, with the same account folder.

## During a turn

**"The Unharnessed session couldn’t be resumed: new session (context cleared)"**
Its session is lost (its file is no longer in the account's `sessions` folder), or Escouade is 2.2.2, which does not resume it by `--resume` (see Use). The agent goes on without the context from before; its instructions are given again with its next message.

**"Unharnessed: {error}"**
Unharnessed ended the turn on an error, in its own words (a provider that refuses, a model not found). Read them.

**The first turn is long**
On its first start, Unharnessed builds its Python kernel with `uv`. Without `uv` in the `PATH`, it cannot.

**An answer that starts with words from before**
They are those of a turn Unharnessed started by itself, when a command ended after its cell (see Use).

## The ACP plugin

The messages of the plugin that runs Unharnessed ("The plugin of Unharnessed restarted.", "… stopped after {n} crashes…") are in the ACP plugin's documentation.
