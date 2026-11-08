# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- `codeberg.org` and `gitea.com` (and their subdomains), over https.
- The instance of each of your accounts, the one you typed: that host alone, on its port, none of its subdomains. It may be on your local network: you chose it.
- No other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network (apart from the instance you typed) is refused before it leaves.
- A redirect to a host other than the instance gets neither your token nor the call's body.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer is capped at 8 MiB and 15 seconds.

## What it reads and writes on the instance

It reads, with your account:

- your login (for the account's row), and the instance's version;
- the pull requests of your projects' repositories: their state, reviews and their lines, comments, the combined statuses of their last commit, Actions runs and jobs and their logs; the repository's merge settings, its labels, and whether a branch is protected.

It writes, when Escouade asks it to:

- a pull request (its title, text, branch), its reviewers and its labels;
- its merge, or its auto-merge on a protected branch;
- rerunning the failed jobs of an Actions run (Gitea 1.26 and later);
- a review you approved, a comment, a reply in a thread and its resolution (Gitea 1.26 and later).

It pushes no code: `git`, run by Escouade, pushes the branches. It deletes no branch nor repository.

## The secrets it receives

- The token of each of your accounts. Escouade keeps it in the system keychain and lends it to the plugin when it asks, for a call; the plugin writes it nowhere.
- It goes with the calls to the API of the account's instance, and there only.
- It may read the tokens of its own accounts, never another plugin's.

## What goes to the instance

- The reads above.
- What Escouade publishes on your behalf: a pull request's title and text, a review and its comments (what you kept, nothing before your click).

## What never goes there

- Your agents' conversations and your files. Your code only leaves through the branch `git` pushes, not through the plugin.
- Your token, to any host other than its instance's API.

## What the plugin cannot do

- Run a program or a command.
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than your Gitea or Forgejo instances.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: pull requests are no longer followed (the agent's card says "Code host stopped: PRs no longer followed" and keeps the last reading), and nothing is lost. Its "Journal" tab shows what it did.
