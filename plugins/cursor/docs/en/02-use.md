# Use

## Start an agent

Choose “Cursor CLI” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects › Project, “Default harness”). Each agent has its own `agent acp` and its own configuration folder.

The “Model” menu lists your account’s models, Auto first, with their efforts (Low to Very high). A Free plan runs Auto alone: another model ends the turn in an error that says so. Model and mode change at once.

A message sent during a turn waits for its end: Cursor would stop the turn under way for it. Attached images go through; no PDFs, no `/` commands in Escouade (to compact its context, type `/summarize` in its terminal view), no “Duplicate the conversation”. Cursor tells neither the tokens nor the cost of a turn.

The Claude Code skills, commands, agents and hooks of your account and of the project run under Cursor too. A hook that refuses a write blocks it, without Cursor saying why.

## The modes

Cursor has no sandbox on Windows. **It reads and writes files by itself**, even outside the agent’s folder, without passing them through Escouade, which therefore cannot stop it. What it asks comes to Escouade, which makes a card of it, but for what its rules accept:

| Mode | In Cursor CLI |
|---|---|
| Ask | Its Agent mode. Each request of Cursor (a command, the web, an MCP tool, a write it submits) is a card. |
| Accept edits | Its Agent mode. A write or a deletion it submits goes through when all its paths are in the agent’s folder, outside `.git`; the rest is a card. |
| Plan | Its Plan mode: it studies, then puts its plan on a card. Once approved, it carries it out. |
| Bypass | Its Agent mode, and Escouade accepts all it asks. |

A Cursor card has “Allow”, “For this agent” and “Deny”: “For this agent” keeps the rule (`Shell(git status)`, say) in the agent’s configuration folder, for it alone and its resumptions. Its questions, plan, tasks and subagents go into the conversation and the “Plan” banner.

Cursor has no mode that changes nothing: it holds no discussion of the Docs tab.

## The Cursor CLI view

The “Cursor CLI view” runs the agent in Cursor’s interface, in a terminal of the window. Its following is limited: Escouade reads the interface’s transcript, without knowing when Cursor waits for your permission or your answer; watch the terminal. What you say there goes before the next message, once the agent is back in Escouade’s view.

## Private projects

For a private project, Escouade refuses at once, without a card, any request of Cursor that names a path of another project: its title, its arguments, the paths it touches.

Its limits, which are wide:

- Cursor reads and writes files without asking: the fence doesn’t hold for those reads and writes;
- a command that reaches a path without naming it gets through;
- in Bypass, Escouade accepts what doesn’t name a fenced path.
