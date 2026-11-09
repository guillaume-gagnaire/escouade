# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- `gitlab.com` (and its subdomains), over https.
- The instance of each of your accounts, the one you typed: that host alone, on its port, none of its subdomains. It may be on your local network: you chose it.
- No other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network (apart from the instance you typed) is refused before it leaves.
- A redirect to a host other than the instance gets neither your token nor the call's body. A list's next page is only followed on the instance's API.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer is capped at 8 MiB and 15 seconds.

## What it reads and writes on GitLab

It reads, with your account:

- your login (for the account's row and the "Assigned to me" chip);
- the merge requests of your repositories' projects: their state, approvals, reviewers, discussions, pipeline and its jobs, a job's log, their deployments; the project's merge settings; the path of the fork an MR comes from;
- for GitLab Issues: the projects you are a member of, their labels, the labels of their boards' lists, and their open issues.

It writes, when Escouade asks it to:

- a merge request (its title, description, branch, draft or not), its reviewers and its labels;
- its merge, or the request for auto-merge;
- rerunning a pipeline's failed jobs;
- a review you approved, its approval, a reply in a discussion, its resolution, a comment;
- for an issue: a note, closing or reopening it, its labels from the status mapping.

It pushes no code: `git`, run by Escouade, pushes the branches. It deletes no branch, project or issue.

## The secrets it receives

- The token of each of your GitLab accounts. Escouade keeps it in the system keychain (or asks `glab` for it, for an account that goes through the CLI) and lends it to the plugin when it asks, for a call; the plugin writes it nowhere.
- It goes with the calls to the API of the account's instance, and there only.
- It may read the tokens of its own GitLab accounts, never another plugin's.

## What goes to GitLab

- The reads above.
- What Escouade publishes on your behalf: an MR's title and description, a review and its comments (what you kept, nothing before your click), an issue's sync notes (the Escouade ticket key, its branch, its criteria, the agent's notes, what was done, the link of the MR).

## What never goes there

- Your agents' conversations and your files. Your code only leaves through the branch `git` pushes, not through the plugin.
- Your token, to any host other than its instance's API.

## What the plugin cannot do

- Run a program or a command (neither `git` nor `glab`).
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than your GitLab instances.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: merge requests are no longer followed (the agent's card says "Code host stopped: MRs no longer followed" and keeps the last reading), issues are no longer synced, and nothing is lost. Its "Journal" tab shows what it did.
