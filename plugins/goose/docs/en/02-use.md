# Use

## Start an agent

Pick “Goose” in the “Harness” menu of “New agent”. Each agent has its own `goose acp`. Its model is the one its session offers, by its configuration option `model`.

A message sent during a turn waits for its end. Goose has no tool to ask a question over ACP: its first message tells it to end its turn on its question, which comes as a card, and your answer is sent to it. Its tools `shell`, `write`, `edit`, `read` and `tree` show under the names `Bash`, `Write`, `Edit`, `Read` and `Glob`, each write with its diff.

ACP tells neither quota nor cost: each turn’s tokens are counted when Goose gives them. For now, a described harness has no terminal view and no written texts.

## The modes

Goose has three modes in Escouade; no Plan, no Auto. Goose reads and writes files by itself: what it asks comes as a card, but for what Escouade’s rules accept.

| Mode | In Goose |
|---|---|
| Ask | Its `approve` mode: it asks before each tool, and each request is a card. |
| Accept edits | Its `smart_approve` mode: it asks only for what it deems sensitive; Escouade accepts its writes in the agent’s folder, outside `.git`. |
| Bypass | Its `auto` mode: it asks nothing. Commands, files, the whole disk. No sandbox. |

## A discussion of the Docs tab

A discussion runs in its `chat` mode, which calls no tool, and Escouade also refuses any write or command it would ask.

## Private projects

Goose writes files by itself, without passing them through Escouade (its manifest says so: `"fs": false`). The fence of private projects wouldn’t hold: **as soon as one of your projects is private, no Goose agent starts**, neither in that project nor in the others, since each agent then gets the private project’s paths to refuse.
