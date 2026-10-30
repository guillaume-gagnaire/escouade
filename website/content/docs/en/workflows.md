# A project’s workflows

A workflow is the series of steps your agents follow in a project (GSD, Superpowers, a `PROMPT.md` loop…). Escouade reads it on your machine, spending nothing, and says where each agent is. This page says what it can follow of each harness and what the MCP server gives of it.

> [!NOTE]
> The word is “workflow”. A “loop” stays a ticket’s turn (“loop 2/5”), and a step back in a workflow is called a return.

## What Escouade reads

Escouade asks nothing of the agent. It reads each harness’s commands (`.claude/commands`), skills, sub-agents and prompts (`.claude/`, all of `.agents/`, `.github/`, `.opencode/`, `.cursor/`), under the project’s folder and in the harnesses’ own folders, then the state files the steps write (`.planning/STATE.md`, `fix_plan.md`…). A workflow is at least two steps that point at each other, or a prompt played again; a command alone (`/commit`) is not one.

For each workflow it keeps:

- its **doors**: the commands or skills one goes in by;
- its **steps**, each with its command, its sub-agents and the files it reads and writes;
- its **returns**: a way back to a step already passed, with the line that says when and the most times;
- its **state files**, and what they say (“phase 3 of 6”, “12 boxes checked of 20”);
- its **exit condition**.

A project of several repositories has the workflow of each repository. The reading is made again when a file it read changes, and on demand.

## Where it is kept

What Escouade understands of a workflow of the repository’s configuration (its entries, its steps, their sub-agents, its ways back, its state files) and your corrections are kept in the repository, a file per workflow in `.escouade/workflows/`, beside the [shared tickets](shared-board). Escouade writes these files and keeps them up to date, but never commits them: commit them with your work, and the whole team that uses Escouade sees the same workflows and the same corrections. A workflow is the repository’s when its entry is, even when a step calls a skill of your own folders (it is named as it is).

- The files keep nothing of one machine: no date, no absolute path, no state of the work (“phase 3 of 6” is read again on the spot). A reading that changes nothing does not touch them.
- The folder is what counts: a teammate whose machine does not read a workflow still sees and follows it. A reading adds a new workflow and updates one that changed, never deleting a file nor making again one you deleted (see [deleting a workflow](#deleting-a-workflow-or-having-none)).
- The files are read again when they change (a `git pull`, another branch); a file that can’t be read is said with its path and why.
- The workflows of your own folders (`~/.claude`, `~/.codex`, GSD or Superpowers installed there) are read, shown and followed the same way, but their file stays on your machine, in `~/.escouade/projects/<id>/workflows/`, never in the repository. “On this machine”, in Settings › Projects › Workflow, keeps the repository’s there too.
- The agents’ history in the workflows stays on your machine, in Escouade’s data folder.
- The format is described by a JSON schema: `https://escouade.dev/schemas/escouade-workflow.schema.json`.

## Deleting a workflow, or having none

- **Delete this workflow**, under its steps in Settings › Projects › Workflow, deletes its file at once. Erasing its file by hand does the same: at the next reading, Escouade sees it is missing. Either way the workflow leaves Escouade, and reading again no longer makes it again, not even after a restart.
- Your machine remembers it for this project, in `~/.escouade/projects/<id>/`. The “Deleted” list shows them, each with “Restore”, which reads it again and writes its file back.
- If what found it changes (a command or a skill edited), it is proposed again, without being written: “Take back” restores it, “Keep deleted” keeps it out as it is now.
- A file that comes back (a `git pull`, another branch) gives the workflow back to the project.
- “This is not a workflow” remains the other way to set it apart: its file is kept, marked, so the whole team sets it apart too.

“No workflow for this project”, in the “New project” assistant (Kanban step) or at the top of Settings › Projects › Workflow, turns everything off. Escouade no longer reads the project’s workflows, writes no file and follows no agent in them. What is already in `.escouade/workflows/` is left as it is. The Workflow tab says so, with where to undo it: “Read the workflows”, then “Save”. In a shared project, it is the `workflows.enabled: false` key of its [`.escouade.yml`](escouade-yml).

## Where each agent is

Each agent is placed by what it does: the door it typed, the sub-agents it launches, the skills it calls, the state files it writes, the end of its turns. A step begins at the first fact that signals it and ends at the first fact of another; its duration and cost are those of the turns between the two.

Every stay in a step is kept in the agent’s history: the time, the duration, the cost, what was done in it, the outcome, and for a return, the reason the step gives. The history is kept with the agent after a restart. An agent that does something else (its message comes from no door and its tools signal no step) is “outside the workflow” until the next door.

## What each harness lets be followed

| Harness | The step | The sub-agent running | The cost per step |
|---|---|---|---|
| Claude Code (conversation) | yes | yes | yes |
| Claude Code (Claude Code view) | yes, by its hooks | yes, by its hooks | at the end of the turn |
| OpenCode | yes | yes | yes |
| Copilot CLI | yes | yes | the turn’s cost, as the account’s plan gives it |
| Codex | by its prompts and the state files | no | per turn |
| Cursor CLI | by its commands and the state files | yes, as far as ACP tells | no |
| Cursor CLI (Cursor view) | by its transcript, a few seconds behind | no | no |

A partial tracking is said so in the window (“Partial tracking”), with what is missing. Nothing is made up: what a harness does not show is not guessed.

Three things seen for real: a door the agent runs by itself (Claude Code calls `/ecrire` with its `Skill` tool within the turn of `/etude`) counts as a door, so the step follows without you typing the second one; the cost of a local model (OpenCode on Ollama) is $0, which is not a gap in the tracking; and in the Cursor view, a typed command is recognised even though the interface replaces it with its file.


## Through the MCP server

Three tools, all read only (see [the MCP server](mcp) for the others):

- **`get_workflow`** (`project`): the project’s workflows, as the Workflow tab shows them. For each one, its doors, its steps (command, sub-agents, files read and written), its returns (`back`, with their `condition` and `max`) and, on each step, the agents on it now (`agentsHere`: id, name, since when, nth stay). Agents that follow a workflow without being on a step are apart (`agentsElsewhere`).
- **`list_agents`**: each agent gains `workflow`, its name, step, turn (that of a prompt played again, or its ticket’s loop) and since when; `null` when it follows none.
- **`get_agent_summary`**: the same `workflow`, and the history of its steps (`history`, the latest 100 stays, oldest first).

An agent does not put itself in a step through a tool: it enters one by doing the work, and that is what these tools read. Nothing more is exposed than the tab shows: never a file of the project, nor a conversation.

### Private projects

A [private project](settings-projects) says nothing of its workflows to the agents of other projects: `get_workflow` does not find it (in the same words as a project that does not exist), and its agents, their step and their history appear in no list of the others. Its own agents see it alone.

## See also

- [The MCP server](mcp): every tool, and what an Escouade agent sees.
- [The Plan banner](plan): an agent’s task list and sub-agents, which complete its step.
- [The harnesses](harness): what each one can do.
