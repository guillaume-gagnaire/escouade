# Use

The plugin serves two things of Escouade on the same account: the GitLab code host (merge requests, their pipelines, discussions and deployments) and the GitLab Issues Kanban source. Everywhere in the app, a pull request is called "MR" on GitLab.

## Merge requests

**Open an MR.** "Open a MR" on an agent, or approving a ticket with "Open a pull request" as its action, has Escouade push the branch, then asks the plugin to open the merge request: its title, its description, its branch and its target, its reviewers (their logins, which the plugin turns into GitLab ids) and its labels. A draft MR has a title that starts with `Draft:`. When it closes an issue, the description ends with "Closes #12".

**Follow it.** The agent's card and its MR's tab show its state, draft or not, its conflicts, its approvals, its open threads, its auto-merge and its pipeline's state. Projects are named by their path, groups and subgroups included (`gitlab-org/api/client-go`).

**Pipelines.** The jobs of the MR's head pipeline (a job allowed to fail does not fail the pipeline), a job's log, which "Agent fixes it" gives the agent, and running the pipeline's failed jobs again.

**Merge.** Merge commit or squash, as the project's squash setting allows; the project's setting says whether GitLab rebases first. Rebase and semi-linear merge are not asked from Escouade. "Merge when green" asks for GitLab's auto-merge, which merges once the pipeline passes.

**Reviews.** The plugin reads the approvals, the reviewers' say and the discussions (on a line of the diff or not, system notes aside). It replies in a discussion, resolves it, comments, and publishes the review you approved: its lines as discussions on the diff, its text, and your approval.

**Deployments.** For a branch, the last deployment of each environment: a done ticket's card says where it runs, and the "Environments" view shows one lane per environment.

**The "Pull requests" view.** It shows the 30 most recently updated open MRs of the project, yours or others'.

## GitLab Issues

Each GitLab account is also a Kanban source.

**Import.** In the "Import tickets" window, the GitLab tab shows the open issues of the linked project, 50 at a time, with their total count:

- "Search for an issue (#12) or by text…": a number (`12` or `#12`) finds the issue; any other text looks for it in the title and description;
- "Assigned to me", then one chip per label of the project (twelve at most);
- each row shows the type (Issue, Incident, Task), the author ("by @ada"), the assignee ("Unassigned" otherwise) and the labels.

**Criteria.** With "Extract the acceptance criteria", they are the list items under a heading or a line that names the criteria, otherwise the description's task list items (`- [ ]`).

**The issue follows its ticket.** In the "Status mapping", an issue's states are "Open", "Closed", then the labels of the lists of the project's boards ("Doing", "To review"…), then its other labels. "Closed" closes the issue; a label reopens it if need be, adds it and removes the mapping's other labels, in a single call. The comment is a note on the issue, in Markdown.

**Automatic import** takes the open issues that carry the chosen label (`claude-ready` by default).

## Limits

- Written from GitLab's documentation and tried against answers recorded from gitlab.com: not yet against a real instance with a token.
- The GitLab Issues project menu offers the 100 projects you are a member of, the most recently active first, archived ones left out.
- The plugin spares the API's limit: it only reads the open threads when GitLab says there are some, slows down as the limit comes near, and waits what GitLab asks after a refusal (429), 30 seconds at most, before asking once more.
- It does not rerun nor approve a deployment.
- An answer from GitLab cannot exceed 8 MiB nor take longer than 15 seconds.
