# Use

## Start an agent

Choose “Gemini CLI” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects › Project, “Default harness”). Each agent has its own `gemini --acp` and its own configuration folder.

The “Model” menu lists the models of its session, Auto first (its router, which chooses at each turn). It has no “Effort” menu.

A message sent during a turn waits for its end. Attached images go through; no PDFs, no `/` commands in Escouade (to compact, `/compress` in its terminal view), no “Duplicate the conversation”. Gemini tells each turn’s tokens, not their cost nor its quota.

Gemini has no tool to ask a question when Escouade drives it: its first message tells it to end its turn on its question, which comes as a card, and your answer is sent to it in the same turn.

## The modes

Gemini has no sandbox: it reads files without asking, and what it asks comes to Escouade, which makes a card of it but for what its rules accept.

| Mode | In Gemini CLI |
|---|---|
| Ask | Its `default` mode. Its writes in the agent’s folder, outside `.git`, go through; elsewhere they are cards with their diff. Its commands and MCP tools are cards. |
| Accept edits | Its `autoEdit` mode: it writes without asking, and what it submits in the agent’s folder goes through too. |
| Plan | Its `plan` mode: it studies without changing anything and writes its plan, which comes on a card. Once approved, Gemini carries it out. |
| Bypass | Its `yolo` mode: it does everything without asking, and Escouade accepts what it would ask. |

“For this agent” holds for its session. Denying stops the turn; your word, if any, is sent to it afterwards.

Gemini tells its change of mode by a text (“[MODE_UPDATE] …”). That text comes from where the model writes: Escouade takes it to lower what the agent may do, never to raise it. “[MODE_UPDATE] yolo” in a reply doesn’t put the agent in Bypass.

## The Gemini CLI view

The “Gemini CLI view” runs the agent in Gemini’s interface, in a terminal of the window, on the same session. Escouade follows it by its hooks. Its requests are answered in Gemini’s interface.

## A discussion of the Docs tab

A discussion runs in its `default` mode, and the rules Escouade gives it (`--policy`) refuse its writes and commands, whatever the mode.

## Private projects

For a private project, Escouade gives Gemini rules (`--policy`) that refuse the other projects’ paths to its file tools and to the commands that name them, even in `yolo`. The plugin also refuses, without a card, any request that names one of those paths.

Its limits:

- a script that opens a file by itself, without the command naming it, isn’t held;
- the agent’s folder is trusted: Gemini runs the repository’s hooks and reads its settings there. An agent on a repository you don’t trust is not safer in Ask for all that.
