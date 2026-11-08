# Troubleshooting

The messages below are those Escouade and the plugin show, word for word, in the app’s English; `{…}` is what changes from one time to the next.

## The agent doesn’t start

**“OpenCode not found. Install it or give its path in the settings.”**
Escouade didn’t find `opencode`. Install it (`npm install -g opencode-ai`), or give its executable’s path in Settings › Accounts and harnesses.

**“Not supported”, in Settings › Accounts and harnesses**
That is OpenCode 2 (`@opencode/cli`). Install OpenCode 1: `npm install -g opencode-ai`.

**“No OpenCode account: add one in Settings › Accounts and harnesses.”**
Add this machine’s account.

**“No OpenCode model to use: connect a provider to OpenCode (opencode auth login) or declare one in its config, then pick its model.”**
OpenCode has no connected provider, or the chosen model isn’t one of theirs. `opencode auth login` (or “Sign in…” in Settings › Accounts and harnesses), then pick a model in the menu.

**“OpenCode’s event stream doesn’t answer”**
OpenCode’s server didn’t open its event stream in time. Send a message to try again; if it comes back, stop the OpenCode agents so that the server starts again.

**“The OpenCode plugin (dev.escouade.opencode) is switched off or not loaded: turn it back on in the Plugins view to start a OpenCode agent.”**
The plugin is off, or couldn’t load. Turn it back on; its page and its journal say why. The agent keeps its session.

**“The OpenCode plugin (dev.escouade.opencode) is no longer installed: the Plugins view offers to install it, to start a OpenCode agent.”**
Install it again from the Plugins view; the agents’ conversations are kept.

**“The plugin of OpenCode doesn’t apply the fence of private projects: the agent doesn’t start.”**, **“… doesn’t hold its harness read-only: the discussion doesn’t start.”**, **“… puts Escouade’s token among its harness’s arguments, which anyone can read: the agent doesn’t start.”**
Escouade refused the launch the plugin described. The official plugin doesn’t do that: update it, and report it if the message stays.

## During a turn

**“The OpenCode session couldn’t be resumed: new session (context cleared)”**
OpenCode didn’t find the agent’s session. The agent goes on, without the earlier context.

**“Compacting the context…”** then **“Context compacted”**
OpenCode sums up the conversation to make room (`/compact`, or by itself). Nothing to do.

**A card that says “Outside the agent’s folder: {paths}”**
The request touches paths outside the agent’s folder.

**A card that says ““Always allow” will keep: {patterns}”**
What OpenCode will keep for the rest of the session if you choose “Always allow”: look at the pattern, it may be wider than the request (`git add *`).

**A `git commit` or a change of `.git` asks in Accept edits**
On purpose: only the plain forms of `git add` and `git commit` go through, and a change of `.git`, `.opencode`, `opencode.json`, `.husky` or `.githooks` always gets its card (see the Permissions tab).

## The plugin

**“The plugin of OpenCode started again.”**
Escouade started it again. The server and the turn went on, and the agent’s session is heard again.

**“The plugin of OpenCode stopped after {n} crashes: the agent stops, its session kept. Start the plugin again in Settings › Plugins, then send a message.”**
After three crashes within five minutes, Escouade no longer starts it again. Start it, then send a message. Its journal says what happened.

**“The plugin of OpenCode is stopped ({reason}): the agent stops, its session kept.”**, **“The plugin of OpenCode is not answering: {reason}”**
Turn the plugin back on and send a message. If it comes back, report it with the version of OpenCode.
