# Use

The plugin serves two things of Escouade on the same account: the GitHub code host (pull requests, their CI, reviews and deployments) and, on the github.com account, the GitHub Issues Kanban source.

## Pull requests

**Open a pull request.** "Open a PR" on an agent, or approving a ticket with "Open a pull request" as its action, has Escouade push the branch, then asks the plugin to open the pull request: its title, its text, its branch and its base, draft or not, its reviewers and its labels. When it closes an issue, the text ends with "Closes #42". A reviewer or a label GitHub refuses (the author themselves, an account with no access) does not fail the pull request, which stays open.

**Follow it.** The agent's card and its pull request's tab in the side panel show its state, draft or not, its conflicts, its reviews, its open threads and its CI. All the pull requests a project follows are read in a single GraphQL request.

**CI.** The check runs and statuses of the pull request's last commit (or of the branch, when there is no pull request). For an Actions job, "Agent fixes it" gives the agent its log; running the failed jobs again reruns each run once.

**Merge.** The merge follows what the repository allows (merge, squash, rebase). "Merge when green" asks for GitHub's auto-merge if the repository allows it; otherwise Escouade itself waits for the pull request to be approved and green, while the app runs.

**Reviews.** The plugin reads the pull request's reviews and comments, with their threads. When you have an agent review a pull request, the plugin publishes at once what you kept: the text, the line comments and your choice ("Comment", "Request changes", "Approve"). It also replies under a line comment.

**Deployments.** The plugin reads the deployments of a commit and those of the repository's environments: a done ticket's card says where it runs, and the "Environments" view shows one lane per environment.

**The "Pull requests" view.** It shows the 30 most recently updated open pull requests of the repository, yours or others'.

## GitHub Issues

On the github.com account, the plugin is also a Kanban source.

**Import.** In the "Import tickets" window, the GitHub tab shows the open issues of the linked repository, pull requests aside:

- "Search for an issue…": a number (`42` or `#42`) finds the issue; any other text goes through GitHub's search, 50 at a time, with the total count;
- "Assigned to me", then one chip per label of the repository (twelve at most);
- with no search nor chip, the list comes 100 issues at a time.

**Criteria.** With "Extract the acceptance criteria", they are the list items under a heading or a line that names the criteria ("Acceptance criteria", "Critères d'acceptation", "Definition of done", "AC", "DoD"…), otherwise the description's task list items (`- [ ]`).

**The issue follows its ticket.** In the "Status mapping", an issue's states are "Open", "Closed", then the repository's labels (marked ◆). "Closed" closes the issue (completed); a label reopens it if need be, adds it, and removes the mapping's other labels. The comment is written in the issue's discussion.

**Automatic import** takes the open issues that carry the chosen label (`claude-ready` by default).

## Limits

- GitHub Issues only serves on the github.com account, not on a GitHub Enterprise.
- The GitHub Issues repository menu offers the 100 repositories you pushed to most recently (yours, those you collaborate on, your organizations').
- The plugin spares the API's limit: it reads again conditionally (a `304` answer does not count), slows down as the limit comes near, and waits what GitHub asks when it refuses for its secondary limit (a minute if it says nothing) before asking once more.
- An Actions job's log on an Enterprise Server that stores it on a host other than itself is not read: nothing names that host in the plugin's permissions.
- It does not rerun nor approve a deployment.
- An answer from GitHub cannot exceed 8 MiB nor take longer than 15 seconds.
