# Where does my data go?

What goes to a team server, encrypted, what never goes there, what stays on your machine, and why, without a team server, nothing changes.

## Without a team server

Nothing changes. Escouade keeps everything on your disk, in `~/.escouade/` (see [local data](data)), and your projects stay in their folders. Shared tickets travel through everyone’s git (see [sharing the Kanban](shared-board)). The team server only exists if your team installs it and you join it; as long as you are in no team, the app reaches no team server.

Usage statistics, if you accepted them, are a separate matter, unrelated to the team server: see the [Privacy](/en/privacy/) page.

## On a team server

The team server is installed by your team, on a machine it chooses (see [Install the team server](team/install)). Everything that goes there is end-to-end encrypted on your machine: the server keeps bytes it can’t read, and sees only metadata (who writes, when, how much), detailed in [the security officer’s guide](team/security#what-the-server-sees).

| What | On the server, encrypted | Stays on your machine | Today |
|---|---|---|---|
| The team | its members and their roles, each one’s devices, the invitations, the envelopes of the recovery key and of the escrow key | your devices’ keys, in the keychain; the name you give the team | there |
| The team’s settings | its team `.escouade.yml`, signed, and its history | your consent to sensitive keys and proposed commands, your device’s policies | there |
| The project list | for each project: its name, color, logo, repositories (their addresses), default branch, who has access | the project’s folder, its worktrees, its settings on this machine | there |
| The tickets of a Kanban shared with the team | key, title, description, criteria and their state, column, place, loops, progress, assignment, each device’s name | attached images and files, account, model, harness, agent, branch, cost; the “Do not share” tickets | there |
| Taking a ticket | which device holds it, until when, its token, the counter of the keys: in clear, without saying which ticket it is | | there |
| The spec, decisions, documents | the pages, the decisions, the uploaded files | the extracted text, the index, the vectors | there, as the team chooses |
| The shared memory | the memories and their signed provenance; an agent’s proposed until a human keeps them | the index and the vectors | there, with the docs |
| The costs | totals by day, once an hour: by project, harness and model, the tokens, the estimated cost, the highest quota of the day, and the total of each ticket by its key; encrypted for the administrators alone; your name only if the team chose “by member” | the detail of each turn, the prompts, the replies, the files, the tickets’ titles | there |

The spec, decisions, documents and memory go to the server by a gesture: “On the team server” in the project’s settings, or “Pour into the team” on the Kanban. The team can also keep them in the repository, as before. A project’s tickets go only if its Kanban is shared with the team.

## The team’s memory

The shared memory of a team’s project (see [the shared memory](shared-memory)) is read by every agent of the team: a memory there is an instruction that lasts. Escouade guards it from what a fooled agent, or a member who means harm, would slip in:

- **Each memory carries its provenance**, signed by the device that writes it: the member, the agent (its ticket, harness and model), the date, by hand or by an agent, and whether that agent had read a third party’s data (an issue, a PR, a web page). A memory whose provenance doesn’t match its signature is refused.
- **A memory an agent writes is born proposed**: only your agents read it, as data. It becomes the team’s once you keep it, on the card of the ticket to approve (“This ticket left 2 memories · Keep · Set aside”) or in Settings › Projects › Team › Shared memory. A memory written by hand is accepted at once. “Review the agents’ memories” is on by default; the team can turn it off, or impose it.
- **The agents read as data**, never as an instruction, a memory not approved or written after a third party’s data.
- **An administrator puts in quarantine** what a ticket’s agent wrote, or a member or one of their devices since a date, after a preview of what goes: the rule holds on every device, for what comes later too. The history stays in the team’s journal.
- **A secret is refused** at the writing: a token, a private key, a webhook’s address. An address the project doesn’t know is shown at its approval.

## The team’s costs

Your device works out what your agents spend on the team’s projects, and sends only **totals by day**, once an hour. Never a prompt, a reply, a file name, a ticket’s title or a turn.

- **Encrypted for the administrators alone**, by a key of their own, apart: the team server does not see the costs, and the other members do not read yours. An administrator who leaves reads nothing of what goes after.
- **By project**, by default: your totals go without your name, and the administrators see each project, never a person. The team may switch to **by member**: Escouade tells you so the next time you open it (“Your team sees your totals by day”), and your totals carry your name only once you read that screen. The server still knows which device writes, as for everything else.
- **You see what goes up of you** in the teams’ window, “Costs”: your totals by day, the team’s choice in one sentence, the last thing sent as it went, and what waits to be sent (offline, the totals wait on your device).
- A Claude Max or ChatGPT Pro account has no cost per turn: what goes is an **API equivalent**, marked as such.
- The team’s budgets are in its settings file: your device weighs them before a start (“The project api spent 92% of its budget for the month”).

## What never goes there

- **Harness accounts**: your sign-in to Claude Code, Codex and the others. The team server sees none of them; each member uses their own. Of your quotas, only the highest percentage of the day goes, in the totals of the costs, encrypted for the administrators.
- **Secrets and tokens**: those of connected services, code hosts, API keys. They stay in your machine’s keychain.
- **Agents’ conversations**: their logs stay in `~/.escouade/conversations/`.
- **The code**: it stays with your code host (GitHub, GitLab…). Cloning a team project goes through your own git accounts; the team server gives no git access and keeps no credential.
- **Your personal projects**: a project that is not a team project sends nothing there.

## What stays on your machine

In `~/.escouade/teams/`:

- `teams.json`: the teams of this device, the name you give them, their server, the state of the recovery key, an alarm;
- a folder per team, with its state (`state.sqlite3`), encrypted under a key from the keychain;
- in that folder, the copy of each team project this machine follows (`projects/<project>.sqlite3`), readable offline. It is not encrypted, like today’s local Kanban: your disk encryption protects it;
- `logos/`: the logos of the team’s projects.

Your devices’ keys are in the system keychain (the Windows Credential Manager, the macOS keychain). On Linux, where Escouade has no keychain, they go in a file only you can read, and the team’s card says so.

## Who can read what

- **The team’s members** read what belongs to the team, and the projects they have access to.
- **The server and its host** read none of the content; they see the activity.
- **The organization**, if the team has an escrow key, can read everything. Every member sees it on the team’s card.
- **A departed member** no longer reads anything written after they left, but keeps what they already read.

## Leaving a team

On the team’s card, “Remove from this device” erases its state and keys from this machine, after confirmation. What you wrote in the team stays there: it belongs to the team. An administrator then removes your device from the group.
