# Use

## Start an agent

Choose “Antigravity CLI” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects, “Default harness”).

The “Model” menu lists your account’s models (`agy models`); for Gemini, the effort (Low, Medium, High) is its “Effort” menu. A change of model, effort or mode takes effect at the next message: Escouade launches `agy` again on the same conversation.

A message sent during a turn waits for its end. “Stop” stops the turn; the next message resumes the conversation. No attached images or PDFs, no “Duplicate the conversation”, no `/` commands in Escouade. Antigravity tells each turn’s tokens, not their cost nor its quota.

## Requests at the end of a turn

When Escouade drives it, `agy` asks nothing during a turn: what its rules don’t allow is refused on the spot. Once the turn is over, Escouade shows you a card for each refusal (“Allow”, “For this agent”, “Deny”) and its questions. Then it writes the rules you allowed, and the agent resumes within the same turn with a message that tells it what is allowed, to do again, and what you answered.

“Allow” holds for that resumption; “For this agent” keeps the rule in the agent’s folder, for it alone and its resumptions.

## The modes

Antigravity has no sandbox. It reads without asking. The rules Escouade writes for the agent decide the rest:

| Mode | In Antigravity CLI |
|---|---|
| Ask | Its default mode. Its writes in the agent’s folder go through; its commands, its writes elsewhere and MCP tools are asked at the end of the turn. |
| Accept edits | Its `accept-edits` mode; `git add` and `git commit` go through too. |
| Plan | Its `plan` mode: it studies and writes a plan without changing anything, which comes on a card; once approved, Antigravity carries it out. |
| Bypass | Escouade’s rules allow it everything: commands, files, MCP tools, the web. |

In every mode, the folders of the files attached to the agent are allowed for reading.

## The Antigravity CLI view

The “Antigravity CLI view” runs the agent in Antigravity’s interface, in a terminal of the window, on the same conversation. Its requests are answered in the interface.

## A discussion of the Docs tab

A discussion refuses every write and every command, by its rules, whatever the mode; those refusals don’t become cards.

## Private projects

For a private project, the agent’s rules refuse it reading the other projects’ paths, which refuses writing them too, even in Bypass. What it wanted to do there never comes as a card: the plugin refuses it at once.

Its limit: a command that would read those paths isn’t held.
