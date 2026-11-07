# GitHub

GitHub in Escouade, on one account per instance (github.com, GHE.com or a GitHub Enterprise Server). The plugin opens the pull requests of your agents and tickets, follows their CI, reviews and deployments, merges when it is green, and publishes the reviews you approve. On the github.com account, it also serves GitHub Issues as a Kanban source: a repository's issues imported as tickets, closed, reopened or labelled as their ticket moves.

## Who it is for

For you if your projects have their repository on GitHub.

## What it needs

- A GitHub account, and either a personal access token, or the `gh` CLI installed and logged in (`gh auth login`), whose token Escouade takes.
- One account per instance: github.com, a GHE.com instance, or your GitHub Enterprise Server.

## What it can do

- Reach GitHub's API (`api.github.com`, `api.<name>.ghe.com`, or your Enterprise Server), and the hosts where GitHub stores Actions logs, which it reaches without your token.
- Receive from you the token of each of your GitHub accounts, which Escouade keeps in the system keychain.
- Read your repositories, their pull requests, checks, reviews, deployments and issues; open a pull request, merge it or ask for its auto-merge; run failed jobs again; publish a review or a reply; comment on, close, reopen or label an issue.
- Nothing else: it runs no program (neither `git` nor `gh`: Escouade pushes the branches and asks `gh` for its token), reads no file and receives nothing from your agents beyond what Escouade asks it to publish.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
