# Remote control

Remote control makes an agent’s session reachable from claude.ai and the Claude mobile app, so you can answer it from a browser or your phone. This page explains how to turn it on, for one agent or for all, what shows in Escouade, how long the agent stays reachable, and how to turn it off.

![Escouade on the left, with the remote control icon on the card of the agent “refacto-auth”, and on the right the Claude mobile app showing the same conversation, named “demo-api · refacto-auth”](/images/en/remote.jpg)

## Turning on remote control

There are three ways to turn it on. The result is the same: the agent’s session is named `project · agent` on claude.ai and in the Claude app, for example `demo-api · refacto-auth` (for a ticket agent, the name comes from the ticket). Renaming the project renames the remote sessions of its agents; an interface in the [Claude Code view](claude-code-view) keeps the old name until its next launch.

- **For one agent**, with a right-click on its card.
- **For all the agents of a project**, with a project setting.
- **For all the agents of all projects**, with a global setting.

### For one agent

1. Right-click the agent’s card in the sidebar.
2. Choose “Turn on remote control” (“claude.ai, mobile”).

Escouade starts the agent’s process if it’s stopped and asks Claude Code to open the remote session. A message confirms: “refacto-auth is reachable from claude.ai and the Claude app”.

If Claude Code can’t open the session, an error message starting with “Remote control unavailable” appears, and the option stays off.

The menu of an archived agent doesn’t offer remote control. For an agent running in [the Claude Code view](claude-code-view), see [below](#in-the-claude-code-view).

### For all agents, or a project’s

Two switches, off to begin with, turn remote control on without going through each agent:

| Setting | Where | Agents covered |
|---|---|---|
| “Remote control for all agents” | Settings › Agents, “Remote control” group | Those of every project |
| “Remote control for this project’s agents” | Settings › Projects › Privacy, “Remote control” group (under “Identity”) | Those of the project chosen |

Like the other settings, they are saved with “Save” (see [Harness settings](settings-claude-code) and [project settings](settings-projects)).

Once on, a setting applies to:

- every agent of the project (or of all projects) that already exists and isn’t archived;
- every new agent, Kanban ticket agents and copies made with “Duplicate the conversation” included.

Escouade starts each agent’s process right away, at most 3 at a time, and it stays running: it starts with the app and is never stopped for inactivity. That keeps one `claude` process in memory for each agent, which the setting’s description says.

Once off, a setting takes remote control away from the agents that had it from that setting, and their remote session is closed. It touches neither the agents whose remote control you turned on by hand, nor those another setting still covers: if you turn off the global setting but a project’s is on, that project’s agents keep theirs, through the project.

The two settings never contradict each other. While “Remote control for all agents” is on, the project’s switch shows on and grayed out, with “On for all projects in Settings › Agents”. The project’s own choice is kept: once the global setting is off, the project’s switch goes back to its state.

## What shows in Escouade

A wave icon appears on the agent’s card, after its name. It says how the link is doing, and its tooltip details it:

| Icon | Start of the tooltip | Meaning |
|---|---|---|
| Blue | “Remote control: connected” | The session is open and connected. |
| Gray | “Remote control: connecting…” or “Remote control: waiting for a connection” | The link is being established. |

Each tooltip contains “(reachable from claude.ai and the Claude app)”. For an agent in the [Claude Code view](claude-code-view), the icon says the state of its interface (see [below](#the-icon-of-an-interface)).

The tooltip ends with what turned on the agent’s remote control, after a middle dot, for example “Remote control: connected (reachable from claude.ai and the Claude app) · turned on by hand”:

| Note | Meaning |
|---|---|
| “turned on by hand” | You turned it on with the right-click on the card. |
| “turned on by the project’s setting” | “Remote control for this project’s agents” is on. |
| “turned on by the setting for all agents” | “Remote control for all agents” is on. |

When the agent is reachable, the card’s menu changes:

| Entry | Effect |
|---|---|
| Turn off remote control | Cuts the link and sets the agent apart (see below). |
| Open on claude.ai | Opens the session in your browser. |
| Copy the claude.ai link | Copies the session’s address (“claude.ai link copied”), to open it on another device. |

The last two entries appear as soon as the session’s address is known. The right-click stays available whatever the source of the remote control.

What you send to the agent from claude.ai or the Claude app also shows in Escouade’s conversation, in a bubble like yours, with the note “from claude.ai” (its tooltip says: “Sent from claude.ai or the Claude app (remote control)”).

Claude, outside Escouade or inside an agent, can read this state through the [MCP server](mcp) but cannot change it: `list_agents` and `get_agent_summary` say, for each agent, whether remote control is in effect, what turned it on and the state of the link, and `list_projects` gives the two settings. No tool turns it on or off: that stays your decision, in the window.

## How long the agent stays reachable

- **As long as Escouade runs.** The agent stays reachable when you close the window: it hides and the app goes on in the notification area (see [agents](managing-agents#closing-the-window-quitting-and-restarting)). If you quit the app with “Quit”, the agents stop and are no longer reachable.
- **No stop for inactivity.** Escouade normally stops an idle agent’s process after a delay (30 minutes by default); an agent whose remote control is on keeps its process, to stay reachable.
- **After a restart.** Escouade restarts the agents whose remote control is on, by hand or from a setting, along with the app and reattaches their previous remote session: the link you had opened on your phone stays valid, and the session keeps its `project · agent` name. If that session is gone in the meantime, a new one is opened, with a new link. If the link fails at startup, a “Remote control unavailable:” notice followed by the cause appears in the agent’s conversation. That notice is only told once per agent and per launch of Escouade, until a link succeeds or you turn remote control back on by hand.

## Turning off remote control

Right-click the agent’s card, then “Turn off remote control”. Escouade closes the remote session, the icon goes away, and the agent becomes an ordinary agent again (which the idle stop can stop again), set apart (see below). Archiving or deleting an agent also turns off its remote control; restoring an archived agent gives it back if a setting covers it (not if you had turned it on by hand).

Turning off remote control, with the right-click, by turning off a setting, by archiving the agent or by deleting it, closes the remote session: it is archived on claude.ai.

If no process of the agent is running at that moment (just after it started, after a failed link, a dead process), Escouade can’t close it right away. It remembers the session: it closes it the next time a process of the agent starts, or takes it back, with the same link, if you turned remote control back on in the meantime. Turning it off and quickly back on doesn’t mix the requests: they go one after the other.

### Setting an agent apart

“Turn off remote control” always sets the agent apart, whether or not a setting (its project’s or the global one) covers it at that moment. A setting turned on afterwards doesn’t give it remote control back, and neither does a restart of Escouade. The agent stays off until you turn it back on by hand, with “Turn on remote control” in its menu.

A copy of the agent (“Duplicate the conversation”) is set apart too. The ticket of an agent that is set apart keeps that state: a ticket sent back to “To do” and started again creates an agent that is set apart too, unless you turned remote control back on by hand.

## In the Claude Code view

The [Claude Code view](claude-code-view) follows the same rules, and an agent can switch to that view while its remote control is on. Claude Code’s interface then holds the remote session, not Escouade.

- **Starting.** The Claude Code interface of an agent in remote control is started with `--remote-control=<project · agent>`, as a single argument (a name that starts with “-” is therefore not taken for an option). Claude Code then shows the session’s link. When the interface resumes its session (Escouade restarting, a relaunch), Claude Code reattaches the same remote session, with the same link. A closed interface archives its remote session.
- **When Escouade starts.** The interface of such an agent is started in the background, without you opening it, so that the agent is reachable. Opening the agent shows the interface already running. These starts are made at most 3 at a time, like those of a setting: a start keeps its place until the interface is ready, 15 seconds at most. If the agent’s remote control goes off before you have opened it, it stops, unless the agent is in the middle of a turn (its work would be lost).
- **A change while the interface is running.** Escouade can’t turn remote control on or off in an interface that is already open. Whether the change comes from a setting or from the right-click, it takes effect the next time its interface starts. After a right-click, a message says so: “Applies at the next launch of its Claude Code interface; until then, its /remote-control command does it.” For a setting, its description says so.
- **The card’s menu.** It offers “Turn on remote control” or “Turn off remote control”. “Open on claude.ai” and “Copy the claude.ai link” are offered too, once the interface is connected: the link comes from the interface.
- **Switching views.** Switching an agent to the Claude Code view closes the remote session of its Escouade view: its interface opens its own. Switching back to the Escouade view opens a new one, with a new link.
- **When Escouade closes the interface.** An interface that Escouade stops for good (back to the Escouade view, archiving, deleting, an interface never shown whose remote control goes off) is closed gently, so that Claude Code archives its session: Escouade sends two `Ctrl+C`, as you would, then stops the interface by force if it isn’t closed after 5 seconds. A start requested in the meantime waits for the old interface to be gone: a session never has two processes. When Escouade quits, it isn’t, so that the link comes back at the next start.
- **The session’s name.** Renaming the project doesn’t change the name of the session of an interface that is already running: it keeps the old name until its next launch.

### The icon of an interface

The icon’s tooltip says how the interface is doing:

| Tooltip | Meaning |
|---|---|
| “Remote control: no interface running, not reachable for now (open the agent to start it)” | No interface is running: the agent is not reachable. |
| “Remote control: connecting… (reachable from claude.ai and the Claude app)” | The interface is started with remote control and the link is being established. |
| “Remote control: connected (reachable from claude.ai and the Claude app)” | The session is connected. The icon is blue. |
| “Remote control: from the next launch of its Claude Code interface” | The open interface was started before the remote control was turned on. |
| “Remote control: not opened by Claude Code (no claude.ai subscription, network, proxy or organization policy)” | The interface is running, but Claude Code didn’t open the remote session (see [below](#when-claude-code-doesn-t-open-the-session)). The conversation says so once. |
| “Remote control still on in the open interface, until its next launch (its /remote-control command turns it off)” | Remote control is off for Escouade, but the open interface is still linked. The icon is crossed out, in the warning color, until the next launch. It only appears if the interface really had its link (“connected”): an interface started with the option but never linked has none. |

### When Claude Code doesn’t open the session

Claude Code doesn’t write in the terminal that remote control failed. Escouade works it out, in two ways, and the icon then says “not opened by Claude Code”:

- The interface’s follow-up (its transcript) contains a `bridge_status` entry without a link. The notice in the conversation is then “Remote control unavailable: Claude Code could not open the remote session”, followed, in parentheses, by what Claude Code said.
- Nothing proves a link 20 seconds after the interface is ready. The notice is then “Remote control unavailable: Claude Code’s interface did not open it (an account with no claude.ai subscription, a network, a proxy or a rule of the organization).”

The interface goes on without remote control. Each notice is only told once per agent and per launch of Escouade, and remote control stays on for the agent: the next launch of its interface tries again. If a link arrives later, the icon goes back to “connected”.

### When the interface closes right away

If an interface started with remote control, which you have never shown, closes within 10 seconds, a notice “Remote control unavailable: Claude Code’s interface closed right after it started with remote control; it is started again without it.” appears in the conversation. It is started again right away in the background, without the option, and the next launches of that agent are made without the option, until Escouade starts again or you turn remote control back on by hand.

An interface that you have seen and that closes within that time is never taken for a failure of the option: “No, exit” on the folder trust box, `Esc`, `/exit` or two `Ctrl+C` give neither a restart without the option nor a notice. If you want to start it again, the terminal’s “Restart” button is yours.

### A refused message

A message from the window or from the MCP server is refused as long as the interface is not ready: “The Claude Code interface of this agent is not ready yet (first launch or a folder to approve): open the agent to see it.” That is the case on an account’s first-launch screen, and when a folder has to be approved (the default choice there is “No, exit”: a typed text would pick it). Open the agent to see the interface.

An interface is ready as soon as one of these signs is there: you have shown it; its transcript contains a `bridge_status` or `bridge-session` entry; or Claude Code has listed it in its registry of sessions, which it does within 2 seconds once ready, and not on the first-launch screen nor on the trust box. That also goes for an interface started again without the option.
