# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- The `*.atlassian.net` sites, over https.
- Your Jira account's site, the one you typed, if it is not under `atlassian.net`: that host alone, on its port, none of its subdomains.
- No other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network is refused before it leaves. Only the site you typed escapes that last check, since you chose it.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer from Jira is capped at 8 MiB and 15 seconds.

## What it reads and writes on Jira

It reads, with your account, through Jira Cloud's REST API v3:

- your email, your name and your account id (for the "Connected · …" row and the "Assigned to me" chip);
- your site's projects (key and name), and the statuses of the linked project's issue types;
- the issues of the linked project you look for: key, title, description, status, type, priority, assignee and labels; the approximate number of issues of a search;
- an issue's status and its workflow's transitions, before moving it along.

It writes only:

- a workflow transition, when its ticket changes column and the status mapping gives a status;
- a comment on an issue, when Escouade asks it to.

It creates, changes, assigns and deletes no issue nor project.

## The secrets it receives

- The API token of your Atlassian account, which you gave in Settings › Connected services. Escouade keeps it in the system keychain and lends it to the plugin when it asks, for a call; the plugin writes it nowhere.
- With your email, it makes the `Authorization` header (Basic) of each call to your site, and there only.
- The site and the email are not secrets: Escouade gives them to the plugin with the account.
- It can read no other secret: neither another plugin's, nor another Escouade account.

## What goes to Jira

- The reads above, and the searches you type in the import window (as JQL).
- The text of the comments Escouade has it post: the Escouade ticket key (`DEM-5`), its branch, its criteria and their state, the agent's notes, what was done, the outcome of the approval and the link of the pull request, depending on the column. They are written in the "Language of texts written by the agents" (Settings › General).

## What never goes there

- Your code, your files, your diffs, your agents' conversations.
- Tickets that do not come from Jira.
- Your token, to any host other than your site.

## What the plugin cannot do

- Run a program or a command.
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than Jira.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: its tickets' sync stops and says so, and imported tickets stay. Its "Journal" tab shows what it did.
