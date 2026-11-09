# Use

## Import cards

In the Kanban, "Import" (⤓) opens the "Import tickets" window. The Trello tab shows the open cards of the linked board, across all its open lists.

- **Search** ("Search for a card…") keeps the cards whose name contains the text, ignoring case. A card number, `#12` or `12`, finds card `#12`.
- **Chips**: "My cards" keeps those you are a member of; then one chip per list of the board keeps the cards of that list (several chips: the cards of any of them).
- **Each row** shows the card number (`#12`), its name, the kind "Card", its list and its labels, and the criteria found ("✓ 2 criteria detected").

Tick the cards, pick "Max loops", then "Import". Each imported ticket keeps the Trello badge and the card number on its Kanban card: a click opens the card in Trello. Its description is the card's.

## Acceptance criteria

With "Extract the acceptance criteria" on (Settings › Connected services, "Sync" group), the plugin takes as criteria, in this order:

1. the items of the checklist named after the criteria ("Acceptance criteria", "Critères d'acceptation", "Definition of done", "Criteria", "AC", "DoD");
2. otherwise the items of all the card's checklists;
3. otherwise, in its description, the list items under a heading or a line that names the criteria, or failing that its task list items (`- [ ]`).

It keeps twenty at most. With no criterion found, the ticket gets Escouade's default criteria.

## The card follows its ticket

In Settings › Projects › Imported tickets, the "Status mapping" gives, for each Kanban column, the Trello list to move the card to. Picking the board fills it in: the list that looks like "in progress" for "In progress" and "To test", the one that looks like "to test" for "Done", nothing for "To do".

When an imported ticket reaches a column:

- the card moves to the chosen list, if "Update the external status" is on;
- Escouade writes a comment on it if the column is ticked under "Comment" (by default "To test" and "Done"): the ticket is taken, ready to test with its criteria, or done with the link of its pull request.

With "Post a summary on every loop", each new loop of an agent also leaves a comment: the criteria met and the agent's notes.

## Automatic import

Settings › Connected services › "Automatic import" imports on its own the open cards that carry the chosen label (`claude-ready` by default). For Trello, it is a label of the card, whose name must be the same, case aside.

## Limits

- One Trello account per Escouade, and one board per project.
- The list of boards is that of your open boards. An archived board is not in it, nor an archived list among the statuses.
- The board is read at once: all its open cards and their checklists, with no pages. An answer from Trello cannot exceed 8 MiB nor take longer than 15 seconds.
- Escouade does not create a card, change it (title, description, members) or delete it: it moves it and comments on it, nothing more.
