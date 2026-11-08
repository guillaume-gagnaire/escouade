# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- `api.github.com` and the GHE.com APIs (`*.ghe.com`), over https.
- The instance of each of your accounts, the one you typed (a GitHub Enterprise Server): that host alone, on its port, none of its subdomains.
- The hosts where GitHub stores an Actions job's log: `*.actions.githubusercontent.com` and `*.blob.core.windows.net`. GitHub sends there by a redirect; the plugin follows it **without your token**, which never leaves GitHub's API.
- No other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network is refused before it leaves. Only the instance you typed escapes that last check, since you chose it.
- A redirect to a host other than the API gets neither your token nor the call's body. A list's next page is only followed on the API itself.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer is capped at 8 MiB and 15 seconds.

## What it reads and writes on GitHub

It reads, with your account:

- your login, and a classic token's scopes (for the account's row);
- the pull requests of your projects' repositories: their state, reviewers, reviews, comments and threads, checks and statuses, an Actions job's log, their deployments and those of the repository's environments; the repository's merge settings;
- for GitHub Issues: your repositories, their labels and their open issues.

It writes, when Escouade asks it to:

- a pull request (its title, text, branch, draft or not), its reviewers and its labels;
- its merge, or the request for auto-merge;
- rerunning the failed jobs of an Actions run;
- a review you approved, a reply under a comment, a comment;
- for an issue: a comment, closing or reopening it, its labels from the status mapping.

It pushes no code: `git`, run by Escouade, pushes the branches. It deletes no branch, repository or issue.

## The secrets it receives

- The token of each of your GitHub accounts. Escouade keeps it in the system keychain (or asks `gh` for it, for an account that goes through the CLI) and lends it to the plugin when it asks, for a call; the plugin writes it nowhere.
- It goes in the `Authorization` header of calls to the API of the account's instance, and there only.
- It may read the tokens of its own GitHub accounts, never another plugin's.

## What goes to GitHub

- The reads above.
- What Escouade publishes on your behalf: a pull request's title and text, a review and its comments (what you kept, nothing before your click), an issue's sync comments (the Escouade ticket key, its branch, its criteria, the agent's notes, what was done, the link of the pull request).

## What never goes there

- Your agents' conversations and your files. Your code only leaves through the branch `git` pushes, not through the plugin.
- Your token, to any host other than its instance's API.

## What the plugin cannot do

- Run a program or a command (neither `git` nor `gh`).
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than GitHub and the storage of its logs.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: pull requests are no longer followed (the agent's card says "Code host stopped: PRs no longer followed" and keeps the last reading), issues are no longer synced, and nothing is lost. Its "Journal" tab shows what it did.
