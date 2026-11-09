# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“Antigravity CLI not found. Install it or give its path in the settings.”**
Escouade didn’t find `agy`. Install it (see the Install tab), or give its executable’s path in Settings › Accounts and harnesses.

**“No Antigravity CLI account: add one in Settings › Accounts and harnesses.”**
Add this machine’s account or a Gemini API key.

**“Antigravity CLI didn’t start the agent’s conversation: {reason}. If it isn’t signed in, run agy in a terminal to sign in to your Google account.”**
`agy` didn’t open the conversation. Most often the Google sign-in is missing: “Sign in…” on the account’s card, or `agy` in a terminal.

**“The Antigravity CLI plugin (dev.escouade.antigravity-cli) is switched off or not loaded: turn it back on in the Plugins view to start a Antigravity CLI agent.”**
The plugin is off, or couldn’t load. Turn it back on; its page and its journal say why. The agent keeps its conversation.

**“The Antigravity CLI plugin (dev.escouade.antigravity-cli) is no longer installed: the Plugins view offers to install it, to start a Antigravity CLI agent.”**
Install it again from the Plugins view; the agents’ conversations are kept.

**“The plugin of Antigravity CLI doesn’t apply the fence of private projects: the agent doesn’t start.”**, **“… doesn’t hold its harness read-only: the discussion doesn’t start.”**, **“… puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start.”**, **“… wants to write {file}, outside the agent’s folder: the agent doesn’t start.”**
Escouade refused the launch the plugin described. The official plugin doesn’t do that: update it, and report it if the message stays.

## During a turn

**“Antigravity’s usage limit reached: {agy’s words}”**
The quota is spent: the turn ends “limited”. The agent tries again by itself at the reset Antigravity announces, else an hour later (a minute if the model’s capacity is short), or moves to another account by “Switch accounts at” and “Continue on the next harness…”. Its `/usage`, in its terminal view, says where the quota stands.

**“The Antigravity session couldn’t be resumed: new session (context cleared)”**
The conversation to resume no longer exists. The agent goes on, without the earlier context.

**Cards all come at the end of the turn**
That is normal: `agy` asks nothing during a turn. Answer them, and the agent resumes within the same turn with what you allowed.

**An answered card does nothing, and the turn ends**
The plugin started again while the card waited: it can no longer resume, and its answer ends the turn. Send a message so the agent does again what it wanted.

**A card that says “Outside the agent’s folder” or “MCP server {server}, tool {tool}”**
The request touches a path outside the agent’s folder, or it is a tool of an MCP server.

## The plugin

**“The plugin of Antigravity CLI started again.”**
Escouade started it again. `agy` went on; what was being translated may be missing until the turn’s end.

**“The plugin of Antigravity CLI stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts it again. Start it, then send a message. Its journal says what happened.

**“The plugin of Antigravity CLI is stopped ({reason}): the agent stops, its session kept.”**, **“The plugin of Antigravity CLI is not answering: {reason}”**, **“The plugin of Antigravity CLI didn’t translate what its harness said ({reason}).”**
Turn the plugin back on and send a message. If it comes back, report it with the version of Antigravity CLI.
