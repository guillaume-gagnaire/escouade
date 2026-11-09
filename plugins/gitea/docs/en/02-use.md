# Use

The plugin is Escouade's Gitea / Forgejo code host (Codeberg is a Forgejo): the pull requests of your agents and tickets, their CI and their reviews. It brings no source to the Kanban.

## Pull requests

**Open a pull request.** "Open a PR" on an agent, or approving a ticket with "Open a pull request" as its action, has Escouade push the branch, then asks the plugin to open the pull request: its title, its text, its branch and its target, its reviewers, its labels (the repository's whose name matches) and the issue it closes. Gitea and Forgejo have no "draft" field to write: a draft pull request has a title that starts with `WIP:`.

**Follow it.** The agent's card and its pull request's tab show its state, draft or not, its reviews, threads and CI. The "Pull requests" view shows the repository's 30 most recently updated open pull requests.

**CI.** The combined statuses of the last commit. Those Actions posts are tied to their job, whose log can be read ("Agent fixes it" gives it to the agent) and whose failed jobs can be run again, depending on the instance's version (see below).

**Merge.** With each strategy the repository allows. "Merge when green" schedules the instance's auto-merge, which it only does on a **protected branch**: the plugin reads the target branch first, and refuses, saying so, when it is not one. Otherwise Escouade itself waits for the pull request to be approved and green, while the app runs.

**Reviews.** The plugin reads the reviews (with their lines) and the comments, publishes the review you approved, and replies in a thread or resolves it where the instance allows it.

## What depends on the version

The plugin asks the instance for its version (`/api/v1/version`) once per account. What its version lacks, it says instead of failing or answering wrong, and Escouade shows nothing for it.

| | Gitea | Forgejo |
| --- | --- | --- |
| Deployments | none (no API) | none (no API) |
| Actions: jobs and logs | 1.25 and later | 12 and later |
| Actions: run failed jobs again | 1.26 and later | no |
| Reply in a thread, resolve it | 1.26 and later | no |

An older instance keeps its statuses (with their address), with no job nor log. **These minimum versions come from Gitea's and Forgejo's sources, not from a trial.**

## Limits

- Written from the API documents of Forgejo 16 and of Gitea's main branch, and tested against a fake server: not yet tried on a real Codeberg nor a real Gitea.
- No deployments: Escouade only follows the CI there.
- Auto-merge only works on a protected branch.
- No ticket source: the account's row says "No tickets: PRs and CI only".
- An answer from the instance cannot exceed 8 MiB nor take longer than 15 seconds.
