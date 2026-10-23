# Changelog

Escouade's visible changes, version by version, in English. The notes of each GitHub release and the English Discord channel take the section of its version; the French notes, those of the in-app update included, are in [CHANGELOG.md](CHANGELOG.md), which also holds the versions before 1.8.2.

To write an entry: one or two short sentences, for a human reader. What they can now see or do, and why it helps; the details (edge cases, settings, file names, tools, ticket numbers) go in the documentation, not here. Each entry of CHANGELOG.md has its English one here, under the same version and the same heading (`## [Unreleased]` for `## [Non publié]`).

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
