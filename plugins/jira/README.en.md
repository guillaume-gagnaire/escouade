# Jira

The issues of a Jira Cloud project in Escouade's Kanban, its statuses as columns. You link a Jira project to an Escouade project, import its open issues, and each Jira issue follows its ticket: when the ticket changes column, Jira goes through the transition that leads to the chosen status, and Escouade can leave a comment on it.

## Who it is for

For you if your team tracks its work in Jira Cloud (a `….atlassian.net` site) and you want to hand those issues to agents without copying them.

## What it needs

- Your Jira Cloud site, your Atlassian account's email and an Atlassian API token, made on your Atlassian account's security page.
- Nothing to install on your machine.

## What it can do

- Reach the `*.atlassian.net` sites, and the site you give it if it is elsewhere; no other host.
- Receive from you your account's API token, which Escouade keeps in the system keychain. The site and the email are not secret: Escouade gives them to it with the account.
- Read your Jira projects, their statuses and their open issues; take an issue through a transition of its workflow; comment on an issue.
- Nothing else: it runs no program, reads no file and receives nothing of what your agents do, apart from the text of the comments Escouade asks it to post.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
