# Permissions

This plugin is a WASM module: it runs inside Escouade, isolated, with no program of its own on your machine. What it can do is what its manifest asks for and what Escouade's host allows it, nothing more.

## What it reaches

- `api.trello.com`, over https, and no other host. Escouade's host enforces it: a call to another host, to an IP address written as such, or to an address of your machine or local network is refused before it leaves.
- It goes through the proxy of Settings › Advanced › Network when you have one.
- Each answer from Trello is capped at 8 MiB and 15 seconds.

## What it reads and writes on Trello

It reads, with your account:

- your username and name (for the "Connected · @…" row and the "My cards" chip);
- your open boards, their open lists, their open cards (name, description, list, number, link, labels, members) and their checklists.

It writes only:

- a card's list, when its ticket changes column and the status mapping gives one;
- a comment on a card, when Escouade asks it to.

It creates, renames, archives and deletes no card, list or board.

## The secrets it receives

- The API key and the token of your Trello account, which you gave in Settings › Connected services. Escouade keeps them in the system keychain and lends them to the plugin when it asks, for a call; the plugin writes them nowhere.
- Trello wants them in the address of each call: so they go to `api.trello.com`, and there only. The plugin's journal records a call's method and host, never its full address.
- It can read no other secret: neither another plugin's, nor another Escouade account.

## What goes to Trello

- The reads above.
- The text of the comments Escouade has it post: the Escouade ticket key (`DEM-5`), its branch, its criteria and their state, the agent's notes, what was done, the outcome of the approval and the link of the pull request, depending on the column. They are written in the "Language of texts written by the agents" (Settings › General).

## What never goes there

- Your code, your files, your diffs, your agents' conversations.
- Tickets that do not come from Trello.
- Your token, to any host other than `api.trello.com`.

## What the plugin cannot do

- Run a program or a command.
- Read or write a file on your machine.
- Receive Escouade's events (agents, tickets, projects): it only knows the calls Escouade makes to it.
- Reach a service other than Trello.

## Switching it off

Its page, in the Plugins view, has a switch for the whole app and a "Projects" tab. Switched off, it receives nothing: its tickets' sync stops and says so, and imported tickets stay. Its "Journal" tab shows what it did.
