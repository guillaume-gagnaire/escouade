# Several servers: the spaces

One Escouade can drive several servers, one for work and one of your own. Each machine is a **space**: this computer, and each server you [connected to](connect-server), with its projects, agents and settings.

Nothing crosses from one space to another. As long as you have one space, the window does not change.

## The spaces in the projects bar

- **The open space** heads its projects: its plate (a screen for this computer, a letter for a server) and its name.
- **Each other space** is a pile under the “+”: two of its projects as cards behind its plate, and a dot with the number of requests waiting there. A click takes you there. Its name turns amber when its connection is cut.
- Beyond three spaces, or in a window 1024 pixels wide, the piles pack into plain plates: the bar never scrolls for them.
- A right click on a space opens its menu: “Go to…”, “Open in a window”, “Retry” when it is cut, the space’s settings, “Remove… from this computer”.

Each server stays connected, even while you are on another space: its SSH tunnel and its connection come back by themselves after a cut, without touching the others.

## “Everywhere”: all that waits, at a glance

From two spaces on, the overview (`Ctrl+Shift+A`) opens on **“Everywhere”**: what waits in each space, under its machine (its name, its address, the round trip time), the open space first. Its tabs filter by space.

- **You answer there without changing space**: a permission you can read whole is granted with “Allow on Work”, and goes to the Work server.
- **The machine is written on each request and on its button**, so you never allow a command on the wrong server.
- A question, a plan, or a request too long to read there is answered in its conversation: “Open” takes you there.
- A space that is cut is amber, with the time of what you see; its requests are greyed, with no button, until it is back.

## Changing space

- A click on a pile, “Open” in “Everywhere”, or a notification.
- `Alt+1` to `Alt+9` (`⌃1` to `⌃9` on macOS): the nth space, this computer being the first. Ctrl and the key under Escape: the next space. `Ctrl+1` to `Ctrl+9` stay the projects’; a question on screen keeps `Alt+1` to `Alt+9` for its options.
- “Open in a window” puts a space in its own window, with its title bar and its machine. One window per space: asking again brings back the first.

## Notifications

A request from another server notifies you with its space’s name first: “Work · billing · unpaid-reminders”. This computer’s say “This computer”. A click takes you to the agent, in its space.

## A server’s devices

On a server, Settings › “Devices” lists the paired devices (this computer is marked), with when they were last seen. “Remove” revokes another device: its connections close. “Create a code” gives a 10-minute pairing code for a new device, without going to the server.

## On a managed computer

The [device policies](managed-policies) can:

- **`servers.personalSpace` set to `false`**: only add the servers of the allowed servers’ list (`servers.allowed`); this computer stays a space.
- **`servers.aggregateView` set to `false`**: “Everywhere” shows only the numbers of a work server, no text nor button; the details stay in its space.
- **`servers.contentlessNotifications` set to `true`**: a work server’s notifications say an agent is waiting, never what.

A work server is one of the `servers.allowed` list when it is set, else any server. These locks show in Settings › “Servers”, under “Imposed on this computer”.
