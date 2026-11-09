# Use

## Import issues

In the Kanban, "Import" (⤓) opens the "Import tickets" window. The Linear tab shows the issues of the linked team that are neither completed nor canceled (states of the `completed` and `canceled` types), the latest updated first, 50 at a time; "Show more" loads the next ones.

- **Search** ("Search by identifier (ENG-42) or title…"): `ENG-42`, `42` or `#42` finds issue number 42 of the team; any other text keeps the issues whose title contains it, ignoring case.
- **Chips**: "My issues" keeps those assigned to you; then one chip per state of the team that is neither completed nor canceled (several chips: the issues of any of them).
- **Each row** shows the identifier (`ENG-42`), the title, the kind "Issue", the state, the priority (except "No priority"), the assignee and the labels, and the criteria found.

Tick the issues, pick "Max loops", then "Import". Each imported ticket keeps the Linear badge and the issue's identifier on its Kanban card: a click opens it in Linear. Its description is the issue's.

## Acceptance criteria

With "Extract the acceptance criteria" on (Settings › Connected services, "Sync" group), the plugin looks for the criteria in the issue's description: the list items under a heading or a line that names them ("Acceptance criteria", "Critères d'acceptation", "Definition of done", "Criteria", "AC", "DoD"), otherwise its task list items (`- [ ]`). It keeps twenty at most, of 300 characters at most. With no criterion found, the ticket gets Escouade's default criteria.

## The issue follows its ticket

In Settings › Projects › Imported tickets, the "Status mapping" gives, for each Kanban column, the issue's Linear state. The team's states are listed in the order an issue goes through them: triage, backlog, unstarted, started, completed, canceled. Picking the team fills the mapping in: the state that looks like "in progress" for "In progress" and "To test", the one that looks like "to test" for "Done", nothing for "To do".

When an imported ticket reaches a column:

- the issue moves to the chosen state, if "Update the external status" is on;
- Escouade writes a comment on it, in Markdown, if the column is ticked under "Comment" (by default "To test" and "Done").

With "Post a summary on every loop", each new loop of an agent also leaves a comment: the criteria met and the agent's notes.

## Automatic import

Settings › Connected services › "Automatic import" imports on its own the issues that carry the chosen label (`claude-ready` by default). For Linear, it is a label of the issue, whose name must be the same, case aside.

## Limits

- One Linear account per Escouade, and one team per project.
- The menu offers your first 100 teams.
- The import window does not say how many issues match in all: Linear does not give it.
- An answer from Linear cannot exceed 8 MiB nor take longer than 15 seconds.
- Escouade does not create an issue, change it (title, description, assignee) or delete it: it changes its state and comments on it, nothing more.
