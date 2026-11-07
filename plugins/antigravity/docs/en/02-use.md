# Use

## Start an agent

Choose “Antigravity CLI” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects, “Default harness”).

The “Model” menu lists your account’s models (`agy models`); for Gemini, the effort (Low, Medium, High) is its “Effort” menu. A change of model, effort or mode takes effect at the next message: Escouade launches `agy` again on the same conversation.

A message sent during a turn waits for its end. “Stop” stops the turn; the next message resumes the conversation. No attached images or PDFs, no “Duplicate the conversation”, no `/` commands in Escouade. Antigravity tells each turn’s tokens, not their cost nor its quota.

## Requests at the end of a turn

When Escouade drives it, `agy` asks nothing during a turn: what its rules don’t allow is refused on the spot. Once the turn is over, the plugin judges each refusal: what Escouade’s rules accept in the mode is accepted without a card, the rest becomes a card (“Allow”, “For this agent”, “Deny”), and its questions too. Then it has the allowed rules written, launches `agy` again, and the agent resumes within the same turn with a message that tells it what is allowed, to do again, and what you answered.

“Allow” holds for that resumption, and `agy` reads its rule as the start of a text: what starts the same also goes through during the resumption, then the rule is taken back. The card says so. “For this agent” is kept by the plugin while it runs: only the same call, to the character, is then accepted without a card. It isn’t offered to a private project’s agent, nor for a write in `.git`, `.husky`, `.githooks`, `.agents` or `.gemini`.

A call whose text has a `*`, a `?`, a parenthesis or a line break can’t become a rule of `agy`: its refusal holds, and the conversation says so.

## The modes

Antigravity has no sandbox. It reads without asking. The rules Escouade writes for the agent decide the rest:

| Mode | In Antigravity CLI |
|---|---|
| Ask | Its default mode. Its writes in the agent’s folder are accepted by the plugin at the end of the turn, except in `.git`, `.husky`, `.githooks`, `.agents` and `.gemini`; its commands, its writes elsewhere and MCP tools are asked. |
| Accept edits | As Ask, and the worktree’s `git add` and `git commit` are accepted too, when the command is nothing else (nothing after a `;`, a `|`, a redirection, a `$(…)`, a line break). |
| Plan | Its `plan` mode: it studies and writes a plan without changing anything, which comes on a card; once approved, Antigravity carries it out. |
| Bypass | Escouade’s rules allow it everything: commands, files, MCP tools, the web. For a private project, it has no wide rule: each refusal is accepted by the plugin, but what names a fenced path. |

In every mode, a read in the folders of the files attached to the agent is accepted by the plugin.

As a write or a `git add` that `agy` refused comes back to the plugin, it takes a resumption of the turn: `agy` is launched again, then does it again.

## The Antigravity CLI view

The “Antigravity CLI view” runs the agent in Antigravity’s interface, in a terminal of the window, on the same conversation. Its requests are answered in the interface.

## A discussion of the Docs tab

A discussion refuses every write and every command, by its rules, whatever the mode; those refusals don’t become cards.

## Private projects

For a private project, the agent’s rules refuse it reading the other projects’ paths, which refuses writing them too, even in Bypass. What it wanted to do there never comes as a card: the plugin refuses it at once, whether it names the path or reaches it by `..`. In Bypass, the agent has no `command(*)`: each command comes back to the plugin.

Its limit: a command that would read those paths without naming them isn’t held.
