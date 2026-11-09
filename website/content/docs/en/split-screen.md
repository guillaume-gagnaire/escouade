# Split screen

The work area splits into several panes, side by side or one above the other, to see at once an agent and a terminal, two agents, or the editor beside a conversation. This page explains how to open a pane, which one takes the keyboard and what is kept.

## Open a pane

What goes in a pane: an agent, a terminal, a launch log or the editor. The board, the pull requests and Docs take the whole area, as before; leave them and your panes are back.

| To… | Do this |
|---|---|
| Open beside by dragging | Drag an agent’s card, a terminal’s or a launch’s row from the sidebar, or the title bar’s “Editor” button, onto the work area. A compass of five cells opens on the pane under the pointer: top, left, right and bottom open beside in that direction, the middle replaces. The cell under the pointer lights up and the place the pane will take is drawn. |
| Cancel the gesture | `Esc`, or drop outside the work area. |
| Open beside without dragging | `Ctrl+click` a card or a row; or right-click it, then “Open beside” or “Open below”. |
| Split at once | `Ctrl+\`: what is shown and the next agent (or a terminal) side by side. The same shortcut goes back to one pane, the active one. |

There are four panes at most. A crossed-out cell on the compass means there is no room: each pane keeps at least 320 pixels of width and 200 of height. The middle always replaces. A content is never shown twice: dropped on another pane it moves there, and in the middle the two panes swap.

## The active pane

One pane is active: the one you clicked last. It is ringed in the project’s color, and its card or row is highlighted in the sidebar; those of the other panes are only outlined.

The active pane takes:

- your typing, and dictation when no field has the focus;
- the keys that answer questions and permissions (`Ctrl+Enter`, `Alt+1` … `Alt+9`): those of another pane are answered with the mouse, or once its pane is active;
- `Ctrl+Tab`, the files panel (`Ctrl+Shift+B`), the editor and the status bar.

A click in the sidebar replaces the active pane’s content. An agent shown in any pane is seen: it does not blink.

## Size and direction

The handle between two panes is dragged with the mouse; when it has the focus, the arrows move it (faster with `Shift`) and `Enter` goes back to half and half, as a double click does.

| Shortcut | Action |
|---|---|
| `Ctrl+\` | Split, or go back to one pane. On a French AZERTY keyboard it is the `*` key: `Ctrl+*`. |
| `Ctrl+Shift+\` | Turn the active pane’s split: side by side or one above the other. |
| `Ctrl+Alt+←` `→` `↑` `↓` | Go to the pane on that side. |
| `Ctrl+Shift+W` | Close the active pane. The `×` at the top right of a pane, on hover, closes it too. |

These shortcuts also work while a terminal has the focus.

## What is kept

Each project’s layout, its direction, the panes’ sizes, their contents and the active pane, comes back when you return to the project and at Escouade’s next start. A terminal does not come back, since it is stopped on closing: its pane closes. An agent archived or deleted, a terminal closed leave their pane, and the others take the room; with one pane left, it fills the whole area.

The “Side by side” of a conversation and its files (`Ctrl+Shift+L`) stays as it is when one content is shown; in a pane, the files open over the conversation.
