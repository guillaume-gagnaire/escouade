# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- `dev.azure.com` and its subdomains (`*.dev.azure.com`: Azure's profile and identities hosts), `*.visualstudio.com`, over https.
- The instance of each of your accounts, the one you typed (an Azure DevOps Server): that host alone, on its port, none of its subdomains. It may be on your local network: you chose it.
- No other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network (apart from the instance you typed) is refused before it leaves.
- A redirect to another host gets neither your token nor the call's body.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer is capped at 8 MiB and 15 seconds.

## What it reads and writes on Azure DevOps

It reads, with your account:

- your identity in the organization (for the account's row, to vote and to set the auto-complete);
- the pull requests of your projects' repositories: their state, reviewers and votes, conflicts, comment threads, the evaluations of their branch's policies, their statuses, builds and the logs of their tasks; the target branch's merge policies;
- for Azure Boards: the organization's projects, their work item types and states, and the linked project's open items (title, description, acceptance criteria, a bug's repro steps, type, state, assignee, author, tags).

It writes, when Escouade asks it to:

- a pull request (its title, description, branch, draft or not), its reviewers, its labels and the work item it closes;
- its merge, or its auto-complete;
- retrying a build;
- a review you approved and your vote, a reply in a thread, its resolution, a comment;
- for Azure Boards: a work item's state (`System.State`), a comment.

It pushes no code: `git`, run by Escouade, pushes the branches. It deletes no branch, repository or work item.

## The secrets it receives

- The token of each of your accounts. Escouade keeps it in the system keychain (or asks `az` for it, for an account that goes through the CLI) and lends it to the plugin when it asks, for a call; the plugin writes it nowhere.
- It goes in the `Authorization` header of calls to the account's organization, and there only: as Basic for a personal token, as Bearer for an `az` Entra ID token.
- It may read the tokens of its own accounts, never another plugin's.

## What goes to Azure DevOps

- The reads above.
- What Escouade publishes on your behalf: a pull request's title and description, a review and its comments (what you kept, nothing before your click), a work item's sync comments (the Escouade ticket key, its branch, its criteria, the agent's notes, what was done, the link of the pull request).

## What never goes there

- Your agents' conversations and your files. Your code only leaves through the branch `git` pushes, not through the plugin.
- Your token, to any host other than its organization's.

## What the plugin cannot do

- Run a program or a command (neither `git` nor `az`).
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than Azure DevOps.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: pull requests are no longer followed (the agent's card says "Code host stopped: PRs no longer followed" and keeps the last reading), work items are no longer synced, and nothing is lost. Its "Journal" tab shows what it did.
