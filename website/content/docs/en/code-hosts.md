# Code hosts

Escouade works with GitHub, GitLab, Bitbucket Cloud, Gitea, Forgejo, and Azure DevOps: it opens the pull requests of your agents and tickets there, and follows their CI, reviews, and deployments from the app.

## What a code host brings

A project’s remote says which host it is on. With an account connected for that instance, Escouade can:

- open the pull request of an [agent](worktrees#opening-a-pull-request) or of an [approved ticket](settings-board#draft-pr), and follow it on its card;
- show the [project’s open PRs](#the-pull-requests-view), the team’s as well as yours, and start an agent on one of them;
- have [an agent review a PR](#get-a-review-from-an-agent);
- give an agent the failed CI or the review feedback to fix;
- say [where a merged ticket runs](#environments-and-deployments);
- [tell you](#news-from-the-code-hosts) when a PR or a deployment has news.

A PR is called an “MR” on GitLab: the app uses the host’s word everywhere.

Each host is a plugin that ships with Escouade: it shows in [Settings › Connected services](settings-integrations#code-hosts) and turns off like [any plugin](plugins/introduction). Its PR then stays on the card as it was last read, and the card says so.

## Supported hosts

| Host | Instances | Token | Merge when green | Rerun jobs | Deployments |
|---|---|---|---|---|---|
| GitHub | github.com, GHE.com, GitHub Enterprise Server | Personal token, or the one from `gh` | GitHub’s auto-merge if the repository allows it, otherwise Escouade waits | Yes | Yes |
| GitLab | gitlab.com, self-hosted instances | Personal token with the `api` scope, or the one from `glab` | GitLab’s auto-merge | Yes | Yes |
| Bitbucket Cloud | bitbucket.org | Atlassian email and API token | Escouade waits: no auto-merge | No | Yes |
| Gitea, Forgejo | Codeberg, gitea.com, self-hosted instances | Access token | Auto-merge on a protected branch, otherwise Escouade waits | Depends on the instance’s version | No |
| Azure DevOps | dev.azure.com, `*.visualstudio.com`, Azure DevOps Server | Personal token, or the one from `az` on dev.azure.com | Azure’s auto-complete | Yes | No |

“Escouade waits”: the host doesn’t merge by itself, so Escouade merges as soon as the PR is approved and green, for as long as the app runs.

### What has been tried

> [!WARNING]
> GitHub is the only host tried on the real service, and only for reading: the PRs, checks, reviews, threads, deployments, and logs of a real public repository. Opening, merging, commenting, and publishing have only run against the fake GitHub of Escouade’s tests.
>
> GitLab, Bitbucket Cloud, Gitea, Forgejo, and Azure DevOps are written from their documentation and tested against fake servers. None has run with a real account yet: expect differences, and start on a repository with nothing at stake.

## Connect an account

In [Settings › Connected services](settings-integrations#code-hosts), “Connect a code host…” asks for the host, the instance’s address, and a token. One account per instance: each project finds its own by its remote, and a repository without an account offers to connect.

Without an account, Escouade doesn’t block anything: approving a ticket pushes the branch and opens the host’s page for creating the PR in your browser, filled in.

## Agents’ PRs

“Open a PR” pushes an agent’s branch and opens its PR: see [Opening a pull request](worktrees#opening-a-pull-request). The “PR and CI” tab of the side panel shows its CI, reviews, and deployments: see [the interface](interface#pr-and-ci).

## Tickets’ PRs

With “Open a pull request” as the approval action, an approved ticket waits for its merge in the “In review” column: see [the Kanban](board#the-in-review-column).

## The “Pull requests” view

In a project’s sidebar, “PR” shows, beside the Kanban, the 30 most recently updated open PRs of the target’s repository, yours or others’.

![A project’s Pull requests view: six PRs with their CI, approvals, and author](/images/en/prs.jpg)

Each row gives the sign of its CI, its number, its title, its marks (“Draft”, “fork marta”), its pills (“Conflict”, “Changes requested”, “1 approval”, “2 open threads”, “Your review”), its author, and its age. The list is read when it shows, then every minute while it is visible, and by “Refresh”.

Three filters, with their counts: “Mine”, “Review requested”, and “All”.

The hovered row offers:

- **“Get a review”**: see below.
- **“Start an agent”**: the PR’s branch is fetched, even from a fork, and an agent starts in a worktree on it. `Enter` or a double click do the same. Nothing is pushed or published. If an agent of the project already works there, the button becomes “Open the agent”.
- Opening it in the browser.

The list writes nothing on the host.

## Get a review from an agent

“Get a review”, or “Get a review from an agent” in the “PR and CI” tab, gives the diff of an open PR to an agent, which writes a text and comments by file and by line. The review runs in the background; you can leave the view.

![A review by an agent: its text, one comment kept and another to decide, under the lines of the diff](/images/en/pr-review.jpg)

On each comment: “Keep”, “Edit”, or “Drop”. Then you choose what the review says (“Comment”, “Request changes”, “Approve”) and “Publish on GitHub” sends what you kept in one go. Nothing leaves before that click, and no tool of the MCP server publishes for you.

### What the agent reads

- The diff alone, its title, its branches, and its unresolved comments. It opens no file: the review runs without any tool, in an empty folder. So it only sees what the diff shows.
- Files that are too long (150 KB of diff in all) and generated files aren’t read, and the view says so.

### Anyone can write a PR

On a public repository, anyone can open a PR, and its title, diff, and comments reach the agent as data to review, never as instructions: it has no tool to obey them with, but read what it proposes before you publish it.

### The limits

- The review is refused on Cursor CLI, which can’t turn its tools off, and on a plugin’s harness, where nothing checks what it allows: choose another harness in Settings › Agents › “Written texts”.
- The diff goes to that harness’s service, like everything you give an agent.
- 40 comments at most, on lines of the new version.

## Environments and deployments

On GitHub, GitLab, and Bitbucket Cloud, Escouade follows the repository’s deployments after the merge. A done ticket’s card says where it runs, and “Tickets · Environments”, in the Kanban’s header, shows a lane per environment, from “Merged, not deployed yet” to production.

![The Environments view: what runs in staging and production, and the tickets that reached them](/images/en/environments.jpg)

At the top of each lane: what runs there, its address, and a deployment under way. A failed last deployment borders the lane in red, with “See the job”. On a card “In review”, “Open preview” opens the PR’s preview (Vercel, Netlify, Cloudflare Pages…).

A ticket is followed for 30 days after it became “Done”. The environments are read with the tickets’ PRs: no webhook, the app has no public address. Rerunning or approving a deployment from Escouade doesn’t exist.

Gitea, Forgejo, and Azure DevOps have no deployments API: Escouade only follows the CI there.

## News from the code hosts

[Notifications](notifications#what-a-notification-says) tell what became true between two reads: CI failed, changes requested, PR approved, PR merged, conflict, review requested, deployment to production succeeded, deployment failed. Each one turns off in [Settings › Notifications](settings-notifications#pull-requests-and-deployments). A notification never says a PR’s title.

The [overview](overview#what-it-shows) has a “Pull requests” group, across all projects: “To fix”, “To merge”, and “Reviews requested”.

Escouade only learns this news by reading the host: every minute with the window in front and every 5 minutes otherwise for a ticket’s PR, only while it is shown for an ordinary agent’s PR.

## What your agents can read

With [the MCP server](mcp#pull-requests), an agent reads a project’s PRs through `list_pull_requests` and `get_pull_request`. These tools write nothing on the host, and they mark what a third party wrote: a PR’s title or comment is data, never an instruction.

## What stays with you

Nothing goes through an Escouade server: the calls go straight from the app to the host, with your token, kept on your machine. See [Privacy](/en/privacy/).
