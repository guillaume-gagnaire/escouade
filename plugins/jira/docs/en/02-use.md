# Use

## Import issues

In the Kanban, "Import" (⤓) opens the "Import tickets" window. The Jira tab shows the issues of the linked project that are not in the "Done" category, the latest updated first, 50 at a time; "Show more" loads the next ones, and the line above the list says how many Jira counts in all ("50 shown of 312") when the list is cut.

- **Search** ("Search by key or text…"): a key, `ATL-12`, finds that issue; any other text is searched by Jira in the issues' text (`text ~`), with the special signs of its search syntax taken out.
- **Chips**: "Assigned to me", "Active sprint" (the issues of the open sprints) and "To do" (the "To Do" category). Several chips add up: an issue must meet each of them.
- **Each row** shows the key, the title, the type (Task, Sub-task, Bug, Story… as Jira names them), the priority, the assignee ("Unassigned" otherwise) and the status, and the criteria found.

Tick the issues, pick "Max loops", then "Import". Each imported ticket keeps the Jira badge and its key on its Kanban card: a click opens it in Jira (`<site>/browse/ATL-12`). Its description is the issue's, turned from Atlassian's document format into Markdown. If the issue moves to another project in Jira, Escouade still follows it: it is held by its id, not by its key.

## Acceptance criteria

With "Extract the acceptance criteria" on (Settings › Connected services, "Sync" group), the plugin looks for the criteria in the description, once turned into Markdown: the list items under a heading or a line that names them ("Acceptance criteria", "Critères d'acceptation", "Definition of done", "Criteria", "AC", "DoD"), otherwise its task list items (`- [ ]`). It keeps twenty at most, of 300 characters at most. With no criterion found, the ticket gets Escouade's default criteria.

## The Jira issue follows its ticket

In Settings › Projects › Imported tickets, the "Status mapping" gives, for each Kanban column, a status of the Jira project (those of all its issue types, each once). Picking the project fills it in: the status that looks like "in progress" for "In progress" and "To test", the one that looks like "to test" for "Done", nothing for "To do".

When an imported ticket reaches a column, if "Update the external status" is on:

- if the issue is already in that status, nothing is sent;
- otherwise the plugin reads the transitions its workflow allows from its current status, and takes the one that leads to the chosen status. If there is none, it forces nothing: the card says so (see **Troubleshooting**).

Escouade also writes a comment on it if the column is ticked under "Comment" (by default "To test" and "Done"). The comment is written in Markdown and turned into Atlassian's document format.

With "Post a summary on every loop", each new loop of an agent also leaves a comment: the criteria met and the agent's notes.

## Automatic import

Settings › Connected services › "Automatic import" imports on its own the open issues that carry the chosen label (`claude-ready` by default). For Jira, it is a label of the issue.

## Limits

- Jira Cloud only: a Jira Data Center or Server is not supported.
- One Jira account per Escouade, whatever its site, and one Jira project per project.
- The menu offers the first 500 projects of the site, sorted by key.
- The status follows the workflow: if no transition leads from the current status to the chosen one, it does not change.
- An answer from Jira cannot exceed 8 MiB nor take longer than 15 seconds.
- Escouade does not create an issue, change it (title, description, assignee) or delete it: it moves it along its workflow and comments on it, nothing more.
