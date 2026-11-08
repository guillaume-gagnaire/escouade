# Trello

The cards of a Trello board in Escouade's Kanban, its lists as columns. You link a board to a project, import its open cards as tickets, and each card follows its ticket: when the ticket changes column, the card moves to another list, and Escouade can leave a comment on it.

## Who it is for

For you if your team keeps its work in Trello and you want to hand those cards to agents without copying them.

## What it needs

- A Trello account, with an API key and a token for that key (read and write). Both are made on Trello, in two clicks, from Settings › Connected services.
- Nothing to install on your machine.

## What it can do

- Reach `api.trello.com`, and no other host.
- Receive from you the key and the token of your Trello account, which Escouade keeps in the system keychain.
- Read your open boards, their lists, their open cards and their checklists; move a card to another list; comment on a card.
- Nothing else: it runs no program, reads no file and receives nothing of what your agents do, apart from the text of the comments Escouade asks it to post.

## Learn more

Its documentation has five tabs: **Install** (connect it step by step), **Use** (what it does, screen by screen), **Permissions** (what it reads, writes and sends), **Troubleshooting** (its messages) and **Develop** (for whoever wants to read or change its code).
