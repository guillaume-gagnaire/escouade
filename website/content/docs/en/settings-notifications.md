# Settings: Notifications

The “Notifications” page sets Escouade’s alerts: which events it tells you about, whether it shows a system notification when the window is not in the foreground, and whether it plays a sound.

## How Escouade alerts you

Three signals combine, and the page controls the last two. The alerts themselves are described in [notifications](notifications).

- Window signals: the project’s tile, the agent’s card, and the taskbar (the Dock on macOS) always flag the agent, whatever the settings. The taskbar flashes when the window is not in the foreground.
- The chime: played for a type of event that is on, when “Sound on” is on too, whether or not the window is in the foreground.
- The system notification: shown for a type of event that is on, when the switch in the “Channels” group is on and the window is not in the foreground, meaning hidden, minimized, or without focus.

## Notify me about

Four switches choose the types of events that chime and show a system notification. A type that is off does neither; the window signals stay.

| Setting | Notifies when | Default |
|---|---|---|
| “Questions and permissions” | An agent asks a question or waits for a permission. | On |
| “Finished tasks” | An agent has finished its turn. Not when you interrupted the turn, nor when a message is waiting in the agent’s queue. | On |
| “Errors” | An agent stops on an error. | On |
| “Tickets (ready to review, blocked)” | A Kanban ticket is ready to review, or blocked. | On |

> [!NOTE]
> An agent whose ticket is “In progress” only notifies for its questions and permissions. For the rest, the [Kanban](board) does the alerting, through the “Tickets” type, when the ticket is ready to review or blocked.

### Pull requests and deployments

Eight more switches, all on by default, for what a [code host](code-hosts#news-from-the-code-hosts) has that’s new: “CI failed”, “Changes requested”, “PR approved”, “PR merged”, “Conflict”, “Review requested”, “Deployment to production succeeded”, and “Deployment failed”. Escouade reports them when it reads what it follows again. A type turned off says nothing more.

## Channels

| Setting | Effect | Default |
|---|---|---|
| “Windows notifications when the app is not in the foreground” | Shows a system notification when an event covered by “Notify me about” happens and the window is not in the foreground. The label names the system: “macOS notifications …” on macOS, “Windows notifications …” everywhere else. | On |

The notification’s title is “project · agent” and its body is the agent’s question, reply, or error. For a ticket, the title is the project and the body says, for example, “ATL-42 ready to review”. On Windows, clicking it brings the window back and shows the agent or the Kanban concerned. On macOS, it only brings the app to the foreground. On Linux, the switch is shown (labeled “Windows”), but Escouade doesn’t show system notifications: the sound and the window signals remain.

## Sound

| Setting | Effect | Default |
|---|---|---|
| “Sound on” | Plays a chime for the events covered by “Notify me about” (“When Claude asks a question or a turn ends”). The same chime for every type. On Linux, it’s played by `paplay`, which must be installed. | On |
| “Test the sound” | The “▶ Test” button plays the chime right away. | — |

The ♪ button in the status bar (tooltip “Notification sound”, text “On” or “Off”) flips the same setting and saves it right away, without opening the settings window.
