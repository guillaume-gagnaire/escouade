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
| The spec, decisions, documents | the pages, the decisions, the uploaded files | the extracted text, the index, the vectors | coming |
| The shared memory | the memories and their provenance | the index and the vectors | coming |

Until the “coming” rows are there, your projects’ spec, decisions, documents and memory stay where they are today: on your machine and in your repository. A project’s tickets go only if its Kanban is shared with the team.

## What never goes there

- **Harness accounts**: your sign-in to Claude Code, Codex and the others, your quotas. The team server sees none of them; each member uses their own.
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
