# Keeping track of your agents

When several agents work at the same time, you need to know which one is waiting for what. This page presents the overview, which lists all agents of all projects, the shortcuts for moving from one agent to another, and the search in conversations.

On macOS, `Ctrl` becomes `⌘` for shortcuts (except `Ctrl+Tab`).

## The overview

The overview takes the place of the project on screen to list all agents of all projects. Open it with the “Overview” button, at the top of the projects rail, or with `Ctrl+Shift+A` (the shortcut also works from a terminal). The same button or shortcut closes it; so does `Esc`, and you’re back to the view it was hiding: the project or the statistics.

### What it shows

The header gives the count: “2 projects · 7 agents · 3 running”. Below, the agents are arranged in groups:

1. **“Waiting for your answer”**: the agents with the “Question” status, from all projects, the one that has waited longest first, with its project’s name.
2. **One group per project**, in the order of the projects rail, with its color; the agents are in the order of the sidebar.
3. **“Pull requests”** (“Merge requests” when they all come from GitLab), when a PR waits for you, across all projects: “To fix”, “To merge”, and “Reviews requested”. A click opens its agent, otherwise its ticket’s Kanban, otherwise its project’s “Pull requests” view (see [code hosts](code-hosts#news-from-the-code-hosts)).

An agent appears only once, and archived agents aren’t listed. Each line has six columns:

| Column | Content |
|---|---|
| Agent | The status dot, the name, and the ticket’s `▸ KEY` label if there is one |
| Activity | “Question”, “Resumes at 3:00 PM” (usage limit), “Setting up the worktree” followed by the step, or what the agent is doing (“Reading src/a.ts”, “Thinking”…); for an agent at rest, its status: “Ready”, “Done”, or “Error” |
| Model | The agent’s model |
| Cost | The agent’s cost, preceded by “≈” during a turn |
| Context | How much of the model’s window is already filled, or “—” while it isn’t known |
| Since | The time elapsed since its last change (“Last change” with the time in the tooltip) |

If the agent publishes a progress line through Escouade’s MCP server (see [the MCP server](mcp)), it’s written under its line.

A click on a line, or `Enter` when it has focus, opens the agent in its project, on its conversation.

### Answering on the spot

The lines of the “Waiting for your answer” group show the agent’s request, and you can answer some of them without leaving the view.

- A **permission request** that can be read in full here (a command, a file, a search, an address: the `Bash`, `PowerShell`, `SlashCommand`, `Read`, `Edit`, `Write`, `MultiEdit`, `NotebookEdit`, `Grep`, `Glob`, `WebFetch`, and `WebSearch` tools) shows the tool and what it wants to do, with “Allow” and “Deny” buttons. There’s no “Always allow”: for that, or to deny with an explanation, open the conversation.
- For everything else, the line shows the request and a “Reply” button that opens the agent: a **question** (its text is shown), a **plan** (“Claude suggests a plan”), an agent waiting in the Claude Code view (“Waiting for your answer in Claude Code’s interface”), a request that Claude Code would deny by default, or a tool whose summary doesn’t say everything it asks (“Read it in the conversation before answering.”). A request whose argument runs past four lines is also sent back to the conversation (“Too long to read here: read it and answer in the conversation.”).

Only the first request of each agent is shown; when you answer it, the next one takes its place. When a line changes, or slides because a line above it disappeared, its buttons stay inactive for half a second: a click meant for the previous request doesn’t answer the next one.

### With the keyboard

| Keys | Effect |
|---|---|
| `↑` / `↓` | Previous / next line, from one group to the next |
| `Home` / `End` | First / last line |
| `Enter` or `Space` | Opens the line’s agent |
| `Esc` | Closes the overview |

## Moving between agents

| Keys | Effect |
|---|---|
| `Ctrl+J` | Puts the focus on the card of the next agent that is waiting for an answer or that needs a look (a question, an end, or an error you haven’t seen yet), in all projects, starting with the one that has waited longest. The conversation on screen stays: `1`, `2`… answer from the card, `Enter` opens the agent. |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | Next / previous agent of the project on screen, in sidebar order, looping around and skipping archived agents. On macOS, it really is `Ctrl+Tab`, not `⌘Tab`. |
| `Ctrl+1` … `Ctrl+9` | Goes to project n (see [the interface](interface)). |

`Ctrl+J` and `Ctrl+Tab` also work from a terminal. The “N waiting” button in the status bar, at the bottom, does the same as `Ctrl+J`.

To keep an eye on the other projects without opening them, look at their tiles, in the projects rail: a pill gives the number of agents waiting for an answer, a green dot shows that an agent is working, the tooltip counts the changed files (“Δ 7”), and a tile that blinks amber, green, or red flags an agent that needs a look (see [notifications](notifications)). The status bar counts the “active”, “waiting”, and “done” agents.

## Searching conversations

`Ctrl+K` opens “Search conversations”, a palette that searches everything you and your agents have written. (In a terminal on Windows and Linux, `Ctrl+K` stays with the shell.)

- **What is searched**: your messages and Claude’s, the commands and files of tool calls with their output, questions and their answers, permission requests, plans, notices, turn errors, and the names of attached files. Thinking, folded in the conversation, isn’t searched.
- **How**: the search ignores case and accents (“cafe” finds “café”), treats `\` and `/` as equivalent in a path, and reads a typographic apostrophe as a straight one. It starts after two characters, a moment after your last keystroke, and reads the conversation logs on disk, including those of agents in the Claude Code view.
- **Where**: “This project” or “All projects” (the first while a project is on screen), and the “Archived agents” checkbox, checked by default.

Results are grouped by agent, the most recently active first, with the project’s name and the “archived” label where relevant. Each result shows an excerpt with the match highlighted, and its age (“3 h ago”). The most recent messages of each agent come first.

`↑` and `↓` choose a result, `Enter` or a click opens it, `Esc` closes the palette. The agent opens on its conversation (the editor, if it was open, makes room), scrolled to the message, which is highlighted for two seconds; if the message is old, the items before it are drawn first. An archived agent opens read-only. If the agent was deleted in the meantime, a message says “This agent no longer exists.”

The palette stops at 200 results (“The first 200 results: narrow your search.”) or after 5 seconds of reading (“… search stopped after 5 s: narrow your search.”): add words to tighten it.
