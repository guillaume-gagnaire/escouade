# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- `api.clickup.com`, over https, and no other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network is refused before it leaves.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer from ClickUp is capped at 8 MiB and 15 seconds.

## What it reads and writes on ClickUp

It reads, with your account:

- your name, your email and your user id (for the "Connected · …" row and the "My tasks" chip);
- your workspaces, their spaces, folders and lists that are not archived;
- the statuses of the linked list, and its open tasks: name, description, status, priority, assignees, tags, checklists, link, id and custom id.

It writes only:

- a task's status, when its ticket changes column and the status mapping gives one;
- a comment on a task, when Escouade asks it to, posted without notifying all its watchers.

It creates, renames, archives and deletes no task, list or space.

## The secrets it receives

- The personal API key of your ClickUp account, which you gave in Settings › Connected services. Escouade keeps it in the system keychain and lends it to the plugin when it asks, for a call; the plugin writes it nowhere.
- It goes in the `Authorization` header of each call to `api.clickup.com`, and there only.
- It can read no other secret: neither another plugin's, nor another Escouade account.

## What goes to ClickUp

- The reads above.
- The text of the comments Escouade has it post: the Escouade ticket key (`DEM-5`), its branch, its criteria and their state, the agent's notes, what was done, the outcome of the approval and the link of the pull request, depending on the column. They are written in the "Language of texts written by the agents" (Settings › General).

## What never goes there

- Your code, your files, your diffs, your agents' conversations.
- Tickets that do not come from ClickUp.
- Your key, to any host other than `api.clickup.com`.

## What the plugin cannot do

- Run a program or a command.
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than ClickUp.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: its tickets' sync stops and says so, and imported tickets stay. Its "Journal" tab shows what it did.
