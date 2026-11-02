# Jira, Trello, GitHub, Linear, ClickUp, GitLab, and Azure Boards

Escouade imports tickets from Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, and Azure Boards into the Kanban, with their acceptance criteria, then keeps their status up to date and leaves comments on them as the work goes. This page covers connecting, importing, and syncing.

![The “Import tickets” window: Jira tickets filtered by “Assigned to me”, with their detected criteria](/images/en/integrations.jpg)

## Connecting a service

Accounts are connected in Settings › Integrations, “Connected accounts” group (see [Integrations settings](settings-integrations)). For each service, “Connect…” opens a form. “Connect” checks the credentials with the service (“Checking…”), then the line reads “Connected · ” followed by who you are there (for example `ada@atlas.dev · atlas.atlassian.net`). If the service refuses, the reason shows under the form. The connection is saved at once: it does not go through “Save”.

| Service | What you give | Where to get it |
|---|---|---|
| Jira | “Site” (`atlas.atlassian.net`, over https), “Email”, and “API token” | “Create an Atlassian API token” |
| Trello | “API key” and “Token” | “Get an API key”, then “Get a token for this key” (available once the key is typed: a read and write token that never expires) |
| GitHub | “Token”, optional | “Create a GitHub token”. Left empty, Escouade uses the one from the `gh` CLI (`gh auth login`); the account then shows with “· via gh” |
| Linear | “API key” | “Create a Linear API key” |
| GitLab | Nothing more: the GitLab code host’s account (gitlab.com or your instance, an `api` token or the one from `glab`) | “Connect GitLab…” opens the form of Settings › Integrations › “Code hosts”. One account serves the merge requests and the tickets |
| ClickUp | “API key” (`pk_…`) | “Create a ClickUp API key” (ClickUp’s Settings › Apps). The row then says “Connected · ” followed by your name in ClickUp |
| Azure Boards | Nothing more: the Azure DevOps code host’s account (an organization of dev.azure.com or a collection of Azure DevOps Server; a personal access token with the Work Items right, read and write, or the one from `az`) | “Connect Azure DevOps…” opens the form of Settings › Integrations › “Code hosts”. One account serves the pull requests and the work items. Without the Work Items right, Azure refuses and the error says where to make another token (the organization’s tokens page) |

### Code host or source of tickets

A code host (GitHub, GitLab, Bitbucket, Gitea, Azure DevOps) serves the agents’ pull requests, CI and deployments; a source of the Kanban serves the tickets. Settings › Integrations shows them in two named parts. Connecting your GitLab as a code host puts no ticket in the Kanban: a GitLab project still has to be linked as a source (see below), which the import window offers in one click. Bitbucket and Gitea have no source yet: their row says “No tickets: PRs and CI only”.

“Disconnect” forgets the account and its secrets. Projects keep their links, unused until you connect again.

### Where the tokens are kept

Tokens stay on your machine, apart from your settings, and are only used for calls to these services. Escouade stores them in the system keychain: Credential Manager on Windows, Keychain on macOS. On Linux, which has no supported keychain, they stay in `~/.escouade/integrations.json`, readable by you alone.

When the keychain refuses a token on Windows or macOS, it stays in that file and the tab says so: “System keychain unavailable: the token stays in ~/.escouade/integrations.json.” If the keychain exists but cannot be read, the tab says “System keychain unreadable: restart Escouade or reconnect the account.” and no call goes out without the tokens. For GitHub through `gh`, Escouade keeps no token.

### The services a plugin serves

GitHub (the code host and GitHub Issues), GitLab (the code host; GitLab Issues is still part of the app), Bitbucket, Gitea and Forgejo, and Trello are served by plugins shipped with Escouade: nothing changes in use, the accounts and the forms are the same. An account served by another author’s plugin says so: “Served by the plugin …”. The token stays in the keychain; Escouade lends it to the plugin of that service alone, when it needs it.

These plugins are switched off in the Plugins view, for the whole app or for one project (see [the plugins shipped with Escouade](plugins/installer#the-plugins-shipped-with-escouade)). Switched off or stopped, a plugin says so instead of going quiet:

- **a code host**: the agent’s card shows “Code host stopped: PRs no longer followed” and keeps the PRs read last; Settings › Integrations says under the account “The plugin GitHub is switched off (Settings › Plugins): this account serves nothing, its PRs are no longer followed”, or that it stopped and why;
- **a source**: the sync stops, and its failure says “The plugin dev.escouade.trello is not answering: …”; the imported tickets stay.

Switched back on, the plugin picks up where it stopped: the first read that works clears the message.

## Linking a project to a source

A project links to one place per service: a Jira project, a Trello board, a GitHub repository, a Linear team, a ClickUp list, a GitLab project, or an Azure DevOps project. In Settings › Integrations, the “Sources linked to `<project>`” group offers, for each connected service, a list: “Jira project”, “Trello board”, “GitHub Issues repository”, “Linear team”, “ClickUp list”, “GitLab Issues project”, “Azure Boards project”. “None” unlinks the service. For GitHub, the project’s own repository, when its remote is on GitHub, comes first (“(the project’s repository)”); likewise for GitLab, when the remote is on a connected instance, and for Azure Boards, when it is on the connected organization. Azure DevOps projects are grouped by organization. GitLab projects are grouped by instance, with their groups and subgroups in the path (“acme/back/api”). For ClickUp, the lists of all your workspaces are grouped by space and folder (“Product › Sprints”), as in ClickUp’s sidebar: the lists outside a folder first, archived ones left out; typing the first letters of a name, with the menu open, goes to it.

Choosing a source loads its statuses and pre-fills the status mapping (see below). Then save the settings.

From the Kanban, “Import” with no linked source shows “No source linked to this project”, names the seven sources and offers “Link a source”, which opens this same tab at “Kanban sources”. When the project’s remote is on a connected code host whose source exists (GitHub, GitLab, Azure DevOps), it offers “Link acme/api (GitLab Issues)” right away: one click links the project with the default statuses, and its tickets show at once.

## Importing tickets

In the [Kanban](board), “Import” (⤓) opens the “Import tickets” window. It adds the tickets you pick to the end of “To do”.

- One tab per linked and connected source (the service, the source’s name, and the number of tickets you picked in it). “⚙ Manage sources” opens the settings.
- The search field searches the chosen source. The search runs 300 ms after the last keystroke.
- Filter chips are clicked to turn on or off. The field and the chips depend on the service:

| Service | Search field | Chips |
|---|---|---|
| Jira | “Search by key or text…” | “Assigned to me”, “Active sprint”, “To do” |
| Trello | “Search for a card…” | “My cards”, then one chip per list of the board |
| GitHub | “Search for an issue…” (a number, `42` or `#42`, finds the issue) | “Assigned to me”, then the repository’s labels (twelve at most) |
| Linear | “Search by identifier (ENG-42) or title…” | “My issues”, then one chip per workflow state that is neither completed nor canceled |
| GitLab | “Search for an issue (#12) or by text…” (a number, `12` or `#12`, finds the issue; a text searches the title and description) | “Assigned to me”, then the project’s labels (twelve at most) |
| Azure Boards | “Search for a work item (#12) or by title…” (a number, `12` or `#12`, finds the work item; a text searches the title) | “Assigned to me”, then one chip per work item type of the project (User Story, Bug, Task, Product Backlog Item, Issue… depending on its process), then its tags (twelve at most) |
| ClickUp | “Search by id (#86abx3, ABC-12) or title…” (a task’s id, with or without `#`, or its custom id when the space has them, finds the task if it is in the list) | “My tasks”, then one chip per status of the list that is neither done nor closed |

Only open tickets are offered: not those in Jira’s “Done” category, nor closed GitHub issues (pull requests are left out), nor completed or canceled Linear issues, nor closed GitLab issues, nor ClickUp tasks whose status is of the done or closed type, nor Azure Boards work items in a completed or removed state (the Completed and Removed categories of their process) or of a type that is no backlog’s (tests, reviews, feedback). For Azure Boards, a WIQL query gives the matching work items (1,000 at most, the latest changed first), then read 50 at a time. As ClickUp has no search by title, Escouade reads the list 100 tasks at a time and keeps those whose name holds the text, up to 5 pages before it shows what it found; “Show more” goes on.

Each row shows the key (for ClickUp, the custom id, otherwise `#` and the task’s id), the title, the type (Bug, Story, Task, Card, Issue, Incident, User Story…), the priority, the assignee, and the state, plus “✓ 2 criteria detected” (on hover: which ones). A ticket already in the Kanban is grayed out (“Already in the Kanban”) and cannot be picked. Tickets arrive in pages; “Show more” loads the next one, and the line above the list says “50 shown of 312” when the service counts more tickets than it has sent.

To import:

1. Check the tickets (one click, or `Space` or `Enter` on a row), or “Select all” for those in the list that are not already imported. The selection is kept from one search, one page, and one source to the next.
2. Choose “Max loops” (3, 5, or 8): the same number for every imported ticket.
3. Click “Import 3 tickets”.

Each imported ticket has its own Escouade key (`DEM-7`) and keeps, on its card, the badge of its service with its key there (`ATL-1294`): a click opens it in the service. Its description takes over the external ticket’s, and ends with a line “Jira ticket ATL-1294: `<address>`”. A message confirms (“3 tickets imported from Jira and Trello”). With the autopilot on, imported tickets start on their own (see [The autopilot](autopilot)).

### The acceptance criteria

With “Extract the acceptance criteria” on (the default), Escouade looks for the criteria in the ticket:

- Jira and GitHub: the list items under a heading or a line that names them (“Acceptance criteria”, “Definition of done”, “AC”, “DoD”, or the French “Critères d’acceptation”), otherwise the checkboxes (`- [ ]`) of the description;
- Trello: the items of the checklist that goes by this name, otherwise those of all the checklists, otherwise those the description lists;
- Linear and GitLab: as for Jira and GitHub, in the description;
- ClickUp: as for Trello, the items of the checklist that goes by this name, otherwise those of all the task’s checklists, otherwise those its description (in Markdown) lists;
- Azure Boards: the Acceptance Criteria field when it is filled in (its list items, otherwise its lines), otherwise the description as for Jira (for a bug, its repro steps when the description is empty). Azure Boards’ fields are HTML: Escouade turns them into Markdown.

It keeps twenty criteria at most, of 300 characters at most. With no criterion found, or with the setting off, the ticket gets the two default criteria. A Jira description is converted to Markdown before it is searched.

## Importing by label

Settings › Integrations › “Automatic import” lets Escouade import on its own:

| Setting | What it does | Default |
|---|---|---|
| “Import labeled tickets” | The open tickets of the linked sources that carry the label arrive in “To do” | Off |
| “Label” | The Jira, Trello, GitHub, Linear, ClickUp, GitLab, or Azure Boards label (a task’s tag) | `claude-ready` |
| “Check every” | The interval between two passes: 5, 15, or 60 min | 15 min |

A pass looks, in each project, at the linked sources whose account is connected. Tickets arrive with 5 loops at most, and a message announces them (“3 tickets imported from Jira into demo”). A ticket the project has already imported is never brought back, even if you have deleted it from the Kanban since. The first pass happens shortly after you turn it on.

## Status and comments follow the ticket

When an imported ticket changes column, Escouade updates the external ticket. The links, the status mapping, and the comments are set per project; the three settings of the “Sync” group apply to every project.

### Status mapping

The “Status mapping” group (once a source is linked) has one row per Kanban column and, for each linked service, a list of statuses. “— unchanged” leaves the external ticket as it is. Choosing a source pre-fills the columns like this:

- “In progress” and “To review”: the status that looks like “in progress” (`In Progress`, `En cours`, `Doing`, `WIP`, `In development`, `Started`…);
- “Done”: the status that looks like “to review” (`To test`, `Testing`, `QA`, `In Review`, `Review`, `Recette`, `À valider`, `Validation`, `Test`…);
- “To do”: unchanged.

For ClickUp, each list has statuses of its own: the mapping is therefore set per project, on those of the linked list. When their names say nothing of the above, their type decides: the first custom status (between the open one and the closed ones) for “In progress” and “To test”, the next one for “Done”, otherwise the status of the done type. A list with only an open and a closed status moves nothing by default. If the list no longer has the chosen status (renamed, removed), the card says so (“The ClickUp list “Sprint 12” has no “review” status: ABC-12 stays where it is; choose another status in Settings › Integrations”) rather than fail without a word.

What “status” means depends on the service: a workflow transition for Jira, the list the card sits in for Trello, a workflow state for Linear, a status of the list for ClickUp. For GitHub, they are “Open”, “Closed”, or a label of the repository (marked ◆): “Closed” closes the issue, a label reopens it if needed and removes the other labels of the mapping. GitLab does the same, with the labels of the lists of the project’s boards first (“Doing”, “To review”…), then its other labels; an issue changes its labels and state in one call, and the comment is a note. For Azure Boards, these are the states of the project’s work item types, by its process (Basic: To Do, Doing, Done; Agile: New, Active, Resolved, Closed; Scrum, CMMI or an inherited process: its own), ordered by category; “In progress” and “To review” take an in-progress state by default (Active, Doing, In Progress…), “Done” the resolved state when the process has one. The state goes in one call (`System.State`, as JSON Patch); when the work item’s type lacks that state (“Committed” of a Product Backlog Item for a Task), it takes its type’s state of the same category (“In Progress”). The comment is written in Markdown in the work item’s discussion.

The last column, “Comment”, chooses the columns whose arrival writes a comment (by default “To review” and “Done”). The comment says where the ticket stands:

| Column | Comment |
|---|---|
| “To do” | “Escouade: DEM-5 is back in “To do”.” |
| “In progress” | “Escouade: DEM-5 is picked up by an agent (branch ticket/dem-5).” |
| “To review” | “Escouade: DEM-5 is ready to review.” (with “(partial goal)” where it applies), the criteria (✓ or ○, with the agent’s note), and “What was done”. |
| “Done” | “Escouade: DEM-5 is done — ” followed by the outcome (“⤵ Merged into main · squash”), and the pull request’s link if there is one. |

### Sync settings

| Setting | What it does | Default |
|---|---|---|
| “Update the external status” | When an imported ticket changes column, its external ticket takes the status of the mapping | On |
| “Post a summary on every loop” | At each new loop, a comment gives the criteria met and the agent’s notes (“Escouade: DEM-5, loop 2/5 — 1/3 criteria met.”) | Off |
| “Extract the acceptance criteria” | See above | On |

### The language of comments

Comments are published for your team: they are written in the “Language of texts written by the agents” (Settings › Application, see [Application settings](settings-application)), not in the interface’s language. The outcome of a done ticket is quoted there in that same language.

## Failed syncs

Every change to pass on is kept in a queue, `~/.escouade/sync-queue.json`, until the service has taken it. A ticket’s changes go in order, one after the other: a change that fails holds back the next ones of the same ticket, not those of other tickets. A status transition replaces the ticket’s one that has not gone out yet, and a ticket that goes back to an earlier column cancels the transition that had not gone out.

A card whose sync failed shows a ⚠. A click on it shows the reason (“Jira refuses these credentials (401)”, “Jira: no transition from “In progress” to “Done” for ATL-1294”, “The ClickUp list 901 no longer exists or can’t be reached any more: choose another one in Settings › Integrations”…) and the “Resync” button.

- When the service can’t be reached, is down or overloaded, or refuses the credentials (which you can fix), the failure may pass later: Escouade tries again on its own, after 1, 5, 15, then 60 minutes, the last wait being repeated, for 24 hours from the first failure.
- A refusal that trying again would not change (a request refused as it is, a ticket gone, no transition to the status) is set aside at once, like anything that still fails after a day. It no longer holds back the ticket’s next changes.
- When ClickUp limits the requests (429), Escouade waits for the end of the limit it announces (`X-RateLimit-Reset`, one minute at most) and asks once more before it counts the failure.
- When Escouade starts, everything that was waiting goes again (except what was set aside).
- “Resync” sends again right away, set-aside changes included. When nothing is left to send, it gives the external ticket the status of its column again (without writing a comment twice). The ⚠ goes as soon as everything has gone through.

Deleting the ticket from the Kanban drops whatever was waiting for it.

## See also

- [The Kanban](board): the card of an imported ticket.
- [Integrations settings](settings-integrations): the complete list of settings.
- [The MCP server](mcp): a ticket’s external ticket is read with `get_ticket`.
