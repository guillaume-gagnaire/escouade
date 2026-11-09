# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- `api.bitbucket.org`, over https, and no other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network is refused before it leaves.
- A redirect to another host gets neither your token nor the call's body. A list's next page is only followed on the API.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer is capped at 8 MiB and 15 seconds.

## What it reads and writes on Bitbucket

It reads, with your account:

- your name and your account id (for the account's row);
- the pull requests of your projects' repositories: their state, participants and approvals, comments and threads, the statuses of their last commit, their pipelines' steps and logs, their deployments and those of the repository's environments.

It writes, when Escouade asks it to:

- a pull request (its title, text, branch, draft or not) and its reviewers;
- its merge;
- a review you approved (its comments, then the approval or the request for changes), a reply under a comment, a thread's resolution, a comment.

It pushes no code: `git`, run by Escouade, pushes the branches. It deletes no branch nor repository.

## The secrets it receives

- Your Atlassian account's API token, which you gave with its email. Escouade keeps it in the system keychain and lends it to the plugin when it asks, for a call; the plugin writes it nowhere.
- With your email, it makes the `Authorization` header of calls to `api.bitbucket.org`, and there only.
- It may read the token of its own accounts, never another plugin's.

## What goes to Bitbucket

- The reads above.
- What Escouade publishes on your behalf: a pull request's title and text, a review and its comments (what you kept, nothing before your click).

## What never goes there

- Your agents' conversations and your files. Your code only leaves through the branch `git` pushes, not through the plugin.
- Your token, to any host other than `api.bitbucket.org`.

## What the plugin cannot do

- Run a program or a command.
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than Bitbucket Cloud.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: pull requests are no longer followed (the agent's card says "Code host stopped: PRs no longer followed" and keeps the last reading), and nothing is lost. Its "Journal" tab shows what it did.
