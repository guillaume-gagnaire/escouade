# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“Codex not found. Install it or give its path in the settings.”**
Escouade didn’t find `codex`. Install it (`npm install -g @openai/codex`), or give its executable’s path in Settings › Accounts and harnesses.

**“No Codex account: add one in Settings › Accounts and harnesses.”**
Add this machine’s account or a new one.

**“The Codex plugin (dev.escouade.codex) is switched off or not loaded: turn it back on in the Plugins view to start a Codex agent.”**
The plugin is off, or couldn’t load. Turn it back on; its page and its journal say why. The agent keeps its session.

**“The Codex plugin (dev.escouade.codex) is no longer installed: the Plugins view offers to install it, to start a Codex agent.”**
Install it again from the Plugins view; the agents’ conversations are kept.

**`initialize: {reason}`** or **`thread/start: {reason}`** (or `thread/resume`, `thread/fork`)
Codex refused to open the conversation, with its reason. Most often the account isn’t signed in: “Sign in…” on its card, in Settings › Accounts and harnesses. Otherwise, update Codex.

**“The plugin of Codex doesn’t apply the fence of private projects: the agent doesn’t start.”**, **“The plugin of Codex doesn’t hold its harness read-only: the discussion doesn’t start.”**, **“The plugin of Codex puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start.”**
Escouade refused the launch the plugin described. The official plugin doesn’t do that: update it, and report it if the message stays.

## During a turn

**“The Codex conversation couldn’t be resumed: new conversation (context cleared)”**
Codex didn’t find the agent’s conversation (deleted, or another account’s). The agent goes on, without the earlier context.

**“Codex reported an error”**
The turn ended in an error Codex didn’t name. Try again; if it comes back, open the Codex view to see what it says.

**“Codex didn’t compact the context: {reason}”**
`/compact` failed. Try again later; the conversation goes on as it is.

**“Compacting the context…”** then **“Context compacted”**
Codex sums up the conversation to make room. Nothing to do.

**“Codex’s automatic reviewer: {message}”**
In Auto, Codex’s reviewer decided a request for you; the conversation notes it.

**A card that says “git writes in the project’s repository, outside Codex’s sandbox.”**
The command writes in `.git` and isn’t a plain form of `git add` or `git commit` the plugin accepts alone: look at it before you allow it.

**A card that says “In {folder}”**
The command runs outside the agent’s folder.

**A card “Codex asks for network access”, “… writing in {paths}”, “… reading {paths}”**
Codex wants to leave its sandbox. “For this session” lets it until its session ends.

## The plugin

**“The plugin of Codex started again.”**
Escouade started it again. Codex went on, and what it asked before still gets its answer.

**“The plugin of Codex stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts it again. Start it, then send a message. Its journal says what happened.

**“The plugin of Codex is stopped ({reason}): the agent stops, its session kept.”**, **“The plugin of Codex is not answering: {reason}”**, **“The plugin of Codex didn’t translate what its harness said ({reason}).”**
Turn the plugin back on and send a message. If it comes back, report it with the version of Codex.
