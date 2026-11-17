# Use

## Start an agent

Pick “Prime Agent” in the “Harness” menu of “New agent”, in **Bypass** mode. Each agent has its own `prime-agent --mode acp --no-session`, started in the agent’s folder. Its model is the one Prime Agent takes by default; its session offers the others by its configuration option `model`.

A message sent during a turn waits for its end. “Stop” stops the turn (`session/cancel`); the next message goes as usual.

## Its tool: a Python cell

Prime Agent has one tool, an IPython kernel that stays alive for the session. Its files, its commands (`await bash(...)`), its MCP servers (`await mcp.call_tool(...)`) all go through code. Each cell shows in the conversation as a **Python** tool: its code on its row, its output once opened. A file write therefore has no diff of its own in the conversation: look at the agent’s diff.

Prime Agent has no tool to ask a question over ACP: its first message tells it to end its turn on its question, which comes as a card, and your answer is sent to it.

## The modes

One: **Bypass**. Prime Agent never asks anything and has no Plan, Ask, Auto edits or read-only mode. See the Permissions tab.

## Escouade’s MCP server

When “Agents can drive Escouade” is on, Escouade gives its MCP server to each agent’s session (`session/new`, over HTTP, the token in a header). Prime Agent takes it and its model calls it from Python; the trial showed it. Escouade writes nothing in `~/.prime/agent/settings.json`: your own MCP servers stay there as they are.

## What it does not do in Escouade yet

- **Resume a session.** Prime Agent 0.10.3 loads no session over ACP (`loadSession: false`). After a restart of Escouade or of the agent, the conversation stays in Escouade, but Prime Agent starts on a new session, without the context from before. Escouade 2.2.3 says so in the conversation and gives that session the agent’s instructions again (the protocol of its ticket, that of questions) with a summary of what it did; 2.2.2 does not.
- **Its subagents.** `rlm.spawn` needs a saved session; those of its agents in Escouade are not saved (`--no-session`, to leave nothing in its daemon). Its model sees it refused and does otherwise.
- **The reasoning effort**, before Escouade 2.2.3. Prime Agent sets it by its option `thought_level`, for the models that reason; since 2.2.3, the agent’s effort (Low, Medium, High) sets it, when its model offers it (`acp.efforts`). A model that does not reason has no such option: nothing is set.
- **Tokens and cost.** Prime Agent says none of them over ACP.
- **A discussion of the Docs tab**, and an agent beside a private project: it does not start.
