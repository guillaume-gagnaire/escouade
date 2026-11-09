# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“Copilot CLI not found. Install it or give its path in the settings.”**
Escouade didn’t find `copilot`. Install it (`npm install -g @github/copilot`), or give its executable’s path in Settings › Accounts and harnesses.

**“No Copilot CLI account: add one in Settings › Accounts and harnesses.”**
Add this machine’s account or a new one.

**“Copilot CLI isn’t signed in to any GitHub account on this account: sign it in in Settings › Accounts and harnesses (“Sign in…”, copilot login).”**
The account has no sign-in of its own. An account other than this machine’s doesn’t count the GitHub CLI’s (`gh`) sign-in: sign it in with “Sign in…”.

**“Copilot CLI speaks protocol {version}, Escouade protocol {version}: update Copilot CLI (npm install -g @github/copilot@latest).”**
The machine’s Copilot is too old, or too new, for the protocol the plugin speaks. Update it; if Copilot is the newer one, update the plugin.

**“Copilot CLI didn’t open the agent’s session: {reason}”**
Copilot refused the session, with its reason. Read it: a model the account doesn’t have, an expired sign-in…

**“The Copilot CLI plugin (dev.escouade.copilot-cli) is switched off or not loaded: turn it back on in the Plugins view to start a Copilot CLI agent.”**
The plugin is off, or couldn’t load. Turn it back on; its page and its journal say why. The agent keeps its session.

**“The Copilot CLI plugin (dev.escouade.copilot-cli) is no longer installed: the Plugins view offers to install it, to start a Copilot CLI agent.”**
Install it again from the Plugins view; the agents’ conversations are kept.

**“The plugin of Copilot CLI doesn’t apply the fence of private projects: the agent doesn’t start.”**, **“… doesn’t hold its harness read-only: the discussion doesn’t start.”**, **“… puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start.”**
Escouade refused the launch the plugin described. The official plugin doesn’t do that: update it, and report it if the message stays.

## During a turn

**“Copilot uses the GitHub CLI’s account (gh), as it isn’t signed in itself: sign in with copilot login to choose its account.”**
This machine’s account has no Copilot sign-in, and Copilot took the GitHub CLI’s. The agent runs, but on that account. Sign Copilot in with `copilot login` to choose.

**“The Copilot session couldn’t be resumed: new session (context cleared)”**
Copilot didn’t find the agent’s session. The agent goes on, without the earlier context.

**“Context compacted”**
Copilot summed up the conversation to make room. Nothing to do.

**“The session’s ceiling of AI credits is reached ({used} of {max}). What about the call waiting?”**
The session has a ceiling of credits and reached it. Choose to add as many, lift the ceiling or stop there.

**A card that says “Outside the agent’s folder”, “Paths: {paths}”, “Addresses: {addresses}” or “Writes to a file through a redirection”**
What the request touches: paths outside the folder, web addresses, a file written by `>`.

**A card that says “git writes in the project’s repository.”**
The command writes in `.git` and isn’t a plain form of `git add` or `git commit` the plugin accepts alone: look at it before you allow it.

**A card that says “MCP server {server}, tool {tool}: only reads” or “…: may change things”**
A tool of an MCP server, and what its server says it does.

**Copilot says “Refused by Escouade: this path is another project’s, and one of the two is private.”**
The plugin refused a request on a fenced path (see the Permissions tab). Nothing to do: that is the fence.

## The plugin

**“The plugin of Copilot CLI started again.”**
Escouade started it again. Copilot went on; what was being translated may be missing until the turn’s end.

**“The plugin of Copilot CLI stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts it again. Start it, then send a message. Its journal says what happened.

**“The plugin of Copilot CLI is stopped ({reason}): the agent stops, its session kept.”**, **“The plugin of Copilot CLI is not answering: {reason}”**, **“The plugin of Copilot CLI didn’t translate what its harness said ({reason}).”**
Turn the plugin back on and send a message. If it comes back, report it with the version of Copilot CLI.
