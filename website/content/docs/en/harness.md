# The harnesses

A harness is the program that runs an agent: Claude Code, Codex, OpenCode, GitHub Copilot CLI, Cursor CLI, Mistral Vibe, Antigravity CLI or Gemini CLI. This page compares them, then tells for each one how to install it, its account, its models and modes, its cost, its terminal view and what it doesn’t do.

> [!NOTE]
> Codex, OpenCode and Copilot CLI have been there since Escouade 1.8, Cursor CLI since 2.0.1, Mistral Vibe, Antigravity CLI and Gemini CLI since the version after 2.0.5; Claude Code from the start.

## Choosing a harness

Escouade starts the harness on your machine, with your own account: install at least one and sign in to it once, as you would to use it in a terminal. Settings › Accounts and harnesses lists each one with its executable, its version and its state (see [the harness settings](settings-claude-code)).

Then add it an account in Settings › Accounts and harnesses: no harness has one to begin with, and nothing starts on a harness without one. “Add an account…” offers the account already signed in on the machine, or a new one (see [multiple accounts](accounts#no-account-to-begin-with)). The model, effort and mode of new agents are set on each account’s card, with its harness’s.

As soon as two harnesses are installed:

- each project has its “Default harness” (Settings › Projects › Project), Claude Code when unset;
- the arrow of “New agent”, or `Ctrl+Shift+N`, opens the “Harness” menu, which numbers them (see [the interface](interface#the-list-of-agents));
- a ticket’s window has its “Harness” field, the project’s by default, which the autopilot follows;
- each agent’s card bears the mark of its harness, as soon as agents run on two harnesses.

An agent keeps its harness. When no account of its own can take it anymore, it can [continue on another harness](accounts#continue-on-another-harness), with its conversation, its worktree and its branch.

Whatever the harness, the agent has the same conversation: the answer and the reasoning live, its tools under the same names (`Bash`, `Edit` with its diff, `Read`, `Task`…), the same permission, question and plan-to-approve cards, the “Plan” banner with its tasks and subagents, resuming after a restart and “Duplicate the conversation”. What a harness can’t do doesn’t show for its agents.

## Comparison

| | Claude Code | Codex | OpenCode | Copilot CLI | Cursor CLI | Mistral Vibe | Antigravity CLI | Gemini CLI |
|---|---|---|---|---|---| --- | --- | --- |
| Installation | Claude Code’s script | `npm install -g @openai/codex` | `npm install -g opencode-ai` | `npm install -g @github/copilot` | Cursor’s script | `uv tool install mistral-vibe` | Antigravity’s script | `npm install -g @google/gemini-cli` |
| Tested with | 2.1.283 to 2.1.301 | 0.163.0 | 1.19.3 | 1.0.98 | 2026.10.22 | 3.0.3 | 1.4.9 | 0.66.0, without an account (not yet tried for real) |
| Account | Claude subscription or API key | ChatGPT account | the providers connected to OpenCode, local models included | GitHub account with Copilot, Free included | Cursor account (`agent login`, or an API key), Free included | Mistral API key (`vibe --setup`) | Google account (its interface’s sign-in) | Google account (its interface’s sign-in, in the account’s folder) or Gemini API key |
| Several accounts | yes | yes | no | yes | yes, by API key | yes, by API key | no | yes |
| Models | those Claude Code lists for each account | those Codex lists for your account | your providers’, with search and favorites | those Copilot lists, with their multiplier | your account’s, Auto first | its session’s | those `agy models` lists, Gemini’s effort apart | its session’s, Auto first |
| Modes | all five | all five | four (no Auto) | all five | four (no Auto) | four (no Auto; Plan played by Escouade) | four (no Auto) | four (no Auto) |
| What protects | its permissions | its sandbox | its permission rules | Escouade’s rules | its permissions, for its commands | its permissions; Escouade’s rules | its rules, written by Escouade; asks once the turn is over | its permissions; its rules for a private project |
| Message during a turn | waits for the end of the turn | enters the turn | waits for the end of the turn | enters the turn | waits for the end of the turn | waits for the end of the turn | waits for the end of the turn | waits for the end of the turn |
| Cost | exact | estimated | exact (0 when local) | exact, in AI credits | not told | exact | not given (tokens) | not given (tokens) |
| Quota | 5-hour and 7-day windows | 5-hour and 7-day windows (30 days on the free plan) | none: daily budget | monthly credits | a link to its dashboard | none: daily budget, a link to Mistral’s console | not read | not read: a link to its usage page |
| Terminal view | “Claude Code view” | “Codex view” | “OpenCode view” | “Copilot CLI view” | “Cursor CLI view”, limited following | “Mistral Vibe view” | “Antigravity CLI view” | “Gemini CLI view” |
| Attached PDFs | yes | no | no | no | no | no | no | no |
| Remote control, background tasks | yes | no | no | no | no | no | no | no |

## Claude Code

### Installation and account

Install Claude Code with its script (`irm https://claude.ai/install.ps1 | iex` on Windows, `curl -fsSL https://claude.ai/install.sh | bash` on macOS and Linux), then sign in with your Claude subscription or an API key. Escouade finds it in the `PATH` or in the usual places, even in WSL (see [installation](installation)). Its accounts each have their folder (`CLAUDE_CONFIG_DIR`): see [several accounts](accounts).

### Models and modes

The models are those the Claude Code of each account lists in its `/model`, with their name, their efforts and whether each takes the “Auto” mode: a model one account has more (Mythos, say) shows in its agents’ Composer, a model it lacks does not. Escouade reads them at startup and when an account is added, without using anything, then at each agent start. The Kanban and the tickets offer those of the active accounts. An agent that moves to an account without its model takes the default one, and its conversation says so. Before the first reading, the list is Fable, Opus, Sonnet and Haiku. The five modes have their original meaning: “Auto” (a classifier approves safe actions), “Ask”, “Plan”, “Accept edits” and “Bypass” (see [the permission modes](managing-agents#permission-modes)). Claude Code’s permissions are what protects.

### Cost, quota and view

A turn’s cost is the one Claude Code gives. The quota is your account’s 5-hour window and weekly window, in the status bar. The [Claude Code view](claude-code-view) runs the agent in Claude Code’s own interface. It alone has [remote control](remote-control), background tasks and reading PDFs.

Claude Code is a plugin shipped with Escouade (`dev.escouade.claude-code`): its agents talk through a program of its own that Escouade starts and watches. It shows in the Plugins view, under “Shipped with Escouade”; off, a Claude Code agent says at its start that it must be turned back on, and keeps its session.

## Codex

Codex is a plugin shipped with Escouade (`dev.escouade.codex`): its agents talk through a program of its own that Escouade starts and watches. It shows in the Plugins view, under “Shipped with Escouade”; turned off, a Codex agent says at its start that it needs turning back on, and keeps its session.

### Installation and account

`npm install -g @openai/codex`, then `codex login` with your ChatGPT account. Escouade starts the native binary of the npm package (on Windows, the package’s `codex.exe`, not its launcher): Codex’s desktop app often installs an older version, which the `PATH` may find first. Each agent has its `codex app-server`. Its accounts each have their `CODEX_HOME`: see [Codex accounts](accounts#codex-accounts).

### Models and modes

The models and their efforts are those Codex lists for each account: Escouade reads them at startup and when an account is added, without starting an agent or using anything, keeps them from one start to the next and reads them again when the “Model” menu opens on a list more than a day old. Before the first reading, the menu offers GPT-6-Luna alone, marked “list being read”. The model the account’s `config.toml` picks (`model = …`) is there too, marked “from config.toml”, even when Codex doesn’t list it. The modes go through Codex’s sandbox:

| Mode | With Codex |
|---|---|
| Plan | Codex’s Plan mode: it studies, may ask you questions, then proposes its plan, without changing anything (read-only sandbox). |
| Ask | Sandbox: writing in the agent’s folder, network off. A command that leaves it asks, `git add` and `git commit` included (Codex protects the whole `.git` folder). |
| Accept edits | The same, and Escouade accepts at once `git add` and `git commit` in the agent’s worktree, when the command is only that and its hooks are the repository’s; the network and the rest ask. |
| Auto | Codex’s automatic reviewer decides on the requests for you, and the conversation says what it approved. |
| Bypass | Codex asks nothing and leaves its sandbox. |

A request from Codex has three buttons: “Allow”, “For this session” and “Deny”. A message sent while it works enters its turn instead of waiting.

### Cost, quota and view

Codex doesn’t give the cost: Escouade estimates it from the turn’s tokens and OpenAI’s public prices, and writes it “≈ … estimated”. The quota is your account’s windows (5 hours and 7 days, or a single 30-day one on the free plan). The “Codex view” runs the agent in Codex’s interface (see [the terminal view](claude-code-view#with-codex)).

### What Codex doesn’t do

It doesn’t read PDFs (the button refuses them and says so) and offers only the `/compact` command in Escouade, which compacts its conversation’s context. A Codex agent doesn’t resume on another account: it waits for the reset, or continues on another harness. No remote control or background tasks.

## OpenCode

OpenCode is a plugin shipped with Escouade (`dev.escouade.opencode`): its agents talk through a program of its own that Escouade starts and watches, on their account’s server. It shows in the Plugins view, under “Shipped with Escouade”. If it crashes, the server and the turn go on and the conversation says so; turned off, an OpenCode agent says it needs turning back on, and keeps its session.

### Installation and account

`npm install -g opencode-ai` installs OpenCode 1, the only one Escouade drives: OpenCode 2 (`@opencode/cli`) is recognized and says “Not supported”. Connect a provider with `opencode auth login` (“Sign in…” in Settings › Accounts and harnesses runs it); Ollama’s local models work too. OpenCode has a single account in Escouade, its usual config. A single `opencode serve` serves all OpenCode agents, protected by a random password that never leaves Escouade; each agent is a session on it.

### Models and modes

Its models, like those of Codex, Copilot CLI and Cursor CLI, are read at startup and when an account is added, without starting an agent, kept from one start to the next and read again when the menu opens on a list more than a day old. The “Model” menu lists the models of the connected providers, with a search, their provider and favorites (★) first, also set in Settings › Accounts and harnesses › OpenCode › “Models”. The effort is the model’s variant when it has one. Escouade always gives the model, so that OpenCode doesn’t pick one by itself.

OpenCode has no sandbox: its permission rules are what follows the mode, and an accepted command can do anything.

| Mode | With OpenCode |
|---|---|
| Ask | The files of the agent’s folder change without asking; commands (`git add` and `git commit` included), the web and paths outside the folder ask. |
| Accept edits | The same, and `git add` and `git commit` go through without asking. |
| Plan | OpenCode’s `plan` agent studies without changing anything, then proposes its plan for you to approve. |
| Bypass | OpenCode asks nothing. |

A request from OpenCode has “Allow”, “Always allow” (the card says the pattern OpenCode will keep for the session, like `git add *`) and “Deny”. A message sent during a turn waits for its end.

### Cost, budget and view

A turn’s cost is the one OpenCode gives, at the provider’s price: 0 with a local model. OpenCode has no quota: the status bar shows its account’s cost for the day, and “Daily budget ($)”, in Settings › Accounts and harnesses, pauses the autopilot until midnight once reached (see [the OpenCode account](accounts#opencode-account)). The “OpenCode view” plugs OpenCode’s interface into the agent’s session without stopping Escouade’s tracking (see [the terminal view](claude-code-view#with-opencode)).

### What OpenCode doesn’t do

No “Auto” mode (OpenCode has no reviewer), no sandbox, a single account, no attached PDFs, no `/` command in Escouade but `/compact`, no remote control or background tasks.

## Copilot CLI

Copilot CLI is a plugin shipped with Escouade (`dev.escouade.copilot-cli`): its agents talk through a program of its own that Escouade starts and watches. It shows in the Plugins view, under “Shipped with Escouade”; turned off, a Copilot agent says at its start that it needs turning back on, and keeps its session.

### Installation and account

`npm install -g @github/copilot`, then `copilot login` with a GitHub account that has Copilot (Free included). Each agent has its `copilot --server --stdio`, without the GitHub MCP server Copilot plugs in by itself, without access to the system’s temporary folder and without the GitHub tokens in its commands’ environment. Its accounts each have their `COPILOT_HOME`: see [Copilot CLI accounts](accounts#copilot-cli-accounts).

### Models and modes

The models menu writes each one’s multiplier (`×3`), or its relative cost on a Free account; `auto` is never offered. The model and the mode change at once, even during a turn.

Copilot has no sandbox when Escouade drives it: Escouade’s rules answer the safe requests without a card.

| Mode | With Copilot CLI |
|---|---|
| Ask | Reads and writes in the agent’s folder go through; commands (`git add` and `git commit` included), the web, MCP tools and paths outside the folder ask. |
| Accept edits | The same, and `git add` and `git commit` of its worktree go through. |
| Plan | Copilot studies and proposes a plan without changing anything; its reads of the folder go through. |
| Auto | Copilot goes on by itself to the end of its task (autopilot), with the rules of “Accept edits”. |
| Bypass | Copilot asks nothing. |

A request from Copilot has “Allow”, “For this session” and “Deny”; no “Always”, which would write a lasting rule into Copilot’s config. A form from an MCP server becomes a card with fields. A message sent during a turn enters it at once.

### Cost, credits and view

A turn’s cost comes from the AI credits of its calls (one credit is $0.01). The “credits” bar says what is used this month, until they come back on the 1st of the month; a session’s credit cap, once reached, becomes a card. The “Copilot CLI view” runs the agent in Copilot’s interface (see [the terminal view](claude-code-view#with-copilot-cli)).

### What Copilot CLI doesn’t do

No “Always allow”, no `assisted` mode (experimental), no attached PDFs, no `/` command in Escouade but `/compact` (followed, if you like, by what to keep), no remote control or background tasks. A Copilot agent doesn’t resume on another account.

## Cursor CLI

Cursor CLI is a plugin shipped with Escouade (`dev.escouade.cursor-cli`): its agents talk through a program of its own that Escouade starts and watches. It shows in the Plugins view, under “Shipped with Escouade”; turned off, a Cursor agent says at its start that it needs turning back on, and keeps its session.

### Installation and account

Install Cursor CLI with its script (`irm 'https://cursor.com/install?win32=true' | iex` on Windows, `curl https://cursor.com/install -fsS | bash` on macOS and Linux), then sign it in with `agent login`. Escouade finds it in `%LOCALAPPDATA%\cursor-agent` on Windows, even when it isn’t on the `PATH`, and starts its latest installed version. Each agent has its own `agent acp` and a configuration folder of its own, removed with it: its rules, its model and its sessions are kept there, never in your Cursor config. The machine’s sign-in (`agent login`) is “the machine’s account”. Another Cursor account is added by its API key (cursor.com, Settings › API Keys) in Settings › Accounts and harnesses: it’s kept in the system keychain and given to its agents by `CURSOR_API_KEY`, never in a file nor on a command line; “Change the key…” gives it again.

### Models and modes

The “Model” menu lists your account’s models, Auto first, with their efforts. A Free plan runs Auto only: another model ends the turn in error, saying so. The model and the mode change at once.

Cursor has no sandbox on Windows. It reads and edits files without asking, even outside the agent’s folder, and without going through Escouade, which therefore can’t stop it; its commands, the web and MCP tools ask. For a private project, Escouade refuses right away its commands that name the other projects’ paths, but not its reads nor its writes.

| Mode | With Cursor CLI |
|---|---|
| Ask | Cursor’s Agent mode: its commands, the web and MCP tools ask; a write it submits goes through in the agent’s folder, and asks elsewhere, with its diff. |
| Auto edits | The same, and `git add` and `git commit` in the agent’s folder go through. |
| Plan | Cursor’s Plan mode: it studies and proposes its plan to approve, without changing anything. |
| Bypass | Escouade accepts all Cursor asks. |

A request of Cursor has “Allow”, “For this agent” and “Deny”: “For this agent” keeps the rule (`Shell(git status)`, say) in the agent’s folder, for it alone and its resumes. Its questions, its plan, its tasks and its subagents come into the conversation and the “Plan” banner. A message sent during a turn waits for its end: Cursor would stop the running turn for it.

The Claude Code skills, commands, agents and hooks of your account and of the project run under Cursor too, as for Claude Code agents; Escouade leaves them as you set them. A hook that refuses a write blocks it: the tool ends failed in the conversation, “Cursor wrote nothing”, as Cursor doesn’t give the reason.

### Cost, quota and what Cursor doesn’t do

Cursor tells neither the tokens nor the cost of a turn when Escouade drives it: the conversation shows none, and the statistics count its turns and their length. Nor does it tell its usage of the month: no bar in the status bar, but “Usage on cursor.com” in Settings › Accounts and harnesses and in the quota panel, which opens its dashboard. A limit reached (“Upgrade your plan to continue”) stops the turn on an amber card that says so, with, when there is one, the account or harness to go on with. No “Duplicate the conversation” (Cursor doesn’t copy a session), no attached PDFs, no `/` commands in Escouade (to compact its context, type `/summarize` in its terminal view), no remote control or background tasks. It gets Escouade’s MCP server through its session; the switch doesn’t declare it in Cursor’s config yet. The “Cursor CLI view” runs the agent in Cursor’s interface, with a limited following (see [the terminal view](claude-code-view#with-cursor-cli)).

## Mistral Vibe

Mistral Vibe is served by the “Mistral” plugin shipped with Escouade (`dev.escouade.mistral`), which also carries its voice: its agents talk through a program of its own that Escouade starts and watches. It shows in the Plugins view, under “Shipped with Escouade”; turned off, the Mistral voice goes too, and a Vibe agent says at its start that it needs turning back on, and keeps its session.

### Installation and account

Install Mistral Vibe with `uv tool install mistral-vibe`, then give it your Mistral API key with `vibe --setup`: it is kept in the system’s keychain, and Escouade never reads it. Escouade finds its `vibe-acp` on the `PATH`, or in `~/.local/bin` where `uv` puts it. Each agent has its `vibe-acp` and its own `VIBE_HOME` folder, removed with it: its rules, its sessions and the trust of its folder are kept there, never in your `~/.vibe`. Its updates and its telemetry are turned off there.

The key of `vibe --setup` is “the machine’s account”. Another Mistral account is added by its API key (console.mistral.ai, API Keys) in Settings › Accounts and harnesses, or by the Mistral key of Settings › Voice, leaving the field empty: the voice and Vibe share the Mistral account. The key is kept in the system’s keychain and given to its agents by `MISTRAL_API_KEY`, never in a file nor on a command line; “Change the key…” gives it again. An account without its key starts nothing. “Resume on <account>” keeps the agent’s session: only the key changes.

### Models and modes

The “Model” menu lists the models its session offers. Its thinking is the effort: “Low” turns it off, “High” keeps it. The model, effort and mode change at once.

Vibe has no sandbox. It reads without asking; its commands off its list (`cat`, `ls`, `git status`… go through), its writes and MCP tools ask.

| Mode | With Mistral Vibe |
|---|---|
| Ask | Its `ask` agent: its writes go through in the agent’s folder and ask elsewhere, with their diff; its commands off its list and MCP tools ask. |
| Accept edits | Its `accept-edits` agent: its writes go through without asking, even outside the folder. |
| Plan | Vibe has none: Escouade asks it to study and propose a plan, refuses its writes, and its last words come as a plan to approve; approved, Vibe carries it out in the same turn. |
| Bypass | Its `auto-approve` agent: it does everything without asking. |

A request of Vibe has “Allow”, “For this agent” and “Deny”: “For this agent” keeps the rule in the agent’s `VIBE_HOME`, for it alone and its resumes. A refusal stops the turn: Vibe would otherwise try the same command again under other forms. Its questions, its tasks and its subagents go to the conversation and the “Plan” banner. A message sent during a turn waits for its end. “Duplicate the conversation” copies its whole session.

For a private project, the other projects’ paths are refused to its read and write tools by the agent’s config, and Escouade refuses right away what it asks about them. In Bypass, Vibe then stays in its `ask` agent, whose rest Escouade accepts, so that these refusals hold: its `auto-approve` would override them.

### Cost, quota and what Vibe doesn’t do

Vibe tells the cost and tokens of each turn: the conversation, the agent’s card, the status bar and the statistics show them, as for OpenCode. It tells no quota (a Mistral API key is billed by use): the day’s cost and a “Daily budget ($)” stand in for it, and “Usage on admin.mistral.ai”, in Settings › Accounts and harnesses and in the quota panel, opens Mistral’s console. Its rate limit (“Rate limit exceeded”, after five minutes of Vibe’s retries) stops the turn on an amber card; the agent goes on by itself a minute later, or, for a ticket’s agent, on another Vibe account when “Resume a ticket’s agent stopped by the usage limit on another account” is on. The texts written, when “Model for written texts” is “Mistral Vibe”, go through `vibe -p`, without any tool, in one turn and 0.05 $ at most. No Auto mode, attached PDFs, `/` commands in Escouade (`/compact` goes as a message), remote control or background tasks. It gets Escouade’s MCP server through its session; the switch doesn’t declare it in Vibe’s config yet. The “Mistral Vibe view” runs the agent in Vibe’s interface, on the same session (see [the terminal view](claude-code-view#with-mistral-vibe)).

## Antigravity CLI

Antigravity CLI is a plugin shipped with Escouade (`dev.escouade.antigravity-cli`): its agents talk through a program of its own that Escouade starts and watches. It shows in the Plugins view, under “Shipped with Escouade”; turned off, an Antigravity CLI agent says at its start that it has to be turned back on, and keeps its conversation.

### Installation and account

Install Antigravity CLI with its script (`irm https://antigravity.google/cli/install.ps1 | iex` on Windows, `curl -fsSL https://antigravity.google/cli/install.sh | bash` elsewhere), then add its account in Settings › Accounts and harnesses (see below). Escouade finds `agy` in the `PATH`, or where its script puts it. Each agent has its own folder, removed with it, which stands as `agy`’s home: its rules, its conversations and Escouade’s MCP server are kept there, never in your `~/.gemini`, which Gemini CLI shares. Its updates are off.

Its machine’s account is your Google sign-in: “Add an Antigravity CLI account…” offers it, signed in or not. Not signed in yet, “Add this account” adds it, then runs `agy` in a terminal of the window, which opens your browser; Escouade never sees your credentials, only that the system keychain keeps the sign-in. “Sign in…”, on its row, does the same again. A `GEMINI_API_KEY` of your machine isn’t given to it.

A second Google account isn’t possible: `agy`’s sign-in is the machine’s, whatever the folder. “A new account” is therefore a Gemini API key (on aistudio.google.com, “Get API key”): kept in the system keychain, given to its agents by `GEMINI_API_KEY`, never written in a file nor on a command line. Its agents then go through that key’s Gemini API, with its models only. “Change the key…” gives it again. Each account has its defaults (model, effort, mode), as the other harnesses’ do.

### Models and modes

The “Model” menu lists your account’s models (`agy models`); for Gemini, the effort (Low, Medium, High) is its “Effort” menu. A change of model, effort or mode takes effect at the next message: Escouade starts `agy` again on the same conversation.

Antigravity asks nothing during a turn when Escouade drives it: what its rules don’t allow is refused on the spot. Escouade asks you once the turn is over (“Allow”, “For this agent”, “Deny”), writes the rule, then the agent goes on and does it again, in the same turn. Its questions come the same way, at the end of the turn, and your answer is sent to it. It reads without asking. No sandbox.

| Mode | With Antigravity CLI |
|---|---|
| Ask | Its writes go through in the agent’s folder; its commands, its writes elsewhere and MCP tools are asked at the end of the turn. |
| Auto edits | Its `accept-edits` mode; `git add` and `git commit` go through too. |
| Plan | Its `plan` mode: it studies and writes a plan without changing anything, which comes as a plan to approve; approved, Antigravity carries it out. |
| Bypass | Escouade’s rules allow it everything, without asking. |

“For this agent” keeps the rule for it alone and its resumes. Its subagents go to the conversation and the “Plan” banner. A message sent during a turn waits for its end. “Stop” stops the turn; the next message resumes the conversation.

For a private project, reading the other projects’ paths is refused to it by its rules (which refuses writing too), even in Bypass, and what it asks about them is refused right away. Its commands that would read them aren’t guarded.

### Cost, quota and what Antigravity doesn’t do

Antigravity gives each turn’s tokens (on its card, in the statistics), not their cost nor its quota: its `/usage` (by group of models, weekly) is only in its interface; “Usage on antigravity.google”, in Settings › Accounts and harnesses and the quota panel, opens the page of its plans. When its quota is spent, the turn ends “limited” on a card that says so; the agent tries again by itself at the reset Antigravity tells, else an hour later (a minute when the model’s capacity is what’s missing), or moves to another account as “Switch accounts at” and “Continue on the next harness” say. No Auto mode, attached images or PDFs, “Duplicate the conversation”, `/` commands in Escouade, remote control or background tasks. It gets Escouade’s MCP server through its folder’s config; the switch doesn’t declare it in yours yet. The “Antigravity CLI view” runs the agent in Antigravity’s interface, on the same conversation (see [the terminal view](claude-code-view#with-antigravity-cli)).

## Gemini CLI

Gemini CLI is a plugin shipped with Escouade (`dev.escouade.gemini-cli`): its agents talk through a program of its own that Escouade starts and watches. It shows in the Plugins view, under “Shipped with Escouade”; turned off, a Gemini agent says at its start that it needs turning back on, and keeps its session.

> [!WARNING]
> Gemini CLI hasn’t been tried with a real account yet: its adapter follows its documentation and what its program does against a fake server. What is left to see for real: the size of its live answers, its questions, its quotas and their messages, its Google sign-in given to each agent, its terminal view and its hooks.

### Installation and account

Install Gemini CLI with npm (`npm install -g @google/gemini-cli`, Node 20 or later) or Homebrew (`brew install gemini-cli`), then add its account in Settings › Accounts and harnesses (see below). Escouade finds `gemini` on the `PATH`, or in npm’s global folder. Each agent has its own configuration folder (`GEMINI_CLI_HOME`), removed with it: its settings, its sessions and its rules are kept there, never in your `~/.gemini`, which Antigravity CLI shares. Escouade turns off its updates and usage statistics there, and trusts the agent’s folder (without it, Gemini would leave out Escouade’s MCP server).

A Gemini account is one or the other, as you choose when you add it:

- **A Google account.** Escouade runs Gemini’s interface (`gemini`) in a terminal of the window, with the account’s folder as its `GEMINI_CLI_HOME`: it opens your browser on your Google account, and keeps its sign-in in that folder. Escouade never reads it: it only sees that it is there, with the account’s address. Its agents get it by `GOOGLE_APPLICATION_CREDENTIALS`, from their own folder. “Sign in again…” makes it again.
- **A Gemini API key** (on aistudio.google.com, “Get API key”): kept in the system keychain, given to its agents by `GEMINI_API_KEY`, never written in a file nor on a command line. “Change the key…” gives it again.

A key or Vertex AI of the machine (`GEMINI_API_KEY`, `GOOGLE_API_KEY`, `GOOGLE_GENAI_USE_VERTEXAI`) never goes to an added account. The machine’s account is offered only when the machine has such a key: the Google sign-in of your `~/.gemini` is never given to the agents. Each account has its defaults (model, mode), as the other harnesses’ do.

### Models and modes

The “Model” menu lists its session’s models, Auto (its router, which picks Pro or Flash at each turn) first. It has no “Effort” menu: its thinking isn’t set along the way.

| Mode | In Gemini CLI |
|---|---|
| Ask | Its `default` mode: it reads without asking; its writes in the agent’s folder go by Escouade’s rules, elsewhere they are asked with their diff; its commands and MCP tools are asked; its subagents are refused. |
| Auto edits | Its `autoEdit` mode: its writes go through without asking. |
| Plan | Its `plan` mode: it studies without changing anything and writes its plan, which comes as a plan to approve; approved, Gemini carries it out. |
| Bypass | Its `yolo` mode: it does everything without asking, subagents included. |

“For this agent” holds for its session. Refusing stops the turn; your word, if any, is sent to it after. Gemini has no tool to ask a question when Escouade drives it: Escouade tells it to end its turn on its question, which comes as a card (in the window as in the list), and your answer is sent to it. It has no task list. A message sent during a turn waits for its end. “Stop” stops the turn; the conversation resumes after a restart. No sandbox.

For a private project, its file tools and its commands that name the other projects’ paths are refused by its rules (`--policy`), even in Bypass, and what it asks about them is refused right away. A script that opens a file itself isn’t guarded.

### Cost, quota and what Gemini doesn’t do

Gemini gives each turn’s tokens, not their cost nor its quota: its usage is read by `/stats` in its interface, and “Usage on aistudio.google.com”, in Settings › Accounts and harnesses and the quota panel, opens the usage page of the API keys. Its limit ends the turn “limited” on a card that says so; the agent goes on by itself when Gemini says (“Please retry in 30s”), else a minute later, or an hour later for its daily quota, or moves to another account as “Switch accounts at” and “Go on with the next harness” say. The texts it writes (an agent’s name, a commit message…) go through `gemini -p`, every tool of it refused. The “Gemini CLI view” runs the agent in Gemini’s interface, on the same session (see [the terminal view](claude-code-view#with-gemini-cli)). No Auto mode, attached PDFs, “Duplicate the conversation”, `/` commands in Escouade (`/compact`: its `/compress`, in its interface), remote control nor background tasks. It gets Escouade’s MCP server by its session; the switch doesn’t declare it in your config yet.

## Other harnesses, by a plugin

An agent that speaks ACP (Goose, say) is described in a plugin’s manifest, without code: see [a described ACP harness](plugins/harnesses). The ACP plugin, shipped with Escouade (`dev.escouade.acp`), starts it and speaks to it; it is switched off in the Plugins view, and the agents of those harnesses then stay, marked “harness missing”, with their conversations. The harnesses of this page are still part of the app.

## The MCP server

Whatever their harness, Escouade’s agents get its MCP server, each with its own token, when “Agents can drive Escouade” is on. The same switch declares it in each installed harness, so that Claude Code, Codex, OpenCode or Copilot CLI started outside Escouade, in a terminal or their app, drive Escouade: see [the MCP server](mcp).

## Written texts

A new agent’s name, the suggested commit messages and pull request descriptions are written by the harness of Settings › Agents › “Model for written texts”: “Claude Haiku”, “Codex (GPT-6-Luna)”, an OpenCode model , “Copilot (GPT-5 mini)”, “Cursor (Auto)”, “Mistral Vibe”, “Gemini 3.8 Flash” (Antigravity CLI) or “Gemini CLI”, among the installed harnesses (see [written texts](settings-application#written-texts)).
