# Changelog

Escouade's visible changes, version by version, in English. The notes of each GitHub release and the English Discord channel take the section of its version; the French notes, those of the in-app update included, are in [CHANGELOG.md](CHANGELOG.md), which also holds the versions before 1.8.2.

To write an entry: one or two short sentences, for a human reader. What they can now see or do, and why it helps; the details (edge cases, settings, file names, tools, ticket numbers) go in the documentation, not here. Each entry of CHANGELOG.md has its English one here, under the same version and the same heading (`## [Unreleased]` for `## [Non publié]`).

## [Unreleased]

### New

- A plugin’s program runs apart, started only when it is needed, and Escouade keeps it in check: a plugin that does not answer in time is no longer waited for, one that crashes restarts (three times, then stops), one that takes too much memory is stopped, and each gets only its own secrets. Its journal keeps what it did and said, and the protocol is described in the plugins’ documentation.

### Fixes

- “Write a first spec from the code”, and every new discussion, closes its window and opens the discussion with its first message even when new agents open in the harness terminal: a discussion always stays in its conversation, where Escouade refuses its writes.
- The card of the tickets “Plan tickets” made now matches the subjects’ cards in the Docs tab: nothing spills out of it, it says how many tickets have started, “Undo” only shows for those not started yet, and it folds once all have started or closes.
- A discussion that can’t start no longer leaves an agent behind: the window says why and keeps your choices, to start again or switch accounts.
- On Linux, Escouade no longer reads every project’s git in a loop while nothing changes, which could keep the window busy and freeze it; a line of several megabytes in a diff is now only partly shown.
- The interface no longer freezes during a long operation (a big folder, git, a lock): it stays responsive while the command works, and the log notes slow commands and window pauses to diagnose them.
- When the interface’s page stops dead, Escouade reloads it without reopening the project it showed and offers to reopen it: a project that brings the page down no longer freezes the whole app.
- The output of a tool that read a binary file is shown cleaned (control characters made visible, terminal colours removed) instead of being lost, and a very long output is shown in parts.

## [2.1.1] — 2026-10-29

### Fixes

- On Linux, opening some projects crashed Escouade’s page, which looked frozen: a flaw of the AppImage’s web engine in laying out some paragraphs, which the app no longer uses on Linux or macOS.

## [2.1.0] — 2026-10-29

### New

- A spec imported into the Docs tab becomes the only one agents follow: they are told the old file is no longer the spec, and nothing touches it.
- Gemini CLI becomes an eighth harness: its agents converse in Escouade over ACP, with its models, its modes, its plan to approve and its questions, each in its own configuration folder; not yet tried with a real account.
- Gemini CLI has its accounts, each signed in to Google by gemini in a terminal of the window or by its API key, writes its texts with gemini -p, goes on by itself after its usage limit, and its “Gemini CLI view” opens its interface on the agent’s session.
- Antigravity CLI, Google’s agent, becomes a seventh harness: its agents converse in Escouade with its models, its efforts and its Plan mode, and what it refuses during a turn is asked of you at the end of the turn, then taken up again.
- Antigravity CLI has its accounts: the machine’s, added and signed in by agy in a terminal of the window, and others by their Gemini API key; when its quota is spent, the agent waits and goes on by itself, or moves to another account.
- The “Antigravity CLI view” opens Antigravity’s interface in a terminal, on the agent’s conversation: Escouade follows its messages, tools and turn ends, gives it its MCP server, and going back to the conversation resumes the same conversation.
- Mistral Vibe becomes a sixth harness: its agents converse, ask their permissions and their questions in Escouade, with its models, its modes and a Plan mode played by Escouade.
- Mistral Vibe has several accounts, each by its Mistral API key, and shows what each turn costs, in the conversation, the status bar and the statistics; at its rate limit, the agent goes on by itself a minute later.
- The “Mistral Vibe view” opens Vibe’s interface in a terminal, on the agent’s session: Escouade follows its messages, tools and turn ends, gives it its MCP server, and going back to the conversation resumes the same session.
- A code index, offered when a project is added and in its settings, lets agents find code faster and for fewer tokens, with nothing leaving the machine; the statistics show what it saved, in tokens and dollars.
- A project’s agents share a working memory, whatever their harness: they note in it the project’s ways of doing and its pitfalls, and Escouade offers once to bring in what Claude Code had remembered.
- The project’s documents have their drive in the Docs tab: folders, drag and drop and a search; a file dropped (PDF, Word, Markdown, text) is read on your machine and filed in the right folder by itself, and each one says whether the agents read it and how many times it is cited.
- A Docs tab joins Agents and Kanban: the product’s specification, by subject on a map that shows where each subject stands against the code, its pages in a tree, to read, edit and share with the team in one click; a spec the project already has is imported, cut by its titles.
- The decisions on the product have their register, numbered and dated: where each comes from, what it changed in the spec, and “Accept” in one click; an accepted decision is no longer rewritten, a new one replaces it, and the one a ticket’s agent left is accepted when you approve the ticket.
- “New discussion” in the Docs tab: discuss the spec with a spec writer agent, or brainstorm with a product agent that thinks about uses, GDPR and what other trades do; what they propose shows page by page and is poured into the spec in one click, without them ever touching the code.
- The spec follows the code: a ticket’s agent updates the pages it touches and leaves a decision, then, once the ticket is approved, an agent compares those pages to the code and shows the gaps on the map of the subjects and at the top of the page, to settle in one click; “Compare to the code” does it again on demand.
- “Plan tickets” turns the changes of the spec into tickets: a planner reads what changed, the code and the board, then makes the tickets needed in “To do”, without duplicates and without starting them, linked to the pages they implement; “Undo” removes them.

### Fixes

- A new agent of a harness without an Auto mode (Cursor CLI, Mistral Vibe, Antigravity CLI, Gemini CLI, OpenCode) starts in “Ask”, no longer in an Auto it doesn’t have.
- “Approve the plan” resumes in the mode the agent was in before Plan, Auto for instance, instead of “Ask”, with every harness; the button says which mode.
- A ticket launched by hand starts even past “Pause above quota”: that threshold now holds back the autopilot alone.
- A ticket launched by hand that can’t start says so, with the reason, instead of staying in “To do” without a word.
- Under WSLg, links, files and folders open in Windows, instead of doing nothing.

## [2.0.5] — 2026-10-27

### Fixes

- An update found, by “Check for updates” or by the automatic check, also shows in Settings › About: downloading, then ready with “Restart now” and what’s new.

## [2.0.4] — 2026-10-27

### New

- Azure DevOps becomes a code host, on dev.azure.com as on Azure DevOps Server: connect your organization with a token or through `az`, then open an agent’s pull request, follow its builds and reviews on its card, and hand it a failed build or the feedback to fix.
- Azure Boards becomes a Kanban source, on the same Azure DevOps account: import a project’s work items with their acceptance criteria; their state and comments follow their ticket, and its agent’s pull request is linked to them.
- The agent and branch of an imported ticket carry the original ticket’s key and its title (`proj-123-fix-the-login`, `45-…`), as its pull request does: your tool links them to the ticket.
- The Workflow tab shows where each agent is: its steps done ticked, the one it is on, and the next marked “Next”, with both ends of a choice; its line and its card’s tooltip say “Next: …” too.
- The Workflow tab can also be read as a diagram, which brings out the loops, the ways back and the choices, and where each agent is; it enlarges over the whole work area, and “Copy as Mermaid” gives the workflow to paste into a doc or a pull request.

### Fixes

- Through the MCP server, an agent’s or a ticket’s model and effort are checked against its harness (Codex, Copilot CLI…), not Claude Code’s: a Codex project accepts GPT-6-Luna, and `list_projects` gives each project’s models.
- In the widened projects rail, a click on a project’s name opens it, as a click on its tile does.
- The name on your shared tickets is taken from git when you gave none: “Share” no longer asks for it, and Settings › Application shows it filled in.
- A setting with wide choices, like “When I approve a ticket” in the “New project” assistant, no longer has its label squeezed word by word: its choices go under it when they do not fit beside it.
- The card of an agent asking a question fits in the agents’ sidebar at any width: a long choice wraps, and its status, its question and its branch are no longer cut at the edge.

## [2.0.3] — 2026-10-27

### New

- Escouade reads your project’s workflow from the skills, commands and sub-agents of your harnesses (`.claude`, `.agents`, GSD, Superpowers, a `PROMPT.md` loop…), at no cost, and follows each agent through it in real time, with its history.
- The Workflow tab of the right panel shows its steps, their ways back and each agent on its own. An agent’s card says its step, its conversation marks each entry in a step, and the loop of the tickets is followed there as a “Tickets” workflow.
- In Settings › Projects › Workflow, you correct the steps, and “Refine…” has a short agent make them more precise, on request and with its cost said first. The configuration is kept in the repository’s `.escouade/workflows/`: commit it for the whole team to see the same.
- The MCP server reads the workflows: `get_workflow`, and `workflow` in `list_agents` and `get_agent_summary`.
- A ticket of the Kanban can be shared with the team: Escouade writes a file of it in the repository, keeps its status up to date, and you commit it with your work. For your teammates, it lands in “To do” with your name; their autopilot never starts it, but they can start it by hand.
- GitLab Issues becomes a source of the Kanban, on gitlab.com as on your own instance, with the GitLab account already connected as a code host.
- ClickUp joins Jira, Trello, GitHub Issues and Linear: a list linked to each project, its tasks imported with their checklist as criteria, then their status and a comment as the ticket moves on.
- The status bar shows the branch of the agent open and the state of its pull request, which a click opens. Its picker moves the agent’s worktree to an existing branch, and the agent learns it with your next message.
- An agent’s pull requests are found on the branch it really is on, whoever opened them, from a fork included. Thanks to @Metalzoid (#17).
- The quota projection now has three settings in Settings › Accounts: “Above 80%” (the default), “Always” and “Never”. Each meter’s tooltip always gives it. Thanks to @epommereuil (#16).
- The sidebar of the agents and the Kanban widens by dragging its right edge, and the side panel closes by its × button.

### Fixes

- `/compact` now really compacts the context of a Codex, Copilot CLI or OpenCode agent, instead of reaching the model as a message.
- The commands menu also opens on a `/` typed in the middle of a message, after a space.
- Settings › Integrations sets the code hosts (PRs, CI) apart from the Kanban’s ticket sources, and the import window offers to link your GitHub repository or GitLab project in one click.
- On Linux, Escouade’s window has its own icon under Wayland (WSLg, GNOME, KDE), no longer Tux’s.
- Pasting on Linux no longer holds the window: the image is read only when the clipboard has one, and a clipboard that does not answer is said within three seconds.

## [2.0.2] — 2026-10-26

### Fixes

- A very long description no longer hides the fields of a ticket’s form: the window scrolls down to the criteria, the account, the model and the other fields, and its buttons always stay in view.
- The branch picker no longer flickers on hover: its list no longer scrolls under the mouse and the branch’s description keeps its height, as “Open a file”, the search through conversations and the editor’s definitions no longer scroll on hover.
- The “PR and CI” tab of the Git panel announces the code hosts for 2.1, no longer “after 2.0”.

## [2.0.1] — 2026-10-25

### New

- A project’s tile in the projects’ rail can say something other than its initials: up to three letters or an emoji, or the logo of your choice in their place, in Settings › Projects or in the “New project” assistant. The agents waiting and those at work stay readable over it.
- Cursor CLI becomes the fifth harness: a Cursor agent converses in Escouade, asks its questions and permissions there, proposes its plan to approve and keeps its tasks and subagents in the Plan banner. Its “Always” becomes “For this agent”: the rule holds for that agent alone, never for your Cursor config.
- Cursor CLI takes several accounts: another Cursor account is added by its API key, kept in the system keychain. Its usage of the month opens in one click on Cursor’s dashboard, and a turn stopped by its usage limit says so on a card, with where to go on.
- Written texts (an agent’s name, a commit message…) can go through Cursor CLI, and a Cursor write refused by a hook shows as failed in the conversation instead of passing for done.
- A project can show a label under its tile, its name or a shorter text, and the projects’ rail widens by dragging its right edge: wide enough, it shows each project’s name beside its tile, as a list. The width chosen is kept for the whole app.
- Settings › About offers to buy a coffee to support Escouade, as do the site, the documentation and the repository’s GitHub page.
- The Cursor CLI view opens Cursor’s interface on the agent, in a terminal: Escouade follows its messages and the end of each turn there, and its band says what it doesn’t see (permissions waiting, results, tokens). Cursor keeps a conversation of its own there, which Escouade links to the agent’s both ways.
- A project can be a folder that holds several git repositories, one per subfolder (front, api, infra…): Escouade finds them, shows each one’s branch and changes, and gives every agent a worktree of each repository on one branch. Merge, pull requests and a ticket’s approval happen in each repository changed, and nothing is merged while one of them can’t be.

### Fixes

- The images and files an agent sends you now show in its conversation: its caption, the images as thumbnails to see full size in a click, the other files to open or find in their folder. The images stay visible even once their folder is gone, and those of a message in Markdown show too.
- The models menu of a Codex, Copilot CLI, Cursor CLI or OpenCode agent offers every model of its account from the start, no longer a stand-in model alone: they are read as soon as the account is added and at startup, without starting an agent, kept from one start to the next and read again every day. Until they are read, the menu says so.
- The model a Codex account’s `config.toml` picks shows in its menu, marked “from config.toml”, even when Codex doesn’t list it.
- A harness installed while Escouade runs (`npm install -g @openai/codex`…) is found without restarting it: when the window comes back to the front, when the “Harness” menu opens or with “Search again”.

## [2.0.0] — 2026-10-24

### New

- Escouade has a light theme: Settings › Application › Display › “Theme” offers System, Light or Dark, applied at once, title bar included. By default, it follows the one of Windows, macOS or Linux.
- Projects move to a rail on the left, like Slack’s workspaces: a tile per project, with its agents waiting and at work, dragged or moved by Alt+↑ and Alt+↓. The title bar keeps the name of the project on screen and the search; the status bar shows the quotas of each harness at work.
- The list of agents is redrawn: the agent waiting goes first with its question, which you answer in a click or with 1, 2… without leaving the conversation open (several questions and permissions too; Ctrl+J puts the focus on its card), and each card says what the agent is doing (“Editing src/auth.ts”), whatever its harness, what it changed, or what still runs once its turn is over. The arrow of “New agent”, or Ctrl+Shift+N, opens the menu of the harnesses, numbered.
- The conversation is redrawn: a header with the status in a pill and the agent’s figures, its actions under “…”, the tool calls of a turn in a single thread, and a floating message field where dictation and dropped files happen. A message refused for want of an account says so in the conversation, with “Add an account…” a click away.
- The “Plan” banner is redrawn in the conversation’s column: a segment per task, then each task with its state, the subagent working on it and the model it really runs on, whatever its harness, what it is doing and what it waits for.
- The Kanban is redrawn: columns without frames, cards that show their criteria checked, the loop in squares, what the agent is doing and what it costs, and the ticket to review in amber with “Approve” and “Test”. In a ticket’s form, every field has its label, the description takes all the free room and “After” opens as a menu.
- Git is redrawn beside the conversation: “Uncommitted” and “History” tabs, the files with their letter and their counts, the diff in a frame, “Commit…” and the merge at hand. The history draws each agent’s branch joining the main one, and tells which agent made each commit.
- Terminals and launches are redrawn: a command’s log tells its state, since when it runs, its command, its folder and the worktree it runs in, and the notification of a command that crashed has “View the log”. Terminals follow the light or dark theme at once, and the “Test” window tells who wrote the commands before they run, then each step with its time. In a short window, Launch and Terminals fold to their title, which unfolds them with a click.
- The editor is redrawn: folders holding changes are marked with a dot, lines added or modified with a green or amber mark in the margin, and the search in the file floats over the text with its count (“1 of 3”). It follows the light or dark theme at once, and “Open a file” and the search through the conversations get the same look.
- The overview and the statistics are redrawn: the agents waiting for your answer first, answered on the spot, then each project’s, and “Awaiting” for an agent whose turn is over but not its work. The statistics show four figures, each day’s tokens in columns with their detail on hover, and the cost by project, by model and by harness, readable in both themes.
- In the Codex or Copilot CLI view, an amber band tells when the harness asks whether it may trust the agent’s folder; the OpenCode view reminds that Escouade keeps the conversation’s thread.
- Windows are redrawn: the “New project” assistant and its rail of steps, “Add an account” with the harnesses in segments and the sign-in in a frame, “Open a PR” telling which uncommitted files are left out, and the notes of an update. Buttons draw their icon, and menus have the preview’s entries.
- The settings are redrawn: their tabs in three groups, a new “MCP server” tab among them, each harness with its state and version, each account on a card with its quota and its new agents’ settings, and all that happens at a quota’s limit gathered in Accounts. An update that does not download stays told in About, with “Download in the browser”.

### Fixes

- A message sent right after the one before, before that one is taken, is no longer lost: it leaves the field and goes right after it.
- An agent’s header no longer overflows when the files and history panel is open or the window is narrow: it puts away its figures, then the words of its buttons, and moves the layout into “…”, always keeping the agent’s name and status.

## [1.8.14] — 2026-10-24

### New

- On Windows, the installer and the app are signed by GAGNAIRE DEVELOPPEMENT: Windows says who publishes them, and the SmartScreen warning fades as this signature builds its reputation.

## [1.8.13] — 2026-10-24

### Fixes

- The “New project” assistant now sets everything a project has at its creation: default harness, “Private project” with the accounts to reserve to it, and the whole Kanban. .escouade.yml also shares “Private project” and the model, effort and mode of the ticket agents; what belongs to each person (accounts, remote control, tickets in parallel, autopilot) stays on their machine.

## [1.8.12] — 2026-10-23

### New

- Each project chooses the branch its new agents’ worktrees start from: the folder’s, the board’s target branch or a branch of your choice, which becomes their base and that of their PR. “+ New agent” can choose another one for an agent, including for a branch you create. Thanks to Metalzoid on Discord.

### Fixes

- Settings › Harness no longer talks about the tested versions: a newer version of a harness shows like the others, with no warning. Only what keeps it from starting is told, like a version too old and the command that updates it.
- An agent without an Escouade worktree that works in a worktree of its own (made by Claude Code, or one it went into with a `cd`) now follows the pull requests of that branch: its header shows its branch and “Open a PR” starts from there, never from the branch of the project’s folder nor from another agent’s. Thanks to Metalzoid on Discord.
- An agent stopped by “OAuth session expired and could not be refreshed” while its account is signed in goes on by itself. If the account really must sign in again, its card says so and the agent goes on once signed in.

## [1.8.11] — 2026-10-23

### New

- A new agent’s worktree is created at its first message: the agent is named first, then its branch and its folder take that name, and its setup runs in that folder. A setup script that uses it for its local address or its database gets the right name. A per-project setting, on everywhere, gives back the old behavior. Thanks to MrKayJayDee on Discord.

## [1.8.10] — 2026-10-23

### New

- A ticket has “Account”, “Model” and “Effort” next to “Harness”: its agent always starts on what you choose, and a ticket set on an account stays there, waiting for its reset rather than switching accounts. By default, “Auto” says what it will give: the current account, the Kanban’s model and effort.
- “New agent on a branch…” also creates the branch you type, from where a new agent starts, and the worktree’s folder is named after the branch (`feat/signup` gives `signup`): a setup script can use it for its local address or its database. Renaming an agent renames its branch too, unless it is pushed already, has a pull request or was there before the agent. Thanks to MrKayJayDee on Discord.
- Through the MCP server, an agent now acts on every project that is not private, not only its own: your back end’s agent can create, edit and start your front end’s tickets, and write to its agents. A private project stays reserved to its own agents. Thanks to Cédric on Discord.

### Fixes

- The `.escouade.yml` Escouade writes passes Prettier: its lists are indented the same way at the root as in a group, and its values take quotes only where YAML requires them.
- Claude Code’s commands (`/compact`, `/cost`, `/context`, `! git status`…) and the wake-up of a `/loop` no longer show with their tags in the conversation: the command as typed, its output under it, its error in red, in conversations saved before too.

## [1.8.9] — 2026-10-22

### New

- The status bar tells where each quota will end at its reset if you keep your pace (“→ 64 %”), and OpenCode’s day cost at midnight: you see in time when to slow down. A setting in Settings › Accounts turns it off. Thanks to @epommereuil (#12).
- A subagent shows the model it really runs on, even when its call names none, with every harness. Thanks to @epommereuil (#14).
- Agents can start from the Kanban’s target branch rather than the folder’s (a box in Settings › Kanban), and the menu next to “New agent” makes one without a worktree, or in a worktree. Thanks to @Metalzoid, whose PR #6 started the idea.
- Settings › Integrations has a “Code hosts” section: connect GitHub, GitLab, Bitbucket or Gitea / Forgejo, several instances each, and see whether a token has expired or which permissions it lacks. Each project finds its account there through its remote, and the github.com account is GitHub Issues’ one too.
- Escouade can read and drive GitHub pull requests (github.com, GHE.com or your company server): open, merge, checks, logs, reruns, reviews and deployments, while sparing your API quota.
- An agent opens its pull request in one click, from its conversation’s header, its card’s menu or the branch picker: its branch is pushed, and a title and description are proposed for you to review. Its card then follows the pull request and its CI, and “Agent fixes it” or “Agent addresses the feedback” sends it the failed checks or the comments, only when you click. Thanks to @Metalzoid, whose PR #7 started the idea.
- An agent whose turn is over but that is still waiting on something no longer says “Done”: its card says “Awaiting”, or the time Claude Code will wake it up, and its tooltip lists what goes on. The agents it drives through the MCP server are listed under its card while they work.
- A project can be private (Settings › Projects): the agents of the other projects no longer see it through the MCP server, and its own see it alone. You and Claude outside Escouade still see everything; a setting makes every new project private.
- Agents no longer read a private project’s files either (folder, worktrees, Escouade’s data, harness sessions), nor its own agents those of the others. The project’s settings say, harness by harness, whether it is a guard or a real barrier.
- The worktree of a done ticket, whatever the approval, or of a deleted one goes on its own, without ever losing work: if it holds uncommitted changes or commits neither merged nor pushed, it stays, and you’re told. “Clean up” in Settings › Projects tidies up those of archived agents.
- Nothing of a private project goes to another through Escouade either: the “New project” assistant doesn’t offer to copy its setup, and says so, and what Escouade writes to the agents never names another project.
- A private project offers to reserve it an account, so that the harness’s sessions, memory and history are shared with no other project, and flags an account still shared. Its agents no longer switch to a shared account by themselves at the usage limit, unless you allow it.
- A `.escouade.yml` in the repository shares the project’s settings with your team (worktrees, launch, Git, Kanban): Escouade uses it as soon as the project opens and reads it again when it changes, but runs none of its commands without your approval. “Export to .escouade.yml” writes it from a project that is already set up.
- “Switch accounts at” (Settings › Accounts) moves an agent to another account when its own reaches 80, 90 or 95% of its quota, at the end of its turn, never in the middle of its work; the conversation and a notification say so. By default, “At the limit”: nothing changes.

### Fixes

- An agent archived right after it was created no longer leaves a process running that held its folder: the worktree of a deleted or validated ticket goes instead of staying, sometimes minutes later.
- A second Claude Code account no longer runs as the first one’s identity when your environment sets `ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN` or `ANTHROPIC_AUTH_TOKEN`: its quota finally moves on claude.ai.
- When an account goes on in extra usage at 100 % of its quota, the status bar says so: its running agents stay on it, only new ones go to the other account.
- With live transcription on Deepgram, a sentence whose stream breaks is no longer lost: it goes whole, as without live transcription.
- Local dictation and voice conversation (Whisper) are much faster, above all on Windows: a sentence is transcribed there in 2.5 s with large-v3-turbo instead of over a minute, and in a quarter of a second with base.
- An agent that scheduled a turn for later (`/loop`, a scheduled task) is no longer stopped for being idle before that time: its wake-up happened in the stopped process, so never.

## [1.8.8] — 2026-10-20

### Fixes

- With live transcription, the voice conversation no longer cuts the starts and ends of your sentences, nor the words said more softly after a pause: the whole sentence goes to the service, and its text comes a moment later.

## [1.8.7] — 2026-10-20

### New

- With online transcription, the text writes itself as you speak, in dictation as in the voice conversation, and the sentence is ready a moment after your last word. A box in Settings › Voice turns it off for each service.

## [1.8.6] — 2026-10-20

### Fixes

- With online transcription, the voice conversation also sends what you say to the chosen service (ElevenLabs…), as dictation does: on Windows, each sentence no longer takes tens of seconds to be understood.

## [1.8.5] — 2026-10-19

### Fixes

- Stopping an agent can no longer cut an update of Claude Code half-way, which could make the `claude` command disappear: Escouade updates it itself, every day (a switch in Settings › Harness). A Claude Code that cannot be found is reported with the command that reinstalls it.

## [1.8.4] — 2026-10-19

### Fixes

- With an online voice, Settings › Voice no longer shows Piper's settings: a single line says which local voice takes over if the service does not answer.

## [1.8.3] — 2026-10-19

### New

- The voice conversation can be read by an online service (OpenAI, ElevenLabs, Mistral, Deepgram, Groq), with the same key as its dictation, and the voice of your choice, your account's own included. For dictation as for the voice, you also choose each service's model.

## [1.8.2] — 2026-10-19

### New

- In a voice conversation, the agent asks its questions aloud, one at a time, even when it has several. The choices show on screen: you answer by saying the choice, its number or your own answer.

### Changes

- Settings › Harness no longer has a "Sign in…" button: each account signs in from Settings › Accounts.
