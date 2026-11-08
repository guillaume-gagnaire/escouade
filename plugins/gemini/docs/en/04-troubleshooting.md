# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“Gemini CLI not found. Install it or give its path in the settings.”**
Escouade didn’t find `gemini`. Install it (`npm install -g @google/gemini-cli`), or give its executable’s path in Settings › Accounts and harnesses.

**“No Gemini CLI account: add one in Settings › Accounts and harnesses.”**
Add a Google account or an API key.

**“Gemini CLI didn’t open the agent’s session: {reason}. If it isn’t signed in, run gemini in a terminal to sign in to your Google account, or give it an API key by GEMINI_API_KEY.”**
Gemini refused the session. For a Google account, “Sign in again…” on its card signs it in again; for a key, “Change the key…”.

**“Gemini CLI speaks ACP {version}, Escouade ACP {version}: update Gemini CLI (npm install -g @google/gemini-cli).”**
The machine’s Gemini speaks another version of ACP than the plugin. Update it.

**“The Gemini CLI plugin (dev.escouade.gemini-cli) is switched off or not loaded: turn it back on in the Plugins view to start a Gemini CLI agent.”**
The plugin is off, or couldn’t load. Turn it back on; its page and its journal say why. The agent keeps its session.

**“The Gemini CLI plugin (dev.escouade.gemini-cli) is no longer installed: the Plugins view offers to install it, to start a Gemini CLI agent.”**
Install it again from the Plugins view; the agents’ conversations are kept.

**“The plugin of Gemini CLI doesn’t apply the fence of private projects: the agent doesn’t start.”**, **“… doesn’t hold its harness read-only: the discussion doesn’t start.”**, **“… puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start.”**
Escouade refused the launch the plugin described. The official plugin doesn’t do that: update it, and report it if the message stays.

## During a turn

**“Gemini’s usage limit reached: {Gemini’s words}”**
The turn ends “limited”. The agent resumes by itself when Gemini says (“Please retry in 30s”), else a minute later, an hour later for its daily quota, or moves to another account by “Switch accounts at” and “Continue on the next harness…”.

**“The Gemini CLI session couldn’t be resumed: new session (context cleared)”**
Gemini didn’t find the agent’s session. The agent goes on, without the earlier context.

**“Gemini CLI: {error}”**
Gemini ended the turn in an error, or refused a setting (the mode, the model: `session/set_mode: …`), in its own words. Read them; if it comes back, report it with the version of Gemini CLI.

**A card that says “Outside the agent’s folder” or “MCP server {server}, tool {tool}”**
The request touches a path outside the agent’s folder, or it is a tool of an MCP server.

**Gemini says “Refused by Escouade: a path of a private project.”** or **“Refused by Escouade: this conversation changes no file and runs no command.”**
The rules Escouade gave it refused the tool: the fence of a private project, or a discussion. Nothing to do.

**The agent doesn’t go to Bypass though Gemini writes “[MODE_UPDATE] yolo”**
On purpose: a text of the model never raises the mode. Change the mode in the Composer.

## The plugin

**“The plugin of Gemini CLI started again.”**
Escouade started it again. The agent keeps its session, its turn and the requests waiting.

**“The plugin of Gemini CLI stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts it again. Start it, then send a message. Its journal says what happened.

**“The plugin of Gemini CLI is stopped ({reason}): the agent stops, its session kept.”**, **“The plugin of Gemini CLI is not answering: {reason}”**, **“The plugin of Gemini CLI didn’t translate what its harness said ({reason}).”**
Turn the plugin back on and send a message. If it comes back, report it with the version of Gemini CLI.
