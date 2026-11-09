# The shared memory

A project’s agents keep a work notebook together: the ways of doing and the pitfalls they learned in this project, found again by each of them at the start, whatever its harness.

## What it holds

The memory is about the work, not the product: “run the e2e tests after a debug build”, “this test is flaky under load, rerun it alone”, “a worktree’s ports are never written in a versioned file”. What the product does stays in its specification.

Each memory has a name, a one-line description, a kind and a text:

| Kind | For |
|---|---|
| Way of working (`feedback`) | a way of working that works, a pitfall to avoid; the default kind |
| Project (`project`) | a constraint of the work under way |
| Reference (`reference`) | where to find something: a page, a dashboard, a tool |
| User (`user`) | what you prefer |

## Where it lives

In the `.escouade/memory/` folder of the project’s folder (in its first repository when it has several), not in each agent’s worktree: every agent of the project reads and writes the same one, whatever its branch.

- One Markdown file per memory, `<name>.md`, with a header (`name`, `description`, `type`), like Claude Code’s memory.
- A short index, `MEMORY.md`, that Escouade writes again at each change.
- **Escouade never commits it**, not even when approving a ticket: you commit it when you want to share it with the team, who get it at their pull. It blocks neither a merge nor a ticket’s approval.
- Escouade never reads or writes it through a symbolic link, nor outside the project’s folder: a cloned repository holding one is refused.

## What the agents do

Every new Escouade agent is asked to look in it at the start, and to note in it what it learns about how to work in the project. It goes through four tools of the [MCP server](mcp), the same for [every harness](harness): an agent that moves from Claude Code to Codex finds the memory as it was.

| Tool | What it does |
|---|---|
| `search_memory` | Searches by the words (name, description, text; case and accents ignored), and by the meaning too once Escouade’s vectors model is downloaded. Returns the memories found, best first, with the start of their text. |
| `read_memory` | Reads a whole memory. |
| `write_memory` | Notes a memory, or replaces the one of the same name. |
| `forget_memory` | Forgets a memory that turned out wrong. |

An Escouade agent may leave out the project for its own. The memory of a [private project](mcp#private-projects) is seen by its own agents alone. Claude outside Escouade reaches it too, by naming the project.

## Bringing in Claude Code’s memory

Claude Code keeps its own memory of each project. The first time you open a project Claude Code remembered something about, Escouade offers to bring it into the shared memory:

- a small window lists each memory, with its kind and its text, none ticked at first;
- tick the ones every agent should know, then “Take”; “Take nothing” copies nothing;
- a memory of the same name already in the project is marked “Replaces the project’s”;
- the question doesn’t come back, whether you took something or not. Closed without a choice, it comes back at the next start.

Escouade reads Claude Code’s project memory in the folder of each of your Claude Code accounts, and nothing else: neither your credentials nor your conversations. It never writes there.

## In the settings

Settings › Projects › Team, “Shared memory” group: the list of the project’s memories, their kind and their description. A click on a memory shows its text. “Forget”, then “Confirm forgetting”, deletes its file. The agents write the memory themselves: the settings are there to read it and clean it up. See [the projects’ settings](settings-projects).
