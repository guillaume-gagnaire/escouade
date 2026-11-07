# Azure DevOps

Azure DevOps in Escouade, on your organization's account (dev.azure.com, or an Azure DevOps Server collection). The plugin serves two things: the code host (the Azure Repos pull requests of your agents and tickets, their branch policies, pipelines, reviews and merge) and the Azure Boards Kanban source (a project's work items imported as tickets, whose state changes as their ticket moves).

It was written from Microsoft's documentation and tested against a fake server; it has not run on a real organization yet: start on a project with nothing at stake.

## Who it is for

For you if your projects have their repository in Azure Repos, or if your team tracks its work in Azure Boards.

## What it needs

- An account of your organization, and either a personal access token (PAT) with the Code (read and write), Build (read and execute) and Work Items (read and write) scopes, or, on dev.azure.com, the `az` CLI installed and logged in (`az login`), whose token Escouade takes.
- One account per organization or collection.

## What it can do

- Reach `dev.azure.com` and its subdomains, `*.visualstudio.com`, and the Azure DevOps Server you add.
- Receive from you the token of each of your accounts, which Escouade keeps in the system keychain.
- Read your repositories' pull requests, their policies, statuses, builds and logs, comment threads; open a pull request, merge it or set its auto-complete; retry a build; publish a review and its vote, reply in a thread and resolve it. For Azure Boards: read the projects, their types and states, their work items; change an item's state; comment on it.
- Nothing else: it runs no program (neither `git` nor `az`: Escouade pushes the branches and asks `az` for its token), reads no file and receives nothing from your agents beyond what Escouade asks it to publish.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
