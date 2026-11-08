# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“Cursor CLI not found. Install it or give its path in the settings.”**
Escouade didn’t find `agent`. Install Cursor CLI (see the Install tab), or give its executable’s path in Settings › Accounts and harnesses.

**“No Cursor CLI account: add one in Settings › Accounts and harnesses.”**
Add this machine’s account or an account by its API key.

**“Cursor CLI didn’t open the agent’s session: {reason}. If it asks for a sign-in, sign it in in a terminal.”**
Cursor refused the session. Most often the machine isn’t signed in: run `agent login` in a terminal.

**“Cursor CLI speaks ACP {version}, Escouade ACP {version}: update Cursor CLI.”**
Cursor speaks another version of ACP than the plugin. `agent update`.

**“The Cursor CLI plugin (dev.escouade.cursor-cli) is switched off or not loaded: turn it back on in the Plugins view to start a Cursor CLI agent.”**
The plugin is off, or couldn’t load. Turn it back on; its page and its journal say why. The agent keeps its session.

**“The Cursor CLI plugin (dev.escouade.cursor-cli) is no longer installed: the Plugins view offers to install it, to start a Cursor CLI agent.”**
Install it again from the Plugins view; the agents’ conversations are kept.

**“The plugin of Cursor CLI doesn’t apply the fence of private projects: the agent doesn’t start.”**, **“… puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start.”**
Escouade refused the launch the plugin described. The official plugin doesn’t do that: update it, and report it if the message stays.

## The account and its limits

**“Cursor CLI isn’t signed in on this machine (“Please sign in to continue”): run agent login in a terminal.”**
This machine’s account is no longer signed in. `agent login`, then send a message.

**“Cursor refused this account’s API key: give it again in Settings › Accounts and harnesses, “Change the key…”.”**
The account’s key is wrong or revoked. Make one on cursor.com, Settings › API Keys, and give it with “Change the key…”.

**“This Cursor account’s usage limit is reached (“Upgrade your plan to continue”): the month’s included usage is spent until its plan resets. Its usage: {address}”**
The month’s usage is spent, on Auto. The card offers, if there is one, another account or another harness to go on with.

**“Cursor refuses the model {model} on this account (“Upgrade your plan to continue”): it isn’t in this Cursor account’s plan, or its included usage of the month is spent. Choose Auto in the Model menu; its usage: {address}”**
The chosen model isn’t in the plan (a Free plan has Auto alone). Choose Auto.

**“This Cursor account’s usage limit is reached: what comes next is on-demand usage, which needs a payment method (“Add a payment method to continue”). Its usage: {address}”**
Add a payment method on cursor.com, or go on with another account.

## During a turn

**“The Cursor CLI session couldn’t be resumed: new session (context cleared)”**
Cursor didn’t find the agent’s session. The agent goes on, without the earlier context.

**“Cursor wrote nothing: the write was refused, by a hook (those of Claude Code run under Cursor too) or by Cursor itself, which doesn’t give the reason.”**
A write tool ended without changing anything. Look at your Claude Code hooks, the account’s and the project’s.

**A card that says “Outside the agent’s folder” or “MCP server {server}, tool {tool}”**
The request touches a path outside the agent’s folder, or it is a tool of an MCP server.

**“Cursor CLI: {error}”**
Cursor ended the turn in an error, or refused a setting (the mode, the model: `session/set_mode: …`), in its own words. Read them; if it comes back, report it with the version of Cursor CLI.

## The plugin

**“The plugin of Cursor CLI started again.”**
Escouade started it again. The agent keeps its session, its turn and the requests waiting.

**“The plugin of Cursor CLI stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts it again. Start it, then send a message. Its journal says what happened.

**“The plugin of Cursor CLI is stopped ({reason}): the agent stops, its session kept.”**, **“The plugin of Cursor CLI is not answering: {reason}”**, **“The plugin of Cursor CLI didn’t translate what its harness said ({reason}).”**
Turn the plugin back on and send a message. If it comes back, report it with the version of Cursor CLI.
