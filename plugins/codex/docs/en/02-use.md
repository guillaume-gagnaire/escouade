# Use

## Start an agent

Choose “Codex” in the “Harness” menu of “New agent”, or make it a project’s default harness (Settings › Projects › Project, “Default harness”). Each agent has its own `codex app-server`, on the first active Codex account that still has quota.

The models and their efforts are those Codex lists for the account; before the first reading, the menu offers GPT-6-Luna alone. The model the account’s `config.toml` chooses is there too. Codex takes model, effort and mode at the next turn.

A message sent while Codex works goes into its turn instead of waiting. Attached images go through; PDFs don’t. Only the `/compact` command is offered. Codex tells no cost: Escouade estimates it from the tokens and OpenAI’s public prices, and marks it “estimated”.

A Codex agent doesn’t resume on another account: at the limit, it waits for the reset, or goes on with another harness.

## The modes

The modes go through Codex’s sandbox. What Codex asks comes as a card; what its sandbox lets it do, it does without asking.

| Mode | In Codex |
|---|---|
| Plan | Its Plan mode, sandbox read-only: it studies, may ask you questions, then puts its plan on a card. |
| Ask | Sandbox: writes in the agent’s folder, network off. A command that leaves it asks, `git add` and `git commit` included (Codex protects the `.git` folder). |
| Accept edits | The same, and the plugin accepts `git add` and `git commit` of the agent’s worktree at once (see the Permissions tab). |
| Auto | Codex’s automatic reviewer decides the requests for you, and the conversation says what it approved. `git add` and `git commit` go through as in Accept edits. |
| Bypass | Codex asks nothing and leaves its sandbox. |

A Codex card has “Allow”, “For this session” and “Deny”.

## The Codex view

The “Codex view” runs the agent in Codex’s interface, in a terminal of the window, on the same conversation. Its hooks are declared for that interface alone, by its command line: nothing is written in your `config.toml`. In this view, requests are answered in Codex’s interface, and the plugin accepts nothing at once.

## A discussion of the Docs tab

A discussion runs in the read-only sandbox of the Plan mode, whatever the mode: Codex writes nothing there.

## Private projects

For a private project, the plugin declares to Codex two permission profiles where the other projects’ paths are refused (`deny`), in place of its usual sandbox. Codex’s sandbox applies them to all its commands: a barrier, not a mere rule.

Its limits:

- in Bypass, Codex has no sandbox: the fence no longer holds for its commands;
- with a fence, a `git commit` that reads a file (`-F`, `--file`) goes to a card, even in Accept edits;
- Codex’s sessions and history are the account’s: so that a private project doesn’t share them, give it an account reserved to it alone.
