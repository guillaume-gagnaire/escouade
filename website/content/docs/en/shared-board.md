# Sharing the Kanban with the team

A shared ticket is a file of the repository that you commit with your work: your teammates find it on their Kanban at their pull, with your name, and its status follows everyone’s work.

## Share a ticket

Right-click the card, “Share with the team”, or the switch of the same name in the ticket’s form. Escouade then writes a file `.escouade/tickets/<id>.yml` in the project’s folder, and a small mark shows beside the ticket’s key; its tooltip gives the file.

- Escouade writes the file again at each change that matters: an agent takes the ticket, it moves to “To review”, it is approved, sent back or put back in “To do”, its title, description or criteria change. Never at each loop.
- **Escouade never commits or pushes this file.** It shows among the uncommitted files of the [Git panel](git), on “Whole project”, in a “Shared tickets” group: each file is named by its ticket’s key and title. Commit it with your work; the team gets it at their pull.
- These files block neither a merge nor a ticket’s approval: a folder where only the tickets’ files changed counts as clean.
- The ticket’s images are shared in a folder beside the file, `.escouade/tickets/<id>/`; its joined files too, up to 5 MB. A bigger joined file stays with you, and the form says so.
- “Stop sharing” deletes the file (the deletion is to be committed); the ticket stays with you as it is. Deleting a shared ticket deletes its file too.

In Settings › Projects › Team, the “Shared Kanban” group says how many tickets the project shares, and “Share every new ticket” shares from the start those you make, by hand, by an import or through MCP.

## Your name

Each file carries its author’s name, and the name of whoever took the ticket. Escouade asks for yours at the first launch, filled in from `git config user.name`; “Later” puts the question off until your first share.

- You change it in Settings › General › You, “Your name”.
- A project may have its own: Settings › Projects › Team › Shared Kanban, “Your name in this project”. Empty, it is the application’s.
- Your name stays on your machine: it is never written in [.escouade.yml](escouade-yml).
- Beside the name, an id made from your git e-mail (the e-mail itself is written nowhere) tells your tickets from a namesake’s. It is what says whether a ticket is yours; it is the same on your other machines.

## Someone else’s ticket

After your pull, a ticket Alice shared shows on your Kanban, in the column of its status, with her name and a disc of her initial on the card.

- In “To do”, the card says “Added by Alice · the autopilot never starts it”: your [autopilot](autopilot) never takes it, even at the top of the column. The “Start” button starts it by hand, autopilot on or off.
- Started on your machine, it behaves like one of yours (worktree, loops, approval), and its file says you took it. It keeps its author.
- Under way on someone else’s machine, the card says “Taken by Alice”; to review, “To review at Alice’s”; finished, “Done by Alice”. These tickets take no place on your Kanban and your Escouade never takes them up: only whoever took it approves it.
- Its menu also offers “Edit” (what you change goes into its file, for the whole team), “Hide for me” (the file stays; a band of the Kanban says how many tickets are hidden and shows them on demand) and “Delete for the team…” (deletes its file, to be committed).

Escouade reads the `.escouade/tickets/` folder again as soon as it changes: after a pull, another branch, an edit by hand. A ticket Alice finished moves to “Done” on your Kanban after your pull.

## The file’s format

```yaml
# A ticket of the Kanban shared by Escouade: https://escouade.dev/en/docs/shared-board
# Escouade writes it again at each status; commit it with your work, Escouade never does.
# yaml-language-server: $schema=https://escouade.dev/schemas/escouade-ticket.schema.json
version: 1
id: q4w7c2na
key: DEM-7
title: Login audit log
status: in-progress
author: { name: Alice Martin, id: a91f03c2 }
taken_by: { name: Bob Durand, id: 5d0e77b1 }
updated_by: { name: Bob Durand, id: 5d0e77b1 }
created: 2026-10-26T09:12:04+01:00
updated: 2026-10-26T15:02:40+01:00
after: [p2v8c4ma]
description: |-
  Each login attempt leaves a line: who, when, from where.
criteria:
  - Every login, successful or not, is logged
  - Tests pass
images:
  - { name: mockup.png, file: 3f2a9c01b7de.png }
files: []
external: null
outcome: null
```

| Key | What it says |
| --- | --- |
| `id` | The ticket’s stable id, its file’s name; never its key. |
| `key` | Its key, as its author’s Kanban gave it. |
| `status` | `todo`, `in-progress`, `review` or `done`. |
| `author`, `taken_by`, `updated_by` | Who shared it, who started it, who last changed the file: `{ name, id }`. |
| `after` | The `id`s of the shared tickets it waits for (“After”). |
| `images`, `files` | Its images and joined files, in the folder of its `id`. |
| `external` | The Jira, Trello, GitHub, Linear, ClickUp, GitLab or Azure Boards ticket it was imported from. |
| `outcome` | What its approval did, once done. |

Stay on your machine: the loops, the cost, the agent’s progress, the criteria met, the account, the model, the agent and its branch. The format’s JSON schema is published at `https://escouade.dev/schemas/escouade-ticket.schema.json`.

For a project of several repositories whose folder is not a repository itself, the `.escouade/tickets/` folder goes in the first repository.

## Keys

A shared ticket keeps everywhere the key its author gave it: Alice’s DEM-12 is DEM-12 on your Kanban. Your new tickets never take the key of a shared ticket you received. If Alice and you each made a DEM-12 before pulling, yours keeps its key and hers carries Alice’s name on its card; for the [MCP server](mcp), it is named `DEM-12@alice`, or by its id.

## When it gets stuck

- **Two people on the same ticket.** If the file you get says Alice took it while your agent works on it, your agent goes on and the card says so; your next status writes the file again.
- **A file in a git conflict.** Escouade does not read it: the card keeps its last state and offers “Keep mine” (Escouade writes the file again with your version, which settles the conflict) or “Take theirs” (the side merged in). Nothing is committed.
- **A file that can’t be read.** It becomes a “Shared ticket that can’t be read” card, with the line in cause; Escouade never writes it nor starts it.
- **A ticket its author no longer shares.** It leaves your Kanban at your pull, unless you had started it: it then stays with you, as one of yours.
- **An import already shared.** Importing a Jira, Trello, GitHub, Linear, ClickUp, GitLab or Azure Boards ticket a teammate already shared makes no duplicate: your import, if not started, becomes the shared ticket.
- **A private project.** It is shared too: “private” is about what Escouade’s other projects see, not your team on the repository. Its shared tickets stay out of sight of the other projects’ agents.

## With the MCP server

The ticket tools of the [MCP server](mcp) say whether a ticket is shared (`shared`: by whom, whether it is you, who took it, its file). `create_ticket` and `update_ticket` take `shared: true` to share, `false` to stop. An agent never starts a ticket someone else shared: `start_ticket` refuses it, and so does `delete_ticket`; only you start, hide or delete it, from the Kanban.
