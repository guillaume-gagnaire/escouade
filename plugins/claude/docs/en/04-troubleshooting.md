# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“Claude Code not found. Install it or give its path in the settings.”**
Escouade didn’t find `claude`. Install Claude Code (see the Install tab), or give its executable’s path in Settings › Accounts and harnesses, or in the account’s “Executable” field.

**“No Claude Code account: add one in Settings › Accounts and harnesses.”**
This harness has no account. Add this machine’s account or a new one.

**“The Claude Code plugin (dev.escouade.claude-code) is switched off or not loaded: turn it back on in the Plugins view to start a Claude Code agent.”**
The plugin is off, or couldn’t load. Turn it back on in the Plugins view; its page and its journal say why it didn’t load. The agent keeps its session.

**“The Claude Code plugin (dev.escouade.claude-code) is no longer installed: the Plugins view offers to install it, to start a Claude Code agent.”**
You uninstalled it. Install it again from the Plugins view; the agents’ conversations are kept.

**“The plugin of Claude Code doesn’t apply the fence of private projects: the agent doesn’t start.”**
A private project exists, and the launch the plugin described doesn’t name the settings file that holds the fence. Update the plugin; if the message stays, report it.

**“The plugin of Claude Code doesn’t hold its harness read-only: the discussion doesn’t start.”**
A discussion of the Docs tab wasn’t described read-only. Same thing: update the plugin.

**“The plugin of Claude Code puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start.”**
Escouade refused a launch that would have shown the token. The official plugin doesn’t do that: check it is the one installed.

## During a turn

**“The plugin of Claude Code started again.”**
The plugin stopped and Escouade started it again. `claude` went on; what was being translated may be missing from the conversation until the turn’s end.

**“The plugin of Claude Code stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts the plugin again. Start it again, then send a message: the agent resumes its session. The plugin’s journal, on its page, says what happened.

**“The plugin of Claude Code is stopped ({reason}): the agent stops, its session kept.”**
The plugin was switched off or stopped while the agent ran. Turn it back on and send a message.

**“The plugin of Claude Code is not answering: {reason}”** and **“The plugin of Claude Code didn’t translate what its harness said ({reason}).”**
The plugin didn’t answer in time, or refused a line of `claude`. If it comes back, look at its journal and report it with the version of Claude Code.

**“Claude Code reported an error”**
The turn ended in an error Claude Code didn’t name. Try again; if it comes back, open the Claude Code view to see what it says.

**“New Claude conversation (context cleared)”**
Claude Code opened a new conversation instead of resuming the old one (a session deleted, or another account’s). The agent goes on, without the earlier context.

**“Context compacted”**
Claude Code summed up the conversation to make room (`/compact`, or by itself). Nothing to do.

## The account

**The card “Sign in again”**
Claude Code couldn’t refresh the account’s sign-in (“OAuth session expired and could not be refreshed”). Click it to sign in again: the agent resumes once signed in. If the sign-in on the disk is already good, it resumes by itself.
