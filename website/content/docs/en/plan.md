# Plan progress

When an agent follows a task list or works with subagents, Escouade shows where it stands in a “Plan” banner above the conversation, on its card, and in the list of its subagents. This page explains where the list comes from, what the banner shows, how to read a subagent, and how to follow background tasks.

## When the banner appears

The “Plan” banner appears between the conversation header and the messages as soon as the agent has at least one task, or at least one subagent at work. There’s nothing to do to display it: the agent decides whether to keep a list, and Escouade only reads it.

## Where the tasks come from

Escouade never creates a task and never changes a plan file: it reads what the agent does. The list comes from one of these two sources.

- **Claude Code’s task list.** The agent keeps it with Claude Code’s task tools (`TodoWrite`, or `TaskCreate` and `TaskUpdate`). The “Agents keep a task list” setting (Settings › Agents, on by default) gives this tool to the agents started afterwards; the model doesn’t have to use it. See [the harness settings](settings-claude-code).
- **A superpowers plan.** For an agent that has no task list, Escouade reads the plan the agent wrote in `docs/superpowers/plans/` (the `writing-plans`, `executing-plans`, and `subagent-driven-development` skills) and the progress ledger those skills keep in `.superpowers/sdd/`, to know which tasks are done. This reading only happens in the agent’s folder (its worktree, or the project) and changes nothing. If the agent also keeps its own list, that list is the list; the plan file then only gives its title and name.

The banner keeps at most 100 tasks.

## What the banner shows

At the top, a line sums up the plan:

- the “Plan” badge and the plan’s title: the plan file’s if there is one, otherwise the title of the agent’s ticket, otherwise the agent’s name;
- the subagents at work, as small dots followed by “3 subagents running” (see below);
- the count of finished tasks, “2/7 tasks”, and the percentage of the whole plan, “29%”;
- an arrow that folds and unfolds the list.

Below that line, a bar has one segment per task: full for a finished task, faded for a task to do, filled as far as its progress for a task in progress.

The list underneath gives one line per task, numbered (01, 02…):

| Element | What it says |
|---|---|
| Icon | ✓ for “Done”, a spinning circle for “In progress”, `!` for “Blocked”, a dash for “To do” |
| Title | The agent’s words, as they are |
| `↳ name` and the model | The subagent working on this task; a click on `↳ name` opens its conversation (see [a subagent’s conversation](#the-conversation-of-a-subagent)) |
| Gray text | What the task is doing right now, in the agent’s words (the subagent’s last tool, otherwise the task’s own “in progress” form) |
| “after 03, 05” | For a task to do, the numbers of the tasks it waits for that aren’t finished |
| Bar and percentage | The task’s progress: 100% when it’s done, 0% when it’s to do. For a task in progress in a plan file, it’s computed from its ticked steps (hover over the percentage to see “2/3 steps”). While no step is ticked, or nothing counts them, the line shows the state’s word instead (“In progress”, “Blocked”). |

The list centers itself on the line to look at: the blocked task, otherwise the task in progress, otherwise the first one to do. If you scroll the list, it doesn’t re-center for six seconds.

## A blocked task

When the agent is waiting for an answer to a question or a permission (“Question” status), its first task in progress goes to “Blocked”: the icon is a `!` and the line says “Waiting for your answer”. The banner doesn’t know which task the question relates to: it picks the first one that is in progress. Answer the card in the conversation (see [the conversation](chat#questions-and-permissions)).

## Subagents

A subagent is an agent that Claude starts itself for part of the work. While it runs, it counts in “N subagents running” and has its dot in the banner’s header (up to six dots, stacked, the others are only counted). The stack of dots is a single button. If only one subagent runs, a click on the button opens its conversation (hover over it to read the subagent’s title), whether it works on a task or not. If several run, the button opens a menu that lists them all, including those whose dot isn’t drawn, each with its title and its model: pick one to open its conversation. If it works on a task of the list, the task’s line names it and says what it’s doing.

When an agent has **no** task list but subagents are running, the banner is called “Subagents”: it gives the title, the stack of dots, and the count, with no list and no folding. It goes away when the last subagent has finished.

What a subagent says and does is read in its own conversation (see below), and in short in the agent’s conversation, on the line of the `Agent` tool (see [the conversation](chat#tool-calls)).

## Folding, unfolding, resizing

- A click on the top line (outside the subagents’ stack of dots, which opens their conversation) folds or unfolds the list. By default, it’s unfolded when the window is at least 800 px tall, folded otherwise. Your choice is kept for that agent until you close Escouade.
- A handle under the list can be dragged up or down to change its height; a double-click puts it back to the default height. The chosen height is used for all plans and is kept from one launch to the next. The messages always keep at least 160 px.

## The card’s line

In the sidebar, the card of an agent that has tasks shows a thin bar and “Plan 2/7” (or “Plan 2/7 · 3 subagents” when subagents are running). It lets you follow several agents at a glance, without opening their conversations. An agent that only has subagents, with no tasks, doesn’t have this line, but its subagents have their group under the card (see below). While subagents or background tasks run, their number is added next to its name, each with its icon (see [background tasks](#background-tasks)).

## Subagents in the sidebar

Under the card of the agent on screen, if it has subagents, a “Subagents” group lists them, even when the agent has no tasks; the other cards only show how many are at work. A click on its title unfolds or folds it. By default it’s unfolded while a subagent runs and folded otherwise; your choice is kept until you close Escouade.

Unfolded, the group first shows the subagents that are running, each with its title, its model, and what it’s doing right now (“Editing src/a.ts”). It shows five; “N more running” unrolls the rest. Below, an “N finished subagents” line unfolds the ones that have finished, from the last to finish to the first, each with its state (Done, Failed, Stopped, or Interrupted): ten at most, then “N more” for the rest. A subagent started by another is indented one step. The group doesn’t scroll: the sidebar already does.

The sidebar only keeps an agent’s 30 latest subagents: those running, then the latest finished. The count of finished subagents counts those; the older ones can still be read in the agent’s conversation (see below).

## The conversation of a subagent

Each subagent has its own conversation, which you read in full size, in place of the agent’s.

### Opening it

One click is enough, on any of these:

- the subagent’s line in the sidebar;
- the stack of dots in the “Plan” banner: in one click for a single subagent that runs, through a menu of all those that run when there are several (whether they work on a task or not);
- the `↳ name` of a task;
- the button at the end of the line of its call (the `Agent` tool), in the agent’s conversation;
- “Open its conversation”, above a permission request or a question it asked.

At the top, a breadcrumb “agent › subagent” tells you where you are. For a subagent started by another, it shows the whole chain, and each link opens the subagent it names. A click on the agent’s name, or `Esc`, takes you back to its conversation, and the focus returns to where you clicked (for the banner’s stack of dots, on its button, even if you chose the subagent in its menu); a click on the agent’s card in the sidebar brings you back too. `Esc` does nothing while a field holds text you typed: a draft isn’t lost to a key.

### What the view shows

- **The state** of the subagent, next to the breadcrumb: Running, Done, Failed, Stopped, Interrupted, or “Launched” when nothing says more.
- **Its sheet**: its type, its model, its tokens, the tools it used, and its duration (which runs while it does). Below, its prompt, that is what it was asked, folded when it’s long (“Show the whole prompt”).
- **Its conversation**: what it says, its thinking, its tool calls (which unfold, as in the agent’s conversation), and its permission requests and questions. You answer them as usual, with the button or the keyboard (see [the conversation](chat#questions-and-permissions)). These requests also stay in the agent’s conversation, headed by “Request of the subagent” and its name.
- **Its end**: “The subagent is working…” while it runs, then its report, rendered as Markdown, or “This subagent handed back no report.”

Titles, prompts, and reports are the agent’s words, as they are: Escouade doesn’t translate them.

### Stopping it

“Stop”, at the top right, is only for a subagent that runs **in the background**. It stops this subagent alone: the agent goes on. Claude Code then gives the agent a turn to notice the stop, and that turn counts like the others (tokens, cost). While Claude Code hasn’t told the subagent’s task, the button stays there, grayed out, and its tooltip says why it can’t act yet.

A subagent in the foreground has no button: the agent waits for it, and the agent’s own “Stop” stops it along with its turn. Stopping one alone has never been tried on a real stream, so Escouade doesn’t offer it. There’s no button in [the Claude Code view](claude-code-view#subagents) either.

### Writing to it

Claude Code doesn’t let anyone write to a subagent directly. For a subagent that runs **in the background**, a “Write to it, through” field followed by the agent’s name appears at the bottom of the view. Your message is sent to the agent, with the request to pass it on as it is to the subagent through Claude Code’s `SendMessage` tool; the field says so under the text area. `Ctrl+Enter` sends the message, and answers no request that is waiting.

> [!NOTE]
> The message goes through the agent. It appears in its conversation as that request, it costs it a turn, and the subagent only receives it at its next tool. The agent is only asked to pass it on: nothing forces it to. In the test done on Claude Code 2.1.300, it did, and the subagent took it into account.

The field isn’t offered for a subagent in the foreground (the agent waits for it and could only pass the message on once it’s over), nor in the Claude Code view, nor for an archived agent. What you typed is kept when the subagent finishes or when you change view, as long as Escouade stays open; the field then stays, with the send grayed out and a note that the subagent has finished.

Claude can also list an agent’s subagents, read their report, stop a subagent in the background, and write to it through the [MCP server](mcp#an-agent-s-subagents-and-background-tasks), with the same refusals as these buttons, including for a message to a subagent that is no longer running.

### Afterwards

A finished subagent’s conversation can still be read. Escouade keeps each subagent’s prompt, its report, and its final figures (state, tokens, tools, duration) with the agent’s conversation. The prompt and the report are cut at 16,000 bytes, and a cut text ends with “…”. When a subagent finishes without handing back a report in due form, its last message serves as its report. The report of a subagent in the foreground is the result of its call, cut at 8,000 bytes.

Its conversation can therefore be read after Escouade restarts, and even when the sidebar no longer has its line (beyond 30 subagents, or once the plan has started over): the button on the line of its call, in the agent’s conversation, still opens it. A subagent that was running when Escouade closed is shown as “Interrupted”.

## Background tasks

Claude Code can start tasks in the background for an agent: a command (the `Bash` tool with `run_in_background`), a monitor (the `Monitor` tool, which follows a command’s output and alerts the agent at each line), a workflow, a remote agent, or a teammate. Subagents have their own list (see above) and aren’t background tasks. Escouade lists these tasks under the agent’s card, opens the detail of each, and shows the output of a command or a monitor live. They don’t open the “Plan” banner: it only deals with the tasks of the list and with subagents.

### In the sidebar

Under the card of an agent that has background tasks, after the subagents group, a “Background tasks” group lists them, built like it. A click on its title unfolds or folds it. By default it’s unfolded while a task runs and folded otherwise; your choice is kept until you close Escouade.

Unfolded, the group first shows the tasks that are running, in the order they were started: five, then “N more running” unrolls the rest. Each line gives a state dot, the task’s title, its duration (which runs while it does), its type (“Background command”, “Monitor”, “Workflow”, “Remote agent”, or “Teammate”) and, while it runs, its command line or the workflow’s phase. Below, “N finished tasks” unfolds the ones that have finished, from the last to finish to the first, each with its state (Done, Failed, Stopped, or Interrupted) and, for a command, its exit code (“Failed (code 3)”): ten at most, then “N more”.

On the agent’s card, next to its name, a number with a clock icon counts the tasks that are running: commands, monitors, workflows, remote agents, and teammates, not subagents. It goes away when nothing runs any more.

A command started in the foreground, which the agent waits for, isn’t in the list: it joins it the moment Claude Code sends it to the background, with its start time.

### A task’s view

A click on a task opens its view in place of the agent’s conversation. One view at a time: a subagent’s conversation opened before gives way. At the top, a breadcrumb “agent › task” tells you where you are; a click on the agent’s name, or `Esc`, takes you back to its conversation, and the focus returns to where you clicked. A click on the agent’s card in the sidebar brings you back too. `Esc` does nothing while a field holds text you typed: a draft isn’t lost to a key. Next to the breadcrumb, the task’s state, and “Stop” while it runs.

Below:

- **Its sheet**: its type, its duration (which runs while it does), the exit code of a finished command and, when a subagent started it, “Launched by” followed by its name. A workflow adds its current phase, its tools, and its tokens.
- **Its command**, folded when it’s long (“Show the whole command”). A task with no command shows its description instead.
- **Its summary**, once finished: Claude Code’s last word on it (for example `Background command "Ticker" completed (exit code 0)`). For a stopped task it’s only its description, already said: Escouade doesn’t repeat it.
- **Its output**, described below.

Titles, commands, phases, summaries, and outputs are the words of the agent and of Claude Code, as they are: Escouade doesn’t translate them. If the task disappears from the agent’s list (the plan has started over), the view closes.

### Live output

A command’s or a monitor’s output is read live, read-only, as in a terminal but without its colors or escape codes. Escouade reads it again every 1.5 seconds while the task runs and the window is visible, then one last time when it finishes; each read replaces what is displayed. The view follows the end while you’re there; if you scroll up, the “Follow the end” button takes you back down.

What you read is the **end** of the output: 8 KiB at most while the task runs, up to 256 KiB once it has finished, so that the first error of a failed build stays readable. When the start is missing, the view says “Start cut off: only the end of the output is shown.” The output of a running task comes from Claude Code itself while the agent’s process runs; a finished task is read first from the output file Claude Code keeps for it, and the view then says “read from the output file”. That file is also used when there is no process any more (process stopped, Escouade reopened, [the Claude Code view](claude-code-view#background-tasks)). Escouade only reads the task’s own file (`<task>.output`, in a `tasks` folder), never another, and never a file on another machine: a network path (`\\server\share`) is ignored and nothing is opened. With neither a process nor a file, the view says “No output available: the output file cannot be found.”

A workflow, a remote agent, and a teammate don’t write to an output of their own: the view says so (“No output to follow for this type of task.”).

### Stopping a task

“Stop”, at the top right, stops this task alone: the agent goes on. Claude Code then tells the agent about the stop, which costs it a turn like the others (tokens, cost). The button is offered for any task that runs. It has been tried on a command; a monitor, a workflow, a remote agent, or a teammate is stopped by the same request, but that hasn’t been tried on a real stream.

Claude can also list an agent’s background tasks, read the end of their output, and stop them through the [MCP server](mcp#an-agent-s-subagents-and-background-tasks), with the same limits and the same refusals.

### What stays after the end

The ten latest finished tasks stay in the list, with their command and their summary, until the plan starts over (a new Claude session, or your first message once everything is finished: as long as a background task runs, the plan isn’t finished). The list is kept when you close Escouade. A task that was running at that moment, or when the agent’s process stops, is shown as “Interrupted”: a command started in the background stops with the process. The end of an agent’s turn doesn’t end a background task.

An agent that has a background task running isn’t stopped as idle (see [idle processes](managing-agents#idle-processes)), and the “Restart” button is disabled while it runs: a command or a dev server doesn’t end by itself, so waiting is no use. Stop its background tasks first, from their view (“Stop”). Escouade’s automatic restart for an update waits for them too (see [updates](updates#restart-automatically)). In the agent’s conversation, the end of a task stays a discreet “Background task completed” line (see [the conversation](chat#subagents-background-tasks-and-other-sessions)): it doesn’t open the task’s view.

### In the Claude Code view

Background tasks are listed too when the agent is in [the Claude Code view](claude-code-view#background-tasks), with their view, but without “Stop”: the view says “This task is stopped from Claude Code’s own interface.” Escouade reads a task’s start in the result of the tool that Claude Code writes in its transcript, and its end in the notification it replays there: the start time is therefore not exact to the second. The output is read from the output file.

### Limits

- Background tasks aren’t in the [overview](overview).
- A command that a subagent starts in the foreground isn’t listed; one it starts in the background is, with “Launched by”.
- Monitors of MCP, remote agents, and teammates are supported from Claude Code’s schemas; Escouade has never seen any on a real stream.

## When its turn is over but the work goes on

An agent can end its turn and leave work going on: subagents or background tasks, a turn Claude Code will start by itself later, or other Escouade agents it set to work through the [MCP server](mcp). Its card then doesn’t say “Done” but “Awaiting”, in the color of a running agent. Hover over the word: the tooltip lists what goes on, for example “1 background task running” or “2 driven agents at work: pr-148, pr-149”. The card goes back to “Done” once nothing runs.

### Wake-ups

Claude Code can schedule a turn for later: a scheduled task of the session (`CronCreate`, what `/loop` does with an interval) or the next wake-up of a loop (`ScheduleWakeup`, `/loop` without an interval). Nothing runs meanwhile, but the agent will resume by itself at that time. When that’s all there is, its card says “Wakes at 2:20 PM”, and the tooltip tells what Claude Code will do. A deleted task (`CronDelete`) or a stopped loop no longer counts. These wake-ups live in the agent’s process: Escouade doesn’t stop it for being idle while one is to come.

### Driven agents

An agent that drives others through the MCP server (it writes to them, creates them, starts, sends back, or resumes their ticket) sees them listed under its card, after its background tasks, in the “Driven agents” group, while one of them works or waits for an answer. Clicking a name opens that agent. The group goes away once they’re all done.

What an agent hands off outside Escouade (a message to another Claude Code session, a cloud routine) isn’t followed: only the tool’s line, in its conversation, says it.

## When the plan goes away

When all the tasks are done and nothing is running (no subagent, no background task), the banner stays displayed (“7/7 tasks”) until your next message: that message starts a new plan, and the banner starts over from zero with whatever the agent lists next. The banner also works when the agent runs in [the Claude Code view](claude-code-view).
