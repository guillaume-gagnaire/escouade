# Settings: Connected services and Imported tickets

The “Connected services” page connects the code hosts and the accounts of Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, and Azure Boards, and sets the sync and the automatic import, for all projects. Each project’s “Imported tickets” page links it to its sources and maps their statuses.

An amber dot on “Connected services”, in the settings bar, says a token expired or can’t be read.

## Open the pages

Open the settings with `Ctrl+,` (`⌘,` on macOS), then choose “Connected services”, or, under “Projects”, the project’s “Imported tickets” page. Imported tickets and their sync are explained in [integrations](integrations).

Connected services has two parts, each with its title and a line that says what it is for:

- **“Code hosts”**: “The agents’ PRs and MRs, CI and deployments: one account per instance, which each project finds through its remote.” (see [below](#code-hosts));
- **“Kanban sources”**: “Import and sync of tickets: an imported ticket follows its column there (status, comments).” The rest of the page is under it.

A code host is not a source: connecting GitLab or GitHub as a code host serves the PRs and CI; its tickets come into the Kanban only once a source is linked to the project. GitHub Issues, GitLab Issues and Azure Boards take their code host’s account for that, asking nothing again.

The sources bring together three kinds of settings:

- “Connected accounts” is shared by the whole app and is saved right away.
- “Sources linked to …” and “Status mapping” are on each project’s “Imported tickets” page. They save as you set them.
- “Sync” and “Automatic import” are shared by all projects, and save as you set them.

With no project open, “Imported tickets” shows “No project open.”.

## Connected accounts

One account per service: Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues, and Azure Boards. Each service shows “Not connected” with a “Connect…” button, or “Connected · ” followed by the account with a “Disconnect” button. Under the title, the app reminds you: “Tokens stay on this machine, stored apart from your settings, and are only used for calls to these services.”

GitLab Issues has no account of its own: its row says “The GitLab code host’s account: it is connected and disconnected in Code hosts.”, and its “Connect GitLab…” button opens the code host’s form (see [GitLab Issues](#gitlab-issues)). Azure Boards likewise with the Azure DevOps code host: “The Azure DevOps code host’s account: it is connected and disconnected in Code hosts.”, “Its token needs the Work Items right (read and write).” and “Connect Azure DevOps…” (see [Azure Boards](#azure-boards)).

“Connect…” opens the service’s form, whose fields are described below. Tokens and the API keys of Linear and ClickUp are masked as you type; Trello’s API key isn’t.

### Jira

| Field | What to enter |
|---|---|
| “Site” | The address of your Atlassian site, for example `atlas.atlassian.net`. Without `https://`, Escouade adds it. Only https is accepted (http only to this machine). |
| “Email” | The email of the account the token belongs to, for example `ada@atlas.dev`. |
| “API token” | An Atlassian API token. |

The link “Create an Atlassian API token” opens `https://id.atlassian.com/manage-profile/security/api-tokens`. Once connected, the account shows the email and the site, for example “ada@atlas.dev · atlas.atlassian.net”. What Jira calls the place of the tickets is a “Project”.

### Trello

| Field | What to enter |
|---|---|
| “API key” | Trello’s API key. |
| “Token” | A token for that key. |

The link “Get an API key” opens `https://trello.com/power-ups/admin`. The link “Get a token for this key”, grayed out while the key is empty, opens Trello’s authorization page for the key you typed: read and write access, no expiration, in the name of “Escouade”. The place of the tickets is a “Board”.

### GitHub Issues

| Field | What to enter |
|---|---|
| “Token (empty: the one from gh)” | A GitHub token. Left empty, Escouade asks the GitHub CLI for its token (`gh auth token`): `gh` must be installed and logged in (`gh auth login`). |

The link “Create a GitHub token” opens `https://github.com/settings/tokens`. An account that goes through `gh` shows “· via gh”. The place of the tickets is a “Repository”.

### Linear

| Field | What to enter |
|---|---|
| “API key” | A Linear personal API key. |

The link “Create a Linear API key” opens `https://linear.app/settings/account/security`. The place of the tickets is a “Team”.

### ClickUp

| Field | What to enter |
|---|---|
| “API key” | A ClickUp personal API key (it starts with `pk_`). |

The link “Create a ClickUp API key” opens `https://app.clickup.com/settings/apps`. Once connected, the account shows with your name in ClickUp, for example “Connected · Ada Lovelace”. The place of the tickets is a “List”. Connecting a ClickUp app through OAuth isn’t offered: a personal key is enough and stays on your machine.

### GitLab Issues

No field: GitLab Issues takes the GitLab code host’s accounts (gitlab.com and self-hosted instances, one account per instance), with their personal token (`api` scope) or the one from the `glab` CLI. The token stays in the system keychain, as the code host’s does: it is the same one. “Connect GitLab…” opens the “Code hosts” form with GitLab already chosen: address (empty: gitlab.com), token (empty: the one from `glab`). Once connected, the row says “Connected · @ada”, followed by the instance when it is not gitlab.com (“@ada · git.acme.dev”). To disconnect it, disconnect the instance in “Code hosts”. The place of the tickets is a “Project”.

### Azure Boards

No field: Azure Boards takes the Azure DevOps code host’s accounts (an organization of dev.azure.com, or a collection of Azure DevOps Server), with their personal access token or the one from the `az` CLI. The token needs the Work Items right (read and write) besides Code and Build; without it, Azure refuses, and the error says so with the address of the organization’s tokens page. “Connect Azure DevOps…” opens the form of “Code hosts”, Azure DevOps already chosen: address (`dev.azure.com/acme`), token (empty: the one from `az`). Once connected, the row says “Connected · ada@acme.dev · acme” (the account, then the organization). To disconnect it, disconnect the organization in “Code hosts”. The place of the tickets is a “Project”.

### Connect and disconnect

Click “Connect” once the fields are filled in; the button reads “Checking…” during the call, and “Cancel” closes the form. Escouade checks the credentials with the service and only keeps them if it accepts them. If it refuses them, or can’t be reached, its answer shows in red under the fields, for example “Jira refuses these credentials (401)”. A missing field is reported before any call, for example “Give the token.”.

“Disconnect” forgets the account and its secrets. Projects keep their linked sources: they’re unused until an account of that service is connected again. To switch accounts, disconnect the old one, then connect the other.

### Where tokens are kept

Keys and tokens go to the system keychain (Credential Manager on Windows, Keychain on macOS); the rest of the account is in `~/.escouade/integrations.json`. On Linux, where Escouade knows of no keychain, and when the keychain refuses a write, the secrets stay in that file, readable by you alone. The account then says so: “System keychain unavailable: the token stays in ~/.escouade/integrations.json.”

If the keychain doesn’t give a token back at startup, the account shows “System keychain unreadable: restart Escouade or reconnect the account.” and no call goes out as long as that’s the case. See [local data](data).

## Sources linked to a project

The group is titled “Sources linked to” followed by the project’s name. While no account is connected, it says: “Connect an account above to link a source to this project.”

For each connected service, a menu chooses the project’s source: “Jira project”, “Trello board”, “GitHub Issues repository”, “Linear team”, “ClickUp list”, “GitLab Issues project”, or “Azure Boards project”. “None” (the default) unlinks. A project has at most one source per service.

- **Jira**: the projects of the site, shown as “KEY — name”.
- **Trello**: your open boards.
- **GitHub Issues**: the repositories you have access to (owner, collaborator, or organization member), the latest pushed first. The project’s own repository, read from its git remote, comes first with “(the project’s repository)”.
- **Linear**: your teams, shown as “Name (KEY)”.
- **GitLab Issues**: the projects you are a member of, on each connected instance (grouped under the instance), their full path with groups and subgroups (“acme/back/api”), the most recently active first. The project of the project’s remote comes first with “(the project’s repository)”. Groups are not offered: issues live in projects.
- **Azure Boards**: the projects of the connected organization (grouped under it), by name. The project of the project’s remote comes first with “(the project’s repository)”.
- **ClickUp**: the lists of your workspaces, grouped under their space and folder (“Product › Sprints”; the workspace first when you have several), the lists outside a folder first, archived ones left out.

Choosing a source loads its statuses and pre-fills the mapping below. If the statuses don’t load, the source is linked anyway, and the tooltip of the mapping menus says why.

## Status mapping

This group only appears once a source is linked on a connected account. It says: “The status the external ticket takes when its ticket reaches the column; “Comment” adds a comment there (criteria, outcome of the approval…).”

One row per Kanban column (“To do”, “In progress”, “To review”, “Done”), one menu per linked service, and a “Comment” switch at the end of the row.

### The status of each column

Each menu offers “— unchanged” (the external ticket isn’t touched) and the source’s statuses:

| Service | Statuses offered | What the status change does |
|---|---|---|
| Jira | The statuses of the project’s issue types | Escouade applies the workflow transition that leads to that status. If the workflow doesn’t lead there from the current status, it’s refused. |
| Trello | The board’s open lists | The card moves into the list. |
| GitHub Issues | “Open”, “Closed”, and the repository’s labels (preceded by ◆) | “Closed” closes the issue as completed; “Open” reopens it. A label is put on the issue (reopened if need be), and the other labels of the mapping are taken off. |
| Linear | The states of the team’s workflow | The ticket takes that state. |
| GitLab Issues | “Open”, “Closed”, the labels of the lists of the project’s boards, then its other labels (preceded by ◆) | As for GitHub: “Closed” closes the issue, “Open” reopens it, a label is put on it (the issue reopened if needed) and the other labels of the mapping removed, in one call. |
| Azure Boards | The states of the project’s work item types by its process (for example New, Active, Resolved, Closed, or To Do, Doing, Done), by category, removed states left out | The state is set in one call (`System.State`, JSON Patch). A work item whose type lacks that state takes its type’s state of the same category. By default, “In progress” and “To review”: an in-progress state; “Done”: the resolved state, when the process has one. |
| ClickUp | The statuses of the linked list (each list has its own) | The task takes that status. If the list no longer has it, the card says so and nothing is sent. |

When you link a source, Escouade pre-fills the menus from the statuses’ names: the one that looks like “in progress” (“In Progress”, “En cours”, “Doing”…) for “In progress” and “To review”, and the one that looks like “to test” (“To test”, “QA”, “In Review”, “À tester”…) for “Done”. “To do” stays “— unchanged”. For ClickUp, when the names say nothing, the statuses’ type decides: the first custom status for “In progress” and “To review”, the next one (or the one of the done type) for “Done”. Fix the menus if your workflow is different.

### Comment

A column’s “Comment” switch adds a comment on the external ticket when a ticket reaches the column. It applies to all of the project’s sources. By default it’s on for “To review” and “Done”, and off for “To do” and “In progress”.

The comment is written in the language of “Language of texts written by the agents” (in the [“Application”](settings-application) tab):

| Column | Comment |
|---|---|
| “To do” | “Escouade: ATL-42 is back in “To do”.” |
| “In progress” | “Escouade: ATL-42 is picked up by an agent (branch ticket/ATL-42).” |
| “To review” | “Escouade: ATL-42 is ready to review.”, or “… (partial goal).” when the ticket arrives without all its criteria for lack of loops. The list of criteria and “What was done” follow when the ticket has them. |
| “Done” | “Escouade: ATL-42 is done — ” followed by the outcome of the approval, then the PR’s link if there is one. |

## Sync

These three settings apply to all projects.

| Setting | Effect | Default |
|---|---|---|
| “Update the external status” | When an imported ticket changes column in the Kanban, the external ticket takes the status the mapping gives the column. Off, statuses are no longer touched; the mapping’s comments are still posted. | On |
| “Post a summary on every loop” | When a ticket begins a new loop (from the second one on), a comment gives the criteria met and the agent’s notes on the external ticket. | Off |
| “Extract the acceptance criteria” | On import, reads the criteria from the imported ticket’s description (Jira, GitHub, Linear, GitLab), checklist (Trello, ClickUp) or Acceptance Criteria field (Azure Boards, otherwise its description). Off, the ticket gets the default criteria. | On |

If a service refuses or can’t be reached, the operation isn’t lost: it’s tried again after 1, 5, 15, then 60 minutes, for 24 hours at most. The ticket’s card then shows the error and “Resync” to try again right away. A refusal that trying again wouldn’t change (a deleted ticket, a workflow that doesn’t lead to the status, a ClickUp list gone or a status it no longer has) is set aside without waiting. A ClickUp request limit (429) is waited out until its announced end, one minute at most, then the request goes once more.

## Automatic import

These three settings also apply to all projects. Importing by hand is the “Import” button in the Kanban header: see [integrations](integrations).

| Setting | Effect | Default |
|---|---|---|
| “Import labeled tickets” | Open tickets from the linked sources that carry the label arrive in “To do”, with 5 loops at most. | Off |
| “Label” | The label’s name: a Jira, Trello, GitHub, Linear, ClickUp, GitLab, or Azure Boards label. Empty, nothing is imported. | `claude-ready` |
| “Check every” | The interval between two checks: 5 min, 15 min, or 60 min. | 15 min |

The automatic import brings a ticket in only once per project: a ticket already imported, even deleted from the Kanban since, isn’t brought back on its own (importing by hand can take it again, as long as it is no longer in the Kanban). Each import shows a message like “3 tickets imported from Jira into demo”. The first check happens about 30 seconds after the app starts, or at most 30 seconds after you save if you turn the import on later; the next ones follow at the chosen interval.

## Code hosts

The first part of “Connected services” connects the [code hosts](code-hosts): GitHub, GitLab, Bitbucket Cloud, Gitea, Forgejo, and Azure DevOps. They serve the pull requests (merge requests on GitLab) of agents and tickets, the CI, and the deployments, not the tickets themselves. One account per instance (github.com, gitlab.com, your company’s…); each project finds its own by its remote. “Connect a code host…” asks for the host, the address (empty: its public instance), and a token (empty: the one from its CLI, `gh`, `glab`, or `az`); a remote without an account offers “Connect” followed by its host.

Each instance says on its row whether it also serves as a source of the Kanban:

- “Also used for GitHub Issues”: the github.com account;
- “Also used for GitLab Issues”: every GitLab account, gitlab.com or self-hosted;
- “Also used for Azure Boards”: every Azure DevOps account;
- “No tickets: PRs and CI only”: Bitbucket, Gitea, Forgejo and a GitHub Enterprise.

### Azure DevOps

An Azure DevOps account is an organization’s: its address is `dev.azure.com/<organization>` (a former `<organization>.visualstudio.com` is the same one), or for Azure DevOps Server the address of its collection, such as `https://server/tfs/DefaultCollection`. Its remotes are read in every form: `https://dev.azure.com/<organization>/<project>/_git/<repository>`, `<organization>.visualstudio.com`, ssh `git@ssh.dev.azure.com:v3/<organization>/<project>/<repository>`, and a Server’s.

The token is a personal access token (PAT), made on the page “Create a token” opens, with these rights: Code (read and write), Build (read and execute), and Work Items (read and write). Left empty, Escouade takes the one of the `az` CLI when you are logged in to it (`az login`), asked again before it expires. The row then says who the token is, by the e-mail of its account.

On Azure DevOps, an agent’s pull request opens as a draft or not, from “Open a PR”. Its card shows the reviewers’ votes, its conflicts, and its checks: the build and status policies of the branch, the statuses posted on the pull request and the Azure Pipelines builds. “Agent fixes it” hands it the log of the failed tasks; “Agent addresses the feedback” the comments of the open threads. The merge follows the strategies the target branch allows (merge, squash, rebase, semi-linear), and auto-merge is Azure’s auto-complete.
