# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{harness}` is the name the manifest gives the harness, and `{…}` what changes from one time to the next.

## The agent doesn’t start

**“{harness} reads and writes files by itself, without passing them through Escouade (its manifest says so): the fence of private projects wouldn’t hold, the agent doesn’t start in this private project.”**
The manifest says `"fs": false`, and the agent has a fence: one of your projects is private. The message speaks of “this private project”, but it holds for every project as long as one project is private. For an author: if the agent can pass its files through the client, declare `"fs": true`.

**“{harness} has no mode that changes nothing (its manifest names none): it doesn’t hold a read-only discussion.”**
A discussion of the Docs tab was started on a harness without `readOnlyMode`. Choose another harness for the discussion; for an author, declare that mode.

**“{harness} speaks ACP {version}, Escouade ACP {version}: update {harness}.”**
The agent speaks another version of ACP than the plugin. Update it.

**“{harness} didn’t open the agent’s session: {reason}. If it asks for a sign-in, sign it in in a terminal.”**
The agent refused the session. Run it in a terminal to see what it asks (a sign-in, a provider).

**“The plugin of {harness} wants to set the variable {variable}, which Escouade refuses a plugin because it would change what the harness runs or touch Escouade: the agent doesn’t start.”**
A third-party plugin’s manifest sets a forbidden variable in `acp.env` (`PATH`, `NODE_OPTIONS`, `LD_*`, `DYLD_*`, `ESCOUADE_*`…). Its author must take it out.

**“{harness} not found. Install it or give its path in the settings.”**
Escouade didn’t find the program in the `PATH`, under the names of `locate.names`.

**“No {harness} account: add one in Settings › Accounts and harnesses.”**
Add an account of the harness.

**The agent is marked “harness absent”**
The ACP plugin, or the plugin that describes the harness, is off, uninstalled or changed. Turn them back on in the Plugins view: the agent comes back with its conversation.

## During a turn

**“{harness} read or changed {path} without passing it through Escouade: the fence of private projects doesn’t hold for these, which only its permissions protect.”**
The agent touched a file by itself while a fence holds. The message comes once per agent. It isn’t an error of the plugin: it is that harness’s limit.

**“The {harness} session couldn’t be resumed: new session (context cleared)”**
The agent didn’t find its session. It goes on, without the earlier context.

**“{harness}: {error}”**
The agent ended the turn in an error, or refused a setting (`session/set_mode: …`, the model), in its own words.

**The agent says “Refused by Escouade: …”**
The client refused a file it asked for: “is in a private project, out of this agent's reach” (a fenced project), “is not an absolute path” (ACP wants an absolute path), “is out of this agent's folders” (outside the agent’s folder and its repository), “this agent is read-only” (a discussion). Nothing to do, but for an author whose agent sends relative paths.

**A card that says “Outside the agent’s folder” or “MCP server {server}, tool {tool}”**
The request touches a path outside the agent’s folder, or it is a tool of an MCP server.

## The plugin

**“The plugin of {harness} started again.”**
Escouade started it again. The agent keeps its session, its turn and the requests waiting.

**“The plugin of {harness} stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts it again. Start the ACP plugin again, then send a message. Its journal says what happened.

**“The plugin of {harness} is stopped ({reason}): the agent stops, its session kept.”**, **“The plugin of {harness} is not answering: {reason}”**, **“The plugin of {harness} didn’t translate what its harness said ({reason}).”**
Turn the ACP plugin back on and send a message. If it comes back, report it with the harness and its version.
