# Use

The plugin serves two things of Escouade on the same account: the Azure DevOps code host (Azure Repos and Azure Pipelines) and the Azure Boards Kanban source.

## Pull requests

**Open a pull request.** "Open a PR" on an agent, or approving a ticket with "Open a pull request" as its action, has Escouade push the branch, then asks the plugin to open the pull request: its title, its description (Azure refuses more than 4,000 characters), its branch and its target, draft or not, its reviewers, its labels, and the work item it closes once completed.

**Follow it.** The agent's card and its pull request's tab show its state, draft or not, its conflicts, its reviewers' votes (an approval counts from "Approved with suggestions" up) and its open threads. The "Pull requests" view shows the repository's 30 most recently updated open pull requests.

**Checks.** The evaluations of the branch's build and status policies, the statuses posted on the pull request, and the Azure Pipelines builds of the merge commit no policy holds. "Agent fixes it" gives the agent the log of the failed tasks; retrying a failed build runs it again.

**Merge.** By the strategies the target branch's policy allows: merge, squash, rebase, semi-linear. A strategy the branch does not allow is refused, and the message says those it allows. "Merge when green" sets Azure's auto-complete.

**Reviews.** The plugin reads the comment threads people wrote (not Azure's own: votes, pushes), replies in a thread, resolves it, and publishes the review you approved: its comments, then your vote.

**No deployments.** Azure Pipelines' environments say nothing simple about a branch: Escouade only follows the CI there.

## Azure Boards

Each Azure DevOps account is also a Kanban source.

**Import.** In the "Import tickets" window, the Azure Boards tab shows the open work items of the linked project, the latest changed first. A WIQL query gives those that match (1,000 at most), then read 50 at a time:

- "Search for a work item (#12) or by title…": a number (`12` or `#12`) finds the item; any other text looks for it in the title;
- "Assigned to me", then one chip per work item type of the project (User Story, Bug, Task, Product Backlog Item, Issue… depending on its process), then its tags (twelve at most);
- not offered: items in a done or removed state (the Completed and Removed categories of their process), nor those of types that are not a backlog's (tests, reviews, feedback).

**Criteria.** With "Extract the acceptance criteria": the Acceptance Criteria field when it is filled in (its list items, otherwise its lines), otherwise the description as for Jira; for a bug with an empty description, its repro steps. Azure Boards' fields are HTML: the plugin turns them into Markdown.

**The item follows its ticket.** In the "Status mapping", the states are those of the project's work item types, according to its process (Basic: To Do, Doing, Done; Agile: New, Active, Resolved, Closed; Scrum, CMMI or an inherited process: its own), sorted by category. By default, "In progress" and "To test" take an in-progress state (Active, Doing, In Progress…), "Done" the resolved state when the process has one. The state goes in a single call; when the item's type does not have that state, it takes its type's state of the same category. The comment is written in Markdown in the item's discussion.

**Automatic import** takes the open items that carry the chosen tag (`claude-ready` by default).

## Limits

- Written from Microsoft's documentation (REST API 7.1) and tested against a fake server: not yet tried on a real organization.
- The `az` CLI only serves on dev.azure.com.
- Azure does not tell a token's scopes: a missing scope shows at the first refusal.
- No deployments.
- When Azure limits the requests (429, or 503 with `Retry-After`), the plugin waits once, 30 seconds at most; it also slows down when Azure announces its limit.
- An answer from Azure cannot exceed 8 MiB nor take longer than 15 seconds.
