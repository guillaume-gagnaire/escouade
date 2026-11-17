# The “Your turn” box

The “Your turn” box gathers what your agents need from you without stopping work: a choice between renders, a go-ahead, an answer, something only you can try. This page says where to find it, how to answer from the keyboard, compare renders, read an attached PDF and take back an answer.

![The “Your turn” box: the queue on the left, a choice between three renders, its PDF read beside the text and the answer at the bottom](/images/en/inbox.jpg)

## Where to find it

The box comes with Escouade’s MCP server: turn on “Agents can drive Escouade” in Settings › Agents › Drive Escouade (see [the MCP server](mcp)). Its agents leave their requests with the `ask_user` tool, then go on working.

- **Its tile** is in the projects’ bar, under “Overview”, with the count of everything that waits for you, every project. The pill is amber when an agent has stopped on a question or a permission.
- **`Ctrl+Shift+J`** opens and closes it; `Esc` closes it too. `Ctrl+J` still goes to the next stopped agent.
- **Server off**, nothing of the box shows: no tile, no count, no shortcut. Turning the server off with requests open asks first: they are kept, without you seeing them or their agents reading them, until you turn it on again, and they expire at their date. On again, the tile comes back with them, without a restart.

The tile’s count and the status bar’s say the same thing: “N waiting” counts the stopped agents, and the tile counts those same agents plus the open requests. A request put off for later only counts once it comes back.

## The queue

On the left, the queue: “All” or “This project”, the meter (a cell per request, green once handled, amber for a stopped agent) and “3 handled today · 9 left”. Below, the groups:

1. **“Your agents have stopped”**: the agents waiting for your answer in their conversation (“Open the conversation” leads there).
2. **“Today”**: the urgent requests, those back from “Later”, those a ticket waits for or left today.
3. **“Earlier”**, then **“Later”** (folded), with the time each comes back.
4. **“Recently answered”**: the answers of the last 7 days (50 at most), with who answered, when, and whether the agent read them.

Each row says who asks (the project and the agent), what, since when, its files, and shows the renders of a choice under its title. A dot marks a request you haven’t opened yet.

## Answering

On the right, one request at a time, in full: who asks, its text, its files, the agent’s recommendation and what your answer unblocks. The answer is at the bottom.

| Kind | What you do |
|---|---|
| A choice | Pick an option (its renders are on their plates), with a word if you like, then “Choose B”. The recommendation is set forward, never chosen for you: the answer starts on the recommended option, which you change with a click or a key. |
| Yes or no | “Yes” or “No”, in the agent’s words (“Yes, publish”). A go-ahead is yours alone: your “yes” goes back to the agent, which then acts with its own safeguards. |
| An answer | Type (or dictate) your sentence; the field starts with the text the agent suggests. `Ctrl+Enter` sends it. |
| A task | Tick its steps, attach a screenshot if something is off, then “Done” or “I won’t do it”. |

The microphone dictates into the field, as the conversation’s does (see [dictation](settings-voice)). The paperclip attaches a file to your answer.

### From the keyboard

| Key | What it does |
|---|---|
| `J`, `K` | The next request, the previous one (“3 of 9”) |
| `1` to `6` | An option of the choice |
| `Enter` | The main gesture: “Choose B”, “Yes”, “Done” |
| `L` | “Later” |
| `↑`, `↓` | From one row of the queue to the next |

An answer sent opens the next request.

### Compare full size

“Compare full size” (or a plate’s icon) shows the options side by side across the window, the recommendation as a seal on its plate. When the agent attached a dark and a light render, “Dark · Light · Both” picks which to show. `1`, `2`, `3` choose, `←` and `→` move from one option to the next, `Space` shows the plate alone in the viewer, `Esc` closes. The “A word for the agent” field and “Choose B” stay at the bottom: you answer without going back.

![Comparing full size: three directions, dark and light, the recommendation’s seal on the second](/images/en/inbox-compare.jpg)

## Attached files

An agent’s file is data: Escouade reads its copy, never a path the agent gives, and runs nothing.

- **An image** (PNG, JPEG, GIF, WebP, AVIF): its thumbnail, then the viewer (the request’s gallery, zoom, “Copy image”, “Save as…”).
- **A PDF**: “Read” opens it beside the text, page by page (“3 of 12”, a page typed leads there), with its thumbnails, zoom, search and “Full screen”. It is drawn by pdf.js, shipped with the app: no script of the PDF runs, nothing loads from the web, its links are not live. A PDF protected by a password doesn’t show.
- **A Markdown file** is drawn under the request, as in the conversation: nothing remote without a click.
- **The others** (a Word document, a spreadsheet, an archive…): their name, their size, “Open” (in the system’s app) and “Save as…”. A program, a script or a page is never opened: “Show in folder” instead.

The files of a closed request are kept 30 days, then erased (Settings › Agents › “Keep the files of closed requests”: 7, 30 or 90 days). Settings › Advanced › Local data says what they weigh and empties those of closed requests.

## Later, taking back, letting the lead decide

- **“Later”** (`L`): “In 1 hour”, “This evening, 6:00 PM”, “Tomorrow, 9:00 AM”, “Monday, 9:00 AM” or “Pick…”. The request leaves the queue and comes back at that time, marked “back from Later”.
- **Taking back your answer**: it goes 10 seconds after your gesture (“goes in 10 s · Undo”). Then, as long as the agent hasn’t read it, “Take back my answer” reopens the request; once read, “Change my mind…” takes you to its conversation with a message ready to finish.
- **Letting the squad lead decide**: when the project has a lead and “Agents can answer the other agents’ requests” allows it, “Let chef-atlas decide” leaves this request to it, and “Take it back” gives it back to you. A go-ahead and a task stay yours.
- **Go to the agent** or **to the ticket** the request is about, from the bar at the top.

## The trace

A closed request keeps its trace: your answer, who gave it (you, or the delegated squad lead), when and how (by click, from the keyboard, by voice), whether the agent read it, and its history (left, put off, answered, read…). An answer that settles a product decision says so: the agent proposes it in Docs › Decisions, where you accept it.
