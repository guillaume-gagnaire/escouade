# Use

## Start an agent

Choose “Antigravity CLI” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects › Project, “Default harness”).

The “Model” menu lists your account’s models (`agy models`); for Gemini, the effort (Low, Medium, High) is its “Effort” menu. A change of model, effort or mode takes effect at the next message: Escouade launches `agy` again on the same conversation.

A message sent during a turn waits for its end. “Stop” stops the turn; the next message resumes the conversation. No attached images or PDFs, no “Duplicate the conversation”, no `/` commands in Escouade. Antigravity tells each turn’s tokens, not their cost nor its quota.

## Requests at the end of a turn

When Escouade drives it, `agy` asks nothing during a turn: what its rules don’t allow is refused on the spot. Once the turn is over, the plugin judges each refusal: what Escouade’s rules accept in the mode is accepted without a card, the rest becomes a card (“Allow”, “For this agent”, “Deny”), and its questions too. Then it has the allowed rules written, launches `agy` again, and the agent resumes within the same turn with a message that tells it what is allowed, to do again, and what you answered.

“Allow” holds for that resumption, and `agy` may read its rule as the start of a text: what starts the same would also go through during the resumption, then the rule is taken back. The card says so. “For this agent” is kept by the plugin while it runs: only the same call, to the character, is then accepted without a card, but each time it comes back, its “start of a text” rule holds again for the resumption. It isn’t offered to a private project’s agent, nor for a write in `.git`, `.husky`, `.githooks`, `.agents` or `.gemini` or of the agent’s folder itself, nor for a path that is relative or not already absolute and normal.

What the plugin accepts by its rules holds as “Allow” does: a “start of a text” rule for the resumption. The conversation says so each time. That is why no command is accepted by its rules, a write is accepted only for a file strictly in the agent’s folder named by its absolute and normal path, and the resumption also refuses writes in the agent’s folder’s `.git`, `.husky`, `.githooks`, `.agents` and `.gemini`. The real trial on `agy` 1.5.3 found a rule holds the whole command, not its start; the plugin keeps this caution all the same.

A call whose text has a `*`, a `?`, a parenthesis, a bracket, a brace or a line break can’t become a rule of `agy`: its refusal holds, and the conversation says so.

## The modes

Antigravity has no sandbox. It reads without asking. The rules Escouade writes for the agent decide the rest:

| Mode | In Antigravity CLI |
|---|---|
| Ask | Its default mode. Its writes of a file in the agent’s folder, named by its absolute path, are accepted by the plugin at the end of the turn, except in `.git`, `.husky`, `.githooks`, `.agents` and `.gemini` and but the folder itself; its commands, its writes elsewhere and MCP tools are asked. |
| Accept edits | As Ask: `git add` and `git commit` are asked too, until the real trial of `agy` (a rule `command(git add .)` might let `git add . ; node x.js` through). |
| Plan | Its `plan` mode: it studies and writes a plan without changing anything, which comes on a card; once approved, Antigravity carries it out. |
| Bypass | Escouade’s rules allow it everything: commands, files, MCP tools, the web. For a private project, it has no wide rule: each refusal comes back to the plugin, which refuses what names a fenced path or leads to one once resolved, asks each command and accepts the rest. |

In every mode, a read in the folders of the files attached to the agent, by its absolute path, is accepted by the plugin.

As a write that `agy` refused comes back to the plugin, it takes a resumption of the turn: `agy` is launched again, then does it again.

## The Antigravity CLI view

The “Antigravity CLI view” runs the agent in Antigravity’s interface, in a terminal of the window, on the same conversation. It has the same rules: no `allow` rule but in Bypass without a private project nor a discussion, the refusals of the fence; Accept edits is `agy`’s default mode there. What no rule allows, the interface asks, and its requests are answered there.

## A discussion of the Docs tab

A discussion refuses every write and every command, by its rules, whatever the mode; those refusals don’t become cards.

## Private projects

For a private project, the agent’s rules refuse it reading the other projects’ paths, which refuses writing them too, even in Bypass. What it wanted to do there never comes as a card: the plugin refuses it at once, whether it names the path or reaches it by `..`, a word of a command included (`type ..\prive\secret.txt`). In Bypass, the agent has no `command(*)`: each command comes back to the plugin, and goes to a card.

Its limit: a command that would read those paths without naming them, or by a path the shell makes (`$HOME`, `%CD%`), isn’t refused at once; it goes to a card.
