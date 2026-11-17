# Use

## Start an agent

Pick "Unharnessed" in the "Harness" menu of "New agent". Each agent has its own `unharnessed --mode acp` and its session, in the agent's folder.

## Its one tool: a Python cell

The model has one tool, `ipython`: a Python REPL that stays alive for the whole session, where it reads, searches and changes files and runs its commands (`bash(...)`). Each cell shows in the conversation as a **Python** tool: its row shows the start of its code, and the cell's output shows when you open it. The whole code of a long cell is not shown today, nor its writes as diffs: Unharnessed does not tell Escouade which files a cell touches.

## The model and the effort

An agent runs on Unharnessed's default model (its `defaultModel` setting, or the last one chosen). Escouade's "Model" menu has nothing to pick: Unharnessed's models are each user's own providers'. To change it, change Unharnessed's default (`unharnessed model list` lists them). Escouade does not set the effort.

## During a turn

- A message sent during a turn waits for its end.
- **Stop** stops the turn: the cell under way is interrupted.
- Unharnessed has no tool to ask a question over ACP: its first message tells it to end its turn on its question, which comes as a card, and your answer is sent to it.
- Unharnessed sometimes starts **a turn of its own**, with no message: when a command started by `bash()` ends after its cell. With Escouade 2.2.3, it is a turn of the conversation: the agent is at work, then back at rest at the end its stream marks, and what it said stays in that turn. Before, its words showed under the last turn and counted in the next one's answer.
- What its context holds and the window of its model are read from its messages (Escouade 2.2.3). Neither the cost nor the tokens in and out of a turn: it does not tell them (its cost is 0, `billed: false`, on a local model), and Escouade estimates none.

## After a restart

A process of Unharnessed holds one session, and takes up another only when it starts (`--resume`), under the id it gave it when saving it. With Escouade 2.2.3, its manifest says so (`acp.resume`): when Escouade starts an agent again (after a restart of Escouade, or of the agent), it launches Unharnessed with `--resume <id>`, then loads the session, its history and its kernel's variables. When the session is lost (its file removed), the agent goes on in a new session, the conversation says so, and its instructions are given again with its next message. With Escouade 2.2.2, the session is not resumed: a new one, said.

## Drive Escouade

When "Agents can drive Escouade" is on (Settings › Agents), each session gets Escouade's MCP server over ACP, over HTTP, the agent's token in a header. In Unharnessed, MCP servers are called from the kernel: the model writes `await mcp.call_tool("escouade", "get_board", {})` (its `mcp` skill). There is nothing to declare with `unharnessed mcp add`: do not add Escouade's server there, its token changes from one agent to the next.

## What it does not do in Escouade

- **A discussion of the Docs tab**: Unharnessed has no mode that changes nothing, a discussion does not start.
- **Private projects**: Unharnessed reads and writes files by itself, the fence would not hold. As soon as one of your projects is private, no Unharnessed agent starts, in any project.
- **Plan, Ask, Auto edits, Auto**: it has Bypass only.
