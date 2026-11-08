# Use

## Import tasks

In the Kanban, "Import" (⤓) opens the "Import tickets" window. The ClickUp tab shows the open tasks of the linked list, subtasks included, the latest updated first, 100 at a time. A task whose status is of the done (`done`) or closed (`closed`) type is not in it.

- **Search** ("Search by id (#86abx3, ABC-12) or title…"):
  - a task's id, with or without `#` (`#86abx3`, or `86abx3` when it mixes letters and digits and is at least six characters long), finds that task if it is in the list;
  - a custom id (`ABC-12`), when your space has them, too;
  - any other text keeps the tasks whose name contains it. As ClickUp has no search by title, the plugin reads the list 100 tasks at a time and stops as soon as a page finds some, or after 5 pages; "Show more" goes on.
- **Chips**: "My tasks" keeps those assigned to you; then one chip per status of the list that is neither done nor closed.
- **Each row** shows the task's custom id (else `#` and its id), its name, the kind "Task", its status, its priority, its assignees and its tags, and the criteria found.

Tick the tasks, pick "Max loops", then "Import". Each imported ticket keeps the ClickUp badge and the task's key on its Kanban card: a click opens it in ClickUp. Its description is the task's, in Markdown.

## Acceptance criteria

With "Extract the acceptance criteria" on (Settings › Connected services, "Sync" group), the plugin takes as criteria, in this order:

1. the items of the checklist named after the criteria ("Acceptance criteria", "Critères d'acceptation", "Definition of done", "Criteria", "AC", "DoD"), in their order;
2. otherwise the items of all the task's checklists;
3. otherwise, in its description, the list items under a heading or a line that names the criteria, or failing that its task list items (`- [ ]`).

It keeps twenty at most. With no criterion found, the ticket gets Escouade's default criteria.

## The task follows its ticket

Each ClickUp list has its own statuses: the "Status mapping" of Settings › Projects › Imported tickets is therefore set on those of the linked list. Picking the list fills it in:

- first by names: the status that looks like "in progress" (`In Progress`, `Doing`, `WIP`…) for "In progress" and "To test", the one that looks like "to test" (`Review`, `QA`, `Testing`…) for "Done";
- otherwise by type: the first custom status (between the open one and the closed ones) for "In progress" and "To test", the next one for "Done", failing that the first status of the done type;
- a list with only an open and a closed status moves nothing by default. "To do" stays unchanged.

When an imported ticket reaches a column:

- the task takes the chosen status, if "Update the external status" is on. If the list no longer has that status (renamed, removed), the task stays where it is and the card says so;
- Escouade writes a comment on it if the column is ticked under "Comment" (by default "To test" and "Done"). The comment is posted without notifying all the task's watchers.

With "Post a summary on every loop", each new loop of an agent also leaves a comment: the criteria met and the agent's notes.

## Automatic import

Settings › Connected services › "Automatic import" imports on its own the open tasks that carry the chosen label (`claude-ready` by default). For ClickUp, it is a tag of the task; ClickUp keeps its tags in lower case.

## Limits

- One ClickUp account per Escouade, and one list per project.
- When ClickUp limits the requests (429), the plugin waits for the end of the limit it announces, one minute at most, and asks once more.
- An answer from ClickUp cannot exceed 8 MiB nor take longer than 15 seconds.
- Escouade does not create a task, change it (name, description, assignees) or delete it: it changes its status and comments on it, nothing more.
