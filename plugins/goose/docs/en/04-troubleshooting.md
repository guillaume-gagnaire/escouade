# Troubleshooting

The messages below are those Escouade and the ACP plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“Goose reads and writes files by itself, without passing them through Escouade (its manifest says so): the fence of private projects wouldn’t hold, the agent doesn’t start in this private project.”**
One of your projects is private: each agent then has a fence, and Goose can’t hold it. The message speaks of “this private project”, but it holds for every project as long as one project is private. Make your projects not private, or use another harness.

**“Goose not found. Install it or give its path in the settings.”**
Escouade didn’t find `goose` in the `PATH`. Install Goose (see the Install tab) and start Escouade again if the `PATH` changed.

**“No Goose account: add one in Settings › Accounts and harnesses.”**
Add this machine’s account or a new one.

**“Goose didn’t open the agent’s session: {reason}. If it asks for a sign-in, sign it in in a terminal.”**
Goose refused the session. Most often its provider isn’t configured: `goose configure`.

**“Goose speaks ACP {version}, Escouade ACP {version}: update Goose.”**
`goose update`.

**The agent is marked “harness absent”**
The Goose plugin or the ACP plugin is off or uninstalled. Turn them back on in the Plugins view: the agent comes back with its conversation.

## During a turn

**“The Goose session couldn’t be resumed: new session (context cleared)”**
Goose didn’t find the agent’s session. The agent goes on, without the earlier context.

**“Goose: {error}”**
Goose ended the turn in an error, or refused a setting (the mode, the model), in its own words. Read them.

**A card that says “Outside the agent’s folder” or “MCP server {server}, tool {tool}”**
The request touches a path outside the agent’s folder, or it is a tool of an MCP server.

## The ACP plugin

The messages of the plugin that runs Goose (“The plugin of Goose started again.”, “… stopped after {n} crashes…”) are in the ACP plugin’s documentation.
