# Bitbucket

Bitbucket Cloud (bitbucket.org) as Escouade's code host. The plugin opens the pull requests of your agents and tickets, follows their statuses, pipelines, comments and deployments, merges, and publishes the reviews you approve. Bitbucket has no ticket source in Escouade: it serves pull requests and CI only.

It was written from Bitbucket Cloud's API documentation and tested against a fake server; nobody has tried it on bitbucket.org yet: start on a repository with nothing at stake.

## Who it is for

For you if your projects have their repository on bitbucket.org. Bitbucket Data Center, whose API differs, is not supported.

## What it needs

- Your Atlassian account: its email, and an Atlassian API token with Bitbucket's scopes (read the account, the repositories and the pipelines, read and write pull requests).
- Nothing to install on your machine.

## What it can do

- Reach `api.bitbucket.org`, and no other host.
- Receive from you your account's token, which Escouade keeps in the system keychain.
- Read your repositories' pull requests, their statuses, pipelines and logs, comments and deployments; open a pull request, merge it; publish a review, reply under a comment, resolve a thread.
- Nothing else: it runs no program (Escouade pushes the branches), reads no file and receives nothing from your agents beyond what Escouade asks it to publish.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
