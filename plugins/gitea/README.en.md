# Gitea / Forgejo

Gitea, Forgejo and Codeberg as Escouade's code hosts, one account per instance. The plugin opens the pull requests of your agents and tickets, follows their CI (statuses and Actions), reviews and threads, merges, and schedules auto-merge on a protected branch. It has no ticket source in Escouade: it serves pull requests and CI only.

It was written from Forgejo's and Gitea's API documents and tested against a fake server; it has not run on a real instance yet: start on a repository with nothing at stake.

## Who it is for

For you if your projects have their repository on Codeberg, gitea.com, or your own Gitea or Forgejo instance.

## What it needs

- An account on the instance, and an access token with the `write:repository`, `write:issue` and `read:user` scopes, made in your account's settings on the instance.
- One account per instance. Nothing to install on your machine.

## What it can do

- Reach `codeberg.org`, `gitea.com` and the instances you add.
- Receive from you the token of each of your accounts, which Escouade keeps in the system keychain.
- Read your repositories' pull requests, their statuses, Actions runs and jobs and their logs, reviews and comments; open a pull request, merge it or schedule its auto-merge; run failed jobs again; publish a review, reply in a thread and resolve it, where the instance allows it.
- Nothing else: it runs no program (Escouade pushes the branches), reads no file and receives nothing from your agents beyond what Escouade asks it to publish.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
