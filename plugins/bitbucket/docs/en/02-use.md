# Use

The plugin is Escouade's Bitbucket Cloud code host: the pull requests of your agents and tickets, their CI, reviews and deployments. It brings no source to the Kanban.

## Pull requests

**Open a pull request.** "Open a PR" on an agent, or approving a ticket with "Open a pull request" as its action, has Escouade push the branch, then asks the plugin to open the pull request: its title, its text, its branch and its target, draft or not, its reviewers, and the issue it closes. Bitbucket has no labels on pull requests: requested labels go nowhere.

**Follow it.** The agent's card and its pull request's tab show its state, approvals, change requests, open threads and checks. The "Pull requests" view shows the repository's 30 most recently updated open pull requests.

**CI.** The statuses posted on the pull request's last commit are its checks. A Bitbucket Pipelines run has a log, read step after step, which "Agent fixes it" gives the agent. Running failed jobs again does not exist: Bitbucket's API offers none.

**Merge.** With the strategy asked: merge commit, squash, rebase (fast-forward) or rebase then merge commit. When Bitbucket merges in the background, the plugin waits for its task to end, about thirty seconds at most.

**No auto-merge.** Bitbucket's API merges at once; it cannot merge later. "Merge when green" therefore has Escouade itself wait: it merges as soon as the pull request is approved and green, while the app runs.

**Reviews.** The plugin reads line comments with their threads, replies under a comment, resolves a thread, and publishes the review you approved: its line comments, then the approval or the request for changes.

**Deployments.** Those of a commit, by environment, and those of the repository's environments: a done ticket's card says where it runs, and the "Environments" view shows one lane per environment.

## The limit by the hour

Bitbucket Cloud allows 1,000 calls an hour (more for large workspaces) and does not say how many remain. The plugin counts its own calls over the hour, believes the limit Bitbucket announces when it does, and slows down as the budget runs out. A refusal for the limit (429) is waited out, 30 seconds at most, then the call goes once more; refused again, the hour is taken as spent.

## Limits

- Bitbucket Cloud only; not Bitbucket Data Center.
- Written from Bitbucket Cloud's API 2.0 documentation and tested against a fake server: nobody has tried it on bitbucket.org yet.
- No auto-merge, no labels, no rerun of failed jobs.
- No ticket source: the account's row says "No tickets: PRs and CI only".
- An answer from Bitbucket cannot exceed 8 MiB nor take longer than 15 seconds.
