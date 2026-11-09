# GitLab

GitLab in Escouade, on gitlab.com and your self-managed instances, one account per instance. The plugin opens the merge requests (MRs) of your agents and tickets, follows their pipelines, discussions and deployments, merges when it is green, and publishes the reviews you approve. On the same account, it also serves GitLab Issues as a Kanban source: a project's issues imported as tickets, closed, reopened or labelled as their ticket moves.

It was written from GitLab's documentation and tried against answers recorded from gitlab.com, not yet against a real instance with a token: start on a project with nothing at stake.

## Who it is for

For you if your projects have their repository on gitlab.com or on a GitLab instance.

## What it needs

- A GitLab account, and either a personal access token with the `api` scope, or the `glab` CLI installed and logged in (`glab auth login`), whose token Escouade takes.
- One account per instance.

## What it can do

- Reach `gitlab.com` and the GitLab instances you add.
- Receive from you the token of each of your GitLab accounts, which Escouade keeps in the system keychain.
- Read your projects, their merge requests, pipelines, jobs, approvals, discussions, deployments and issues; open a merge request, merge it or ask for its auto-merge; run failed jobs again; publish a review, reply in a discussion, resolve it; comment on, close, reopen or label an issue.
- Nothing else: it runs no program (neither `git` nor `glab`: Escouade pushes the branches and asks `glab` for its token), reads no file and receives nothing from your agents beyond what Escouade asks it to publish.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
