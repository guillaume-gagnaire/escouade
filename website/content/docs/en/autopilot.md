# Autopilot

The autopilot takes a project’s “To do” tickets and gives each one to an agent, in its own worktree, which loops until the acceptance criteria are met. This page explains how it chooses, loops, pauses near the quota limit, and resumes.

## Turning it on or off

The “Autopilot” switch is in the header of the [Kanban](board). It is per project and on by default. It is the same setting as “Assign tickets automatically” in Settings › Kanban (see [Kanban settings](settings-board)).

When it is off, the header reads “Autopilot · off”, the “To do” cards say “Autopilot is off”, and each ticket starts when you click “Start” on its card (see below).

Nothing starts, whether the autopilot is on or not, in three cases:

- Claude Code is not found: the header says “Claude Code not found — no tickets will start”.
- The Kanban’s target branch has no commit yet, or no longer exists: the header says “main has no commit yet — no tickets will start” or “Target branch main not found — no tickets will start”, and the “To do” cards say “Waiting for the target branch”.
- No active Claude account serves the project (every one is “Reserved to projects” other than this one): the header says “No Claude account for this project — no tickets will start”.

## How many agents in parallel

The “In parallel” setting (Settings › Kanban) sets how many tickets are “In progress” at the same time: from 1 to 6, 2 by default. The header counts the free slots (“1 free slot”, “All slots are taken”).

An “In progress” ticket holds a slot, even while its agent waits for its quota to reset. A blocked ticket no longer holds one.

## Which ticket starts, and when

As soon as a slot frees up, the autopilot takes the first “To do” ticket that can start.

- **The order** is the column’s, from top to bottom. A ticket moves to the top with right-click › “Move to top”; new ones arrive at the bottom.
- **Dependencies**: a ticket whose “After” field names tickets that are not all “Done” is passed over, and the next one goes ahead. Its card says “⏸ after DEM-3”. It starts as soon as the last of those tickets is done. A deleted ticket holds nobody back.
- **The next ticket’s card** says “Picked up as soon as a slot is free”; the others say “Waiting for a slot (2/2)”.

### Starting a ticket by hand

With the autopilot off, a “To do” card has a “Start” button as long as a slot is free and nothing holds the tickets back. The ticket starts at once, without waiting for its turn (its card says “Start requested…” while it starts).

If the ticket comes after tickets that are not done, Escouade first asks “DEM-5 is waiting for DEM-3, which is not done yet. Start it anyway?”; “Start anyway” starts it without them. A ticket started by hand respects the pause and the free slots like any other.

## One ticket, one worktree, one agent

When a ticket starts, Escouade:

1. moves it to “In progress”, at loop 1;
2. creates an agent, named after the ticket’s key and title (`dem-5-export-csv-des-factures`);
3. prepares a worktree for it on the ticket’s branch, `ticket/dem-5` (the prefix is the project’s “Tickets’ branch prefix”), made from the target branch. For a ticket imported from a source, the branch and the agent carry the original ticket’s key and its title (`ticket/proj-123-fix-the-login`). If the ticket takes up an existing branch, the agent works on that one;
4. waits for the worktree setup: copied `.env*` files, the project’s setup commands (the card says “Setting up the worktree · …”). If a command fails, the agent is told with its first message, so that it can sort it out;
5. sends the agent its first message: the key and title, the description, the numbered criteria, and the instruction to work until it meets them, then end with the report.

This message, like the following ones, is written in “Language of texts written by the agents” (Settings › Application, see [Application settings](settings-application)).

On every start, Escouade also adds to the agent’s system prompt a protocol that outlives compaction and resumes: work alone, check each criterion yourself (tests, runs) before declaring it met, don’t commit the files copied from the project, ask questions with the question tool when a decision is missing, and end every answer with the criteria report. Escouade also reserves a block of ports for the ticket, and the protocol asks the agent how to launch its work to test it (see [Reviewing and approving a ticket](testing)).

### Model, effort, and permissions

In Settings › Kanban, the “Agents” group sets the agents of the project’s tickets: “Model”, “Effort”, and “Mode” (of permission). Left on “Default”, they follow the defaults of Settings › Harness (see [Harness settings](settings-claude-code)).

Ticket agents stay in Escouade’s view. If one of them asks for a permission or asks a question, its card turns amber with “Question waiting for your answer”, you are notified, and you answer in its conversation (see [Chat](chat)).

### Which account it starts on

A ticket starts on the project’s “Preferred account” if it is active. Otherwise, on the first active account, in the order of Settings › Accounts, that is “good” (see [Claude accounts](accounts#the-automatic-choice)) and under the “Pause above quota” threshold, has no pause due to the usage limit, and has no agent waiting for its reset. A “Reserved to projects” account only serves those projects. The agent then stays on its account, unless it resumes on another account after the usage limit (see below). See [Claude accounts](accounts).

## The loop

A loop is one pass of the agent on the ticket: it works, answers, and ends its answer with the criteria report. Escouade reads this report at the end of every turn (see [The Kanban](board#the-criteria-report)).

| At the end of the turn | What Escouade does |
|---|---|
| Every criterion is met | The ticket moves to “To review” and you are notified (“ready to review”). |
| Criteria are missing, and loops remain | The next loop begins: the agent receives the list of criteria not met, with its own notes, and the card’s counter moves to “Loop 2/5”. |
| Criteria are missing on the last loop | The ticket moves to “To review” with the “Partial goal” mention. It is up to you to approve it as it is or send it back. |
| No report | The agent is prompted once. If it gives none again, the ticket gets blocked (“Criteria report missing”). |
| Turn interrupted (`Esc`, stop button) | The ticket gets blocked (“Interrupted”). |
| Claude Code error | The ticket gets blocked (“Error: …”). |

The maximum number of loops is the ticket’s “Max loops”: 3, 5, or 8. A criterion that the report forgets counts as not met. At each loop the report replaces the previous one: a criterion can go from ✓ back to ○.

A blocked ticket notifies you (“blocked”), no longer holds a slot, and shows its reason on its card. “Resume” sends the agent back to work (“Pick up ticket DEM-5 where you left off, then end with the report”). If the agent no longer exists, the ticket starts again from scratch. While the agent waits for its quota, “Resume” answers “Waiting for the quota — resumes at 14:30”.

When a ticket goes back to its agent after “Send back”, failing tests, or a conflict (see [Reviewing and approving a ticket](testing)), the counter starts again at “Loop 1/5”.

## Pausing near the quota limit

The “Pause above quota” setting (Settings › Kanban, “for all projects”) takes 80%, 90%, 95%, or 100% (the default). As soon as an account’s 5-hour window or weekly window reaches this threshold, the autopilot starts no ticket on it until the window resets. This threshold holds back the autopilot alone: a ticket launched by hand (“Launch”, or an agent’s `start_ticket`) still starts, and only a usage limit reached holds it back. The queue starts again by itself 30 seconds after that reset, or as soon as the quotas read are back under the threshold.

The Kanban’s header says so on a line of its own, for example “Autopilot paused: 5-hour quota at 95% (resumes at 2:30 PM)” or “Autopilot paused: weekly quota at 96% (resumes on Friday, October 2 at 9:30 AM)”, with a “Resume now” button. The “To do” cards say “Waiting: autopilot paused”. Agents already at work carry on.

- With several accounts, the pause only applies when every account the project can use is held back: a single free account is enough, and the ticket starts on it. The line then adds the accounts that went past the threshold (“The Main and Pro accounts are past the threshold.”), and the pause lasts until the first one frees up.
- A project that has a “Preferred account”, or “Reserved to projects” accounts, is paused according to its own accounts.
- “Resume now” lifts the pause: the windows past the threshold hold nothing back until they end, and whatever can start starts.

During a pause nothing starts, not even a ticket started by hand.

## Usage limit and resume

When a ticket’s agent reaches its account’s usage limit, its turn stops. What follows depends on your settings (Settings › Harness):

- **“Auto-resume after the usage limit”** (on by default): Escouade waits for the quota to reset, then sends “continue” to the agent. The ticket stays “In progress” and keeps its slot; its card says “Resumes at 2:30 PM”. When every account the project can use has an agent waiting like this, the header says “Quota reached — resumes at 2:30 PM” and nothing starts before then. As long as an account has none, a ticket can start on it.
- **“Resume a ticket’s agent stopped by the usage limit on another account”** (with several accounts, off by default): the agent goes on at once on another usable account, with its session and its conversation. An account is usable if it is signed in, under the threshold, has no pause due to the usage limit, and has no agent waiting for its reset. A session does not move between a Windows account and a WSL account. The account it leaves stays held back until it resets (30 minutes if the reset is not known). Without this setting, the agent waits for its account to reset.
- **No resume planned** (setting off, API key, unknown reset): the ticket gets blocked (“Error: usage limit reached”), and the autopilot pauses for 30 minutes on that account, because the next ticket would meet the same limit (“Autopilot paused: usage limit reached (resumes around 2:30 PM)”). “Resume now” lifts this pause.

## When the app restarts

Closing the window does not stop the agents. If the app stops and starts again, each “In progress” ticket that is not blocked resumes: its agent receives a message to carry on where it was. A ticket whose start was cut before the agent’s first message goes back to “To do” and starts again, even with the autopilot off.

## See also

- [The Kanban](board): the columns, the cards, and the criteria report.
- [Reviewing and approving a ticket](testing): what happens after “To review”.
- [Kanban settings](settings-board): “In parallel”, “Pause above quota”, model, effort, and mode.
- [The MCP server](mcp): start tickets from Claude, under the same guardrails.
