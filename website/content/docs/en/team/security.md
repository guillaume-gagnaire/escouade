# The security officer’s guide

For the security officer who assesses a team server: what it sees and what it doesn’t, whom it protects against and whom not, the keys, what a team can impose and how it is signed, and what remains a matter of trust.

> [!WARNING]
> **The external audit of the cryptography has not been done.** The scheme rests on MLS (RFC 9420) and standard primitives, but the way Escouade puts them together has only been checked by its own tests and fuzzing. An audit by outside auditors is planned before the server is called stable; until then, the team server is a preview.

## In short

- **The server is blind.** It keeps, numbers and broadcasts bytes encrypted on the devices. It holds no content key and runs nothing: no agent, no command, no plugin.
- **End-to-end encryption is the only mode.** There is no option where the server reads the content.
- **It doesn’t see the content, it sees the activity**: who writes, when, how much, on how many projects. The details are below, without playing anything down.
- **A team can only restrict.** Nothing that runs is imposed by the team: each command waits for each device to read and accept it.
- **Harness accounts, secrets, agents’ conversations and code never go to the server.** See [Where does my data go?](team/data).

## What the server sees

The server is hosted by the team: it chooses the machine and who administers it. Here is what that host, or whoever takes the server, can read.

| Visible to the server | Hidden from the server |
|---|---|
| The existence of each team, its number of devices, each one’s public key, its role on the server (administrator or member), revoked or not | The names of the members and of the team: a team’s name stays on each device |
| The number of the team’s epochs, that is, of changes to its members and its keys | Who is a member of what in the team’s roster, which travels in encrypted messages |
| Each write: its stream (the team or a project), its kind, the device that wrote it, its time, its size | What it says: the operations, the settings, the invitations, the key envelopes |
| The number of projects, under opaque ids, and their streams | The projects’ names, repositories and branches: the team’s project list is encrypted |
| Files: an opaque id, the size, the device that wrote it, the time | Their name, their type, their content |
| The SHA-256 of enrollment tokens, their expiry, a second use | An invitation’s secret, which stays after the link’s `#` |
| The devices’ IP addresses, held in memory for the rate limit, and the connection times | The team’s `.escouade.yml` and what it imposes |

- **What this reveals**: the team’s activity (who works when, at what pace, on how many projects), the size of objects, changes to its members.
- **The server’s log** keeps only each request’s method, route template, status and duration: no id, no IP address, no token. The log of the reverse proxy in front of it, however, keeps IP addresses.
- **The metrics** carry no team, project or device id.
- **Coming**: taking a ticket, once the team Kanban is there, will be in clear on the server (which device holds which opaque id, until when), so that it guarantees two agents never take the same ticket. The server won’t know which ticket it is.

Escouade’s tests look for the ids in the server’s log and metrics, and for the titles, names and addresses in what it keeps, and don’t find them.

## The threat model

| Threat | What it gets |
|---|---|
| The server taken over, or its curious system administrator | The encrypted bytes and the metadata above. It can neither read, nor write on behalf of a member, nor slip its own key into the team. It can deny service, delay, or show an old state, which the devices see. |
| A leaked backup of the server | The same encrypted bytes and metadata. |
| A leaked invitation link | Entry into the team, if it is used before the invitee: the rightful invitee then sees that the invitation was already used, and the administrator is told. The link carries no key. |
| A member who leaves | Nothing written after they leave. They keep what they already read. |
| A stolen device | Nothing after its revocation. Before: what the system keychain, the locked session and disk encryption protect. |
| A member’s compromised machine | Everything this member reads. It is out of scope: their agents run on this machine and already read the code and the docs. |

### How the server is held in check

- **It can’t slip in a key.** The team is an MLS group; each device checks each change of members against the team’s rules (an administrator adds and removes, a member adds only their own devices), and wraps a key only for a device of that verified group.
- **It can’t forge a write.** Each operation is signed by the device that writes it, chained to the earlier ones of the same device, and encrypted with its project and its epoch as associated data: the server can neither move it, replay it, reorder it nor drop one without it showing.
- **It can’t roll back.** Each device keeps, in the keychain, the highest epoch it saw and its head, and refuses an older state, from the server as from a backup put back on the disk.
- **A fork shows.** If it shows two versions of the team to two members, the first write that passes from one version to the other reveals it. The app then says “The server shows two versions of the team: contact an administrator.”, stops reading the team and writing to it, and shows the six words of the team’s head to compare out loud.

### What it doesn’t protect against

- **A member’s compromised machine**, nor what a member already read before leaving.
- **Activity analysis**: the server sees who writes, when and how much. Sizes are not rounded.
- **Denial of service**: the server, or an administrator, can stop the team from working together. Each device then keeps its copy, readable offline.
- **A lasting partition**: a server that forever shows two versions of the team, with nothing passing from one to the other, only shows when the six words of the head are compared out of band. Likewise, a version of the settings hidden from a device that never saw it doesn’t show from that device alone.
- **The holder of the escrow key**, if the team has one: they read everything, by design (see below).
- **The local copy** of each team project, on each device, is not encrypted, like today’s local Kanban: only the keys are in the encrypted state. The device’s disk encryption protects it.

## The keys

- **Each device has its keys, per team**, created on it and never exported: an Ed25519 key to connect to the server (it signs a challenge: there is no password), an Ed25519 key as a member of the MLS group (it signs its writes), and the key that encrypts its state on the disk. Two signing keys rather than one: the server can’t link a device to its writes.
- **Where they live**: in the system keychain, the Windows Credential Manager, the macOS keychain. On Linux, where Escouade has no keychain, they go in a file only the owner can read, and the team’s card says so. The checkpoint against rollback is then on the same disk: its protection is weaker.
- **The encryption**: MLS (RFC 9420) through openmls, with the suite `MLS_128_DHKEMX25519_CHACHA20POLY1305_SHA256_Ed25519`. The content keys are derived from the group at each epoch, then per project and per object; the content is encrypted with XChaCha20-Poly1305 and signed with Ed25519. Nothing is invented in cryptography: standard primitives, in Rust (RustCrypto).
- **The history can be read**: a new device climbs back through past epochs by a chain of links. This is on purpose, since a Kanban and a spec last; the flip side is that a compromised device gives away all the history it can read.
- **The invitation**: the link carries a secret after the `#`, which neither browsers nor HTTP send to the server. The server keeps only the hash of a token derived from that secret, and lets it serve only once; it expires (72 h by default). The invitee checks the six words of the team’s fingerprint with whoever invites them, and an administrator online lets them in. The server can neither swap its own key for the invitee’s, nor show them a fake team.
- **Another device of the same member** joins by a QR code (a link valid for an hour) or by an 8-character short code, exchanged over SPAKE2: one try, 10 minutes, and nothing the server can try offline.

### The recovery key and the escrow key

- **The recovery key**, mandatory: 24 words, shown once to the owner when the team is created, who types three back. It counts as an offline administrator. If every administrator loses their devices, the host gives a recovery token and the 24 words bring the team back on a new device; a new recovery key then replaces the old one, which no longer opens anything. Keep it printed, or in the organization’s password manager.
- **The escrow key**, optional, at creation: a key pair held by the organization, for which all the content is encrypted too. The organization can then always read it, which some companies require. **Every member sees it**, on the team’s card, with its holder. It reads, it writes nothing.
- **A member who loses their devices loses nothing**: an administrator invites them again.
- **A single administrator** is a risk: the app says so as long as there is only one, and suggests naming a second.

## A member leaving and key rotation

- Removing a member first has the server cut off their devices (their sessions and live connections close), then opens a new epoch without them: what is written next escapes them. Revoking a stolen device does the same for that device.
- The old content is not re-encrypted: the departed member already had it, and re-encrypting it would take nothing away from them. The server no longer serves it to them.
- In the app, the screen of the team’s members and devices, which removes a member, revokes a device or changes the keys on demand, is coming. Today, a device removes itself from a team (“Remove from this device”); it stays in the group until an administrator removes it.

## What a team can impose

A team recommends and imposes settings through its team `.escouade.yml`: a file signed by its administrators, encrypted under the team’s key, that lives on the server and never in a repository. Its keys are those of the [device policies](managed-policies#the-policies) and of a repository’s [.escouade.yml](escouade-yml).

- `defaults`: what the team recommends; the repository and the device change it.
- `enforced`: what it imposes. The setting is grayed out, with a padlock and who imposes it, and it is Escouade’s core that refuses to change it, through the window, the MCP server or the repository’s `.escouade.yml` alike.
- `proposed`: what runs, proposed only (see [the hook trap](#the-hook-trap)).
- `reasons`: the reason for each setting, shown to the member.

What a team imposes holds for its projects only. The settings of the app itself (the network, telemetry, voice, updates, plugins) hold for the app while the device is in the team: the team’s place as a separate space, which will bound them to it, is coming.

### The kinds of keys

Each key has its kind, set by the app, not by the team:

| Kind | Examples | What it becomes |
|---|---|---|
| Restrict | the allowed permission modes, harnesses and models, telemetry turned off, Remote Control forbidden | Applied right away. The worst an attacker gets from it is a denial of service. |
| Value | the target branch, a branch prefix, what approval does | Applied and shown. |
| Sensitive | a proxy’s address, its exclusions, a certificate authorities file | Waiting for each member’s consent, on their device, with what it allows said plainly (“This certificate lets its holder read Escouade’s TLS traffic.”). A changed value asks for consent again. |
| Executable | the worktree commands, `launch`, the test command, hooks, an MCP server, the agents’ environment variables, a plugin to install | Never imposed. Only proposed, read and accepted on each device at its fingerprint. |

### How it is signed

- **Each version** of the file carries its number, the hash of the one before, the name its author gives and the time. It is signed by the key of an administrator’s device. A device refuses a version that is unsigned, forged by the server, signed by someone who is not or no longer an administrator, replayed, or going backward; it never undoes the chain it chose.
- **The quorum**: a version that changes a sensitive key, the list of signers (`signers`) or the quorum itself waits for a second administrator’s signature. Two devices of the same administrator count as one. The quorum is on by default as soon as the team has two administrators; with one, it doesn’t apply, and the screen says so.
- **Each change is shown**: a band across the whole window says “The team Acme changed 2 settings · Claire Martin, 5 min ago · View”, and the history of each version, with its change key by key, is in the team’s settings.

## The hook trap

The scenario: someone takes the server, or an administrator’s machine, and pushes into the team’s file a worktree setup command (`curl https://… | sh`), a hook, a plugin to install by default, an API address that diverts the agents’ requests, or a root certificate that intercepts all TLS. At the next sync, every device would run it: the whole fleet taken in a minute. A central configuration server is the dream target.

The defenses, in layers:

1. **The server can forge nothing**: a version not signed by an administrator of the team is refused.
2. **Each change is shown** to each member, with its author and its diff.
3. **The team can only restrict**: a sensitive key waits for each member’s consent, a key that runs is never imposed. Imposing a command can only mean one thing: “only this one is allowed”, never “run it”.
4. **The quorum**: an administrator taken alone gets no sensitive key through.
5. **What runs is imposed only by the managed device**: an MDM already installs what it wants on the machine. The team server adds no power to run anything beyond what the IT department already has.

**What never gets in, even signed.** A list written in the app, which the team doesn’t change. A version holding one of its lines is refused whole, on reading, and the administrator’s screen refuses to sign it:

- `bypassPermissions`, named anywhere, or allowed;
- a command that would run without consent: a key that runs outside `proposed`, or a proposal marked as accepted in advance;
- a plugin installed without the member’s gesture;
- a `dev.escouade.*` plugin signed by the team;
- a project made public by the file.

These rules extend those of a repository’s [.escouade.yml](escouade-yml), whose commands already run only once approved on each machine.

## Device policies

The IT department also imposes settings through its own tools, with no team server at all: group policies or Intune on Windows, a configuration profile on macOS, files under `/etc/escouade/policies` on Linux. See [device policies](managed-policies).

- **The managed device comes before the team.** A device policy holds for the whole app; the team’s, for its projects.
- **For the same key**: a bound (the allowed harnesses, the allowed modes) is intersected, what both allow remains; a value (the proxy’s address) from the managed device wins.
- **It enrolls and bounds**: `team.server` and `team.fingerprint` set the company’s team server and fingerprint, `team.onlyThisServer` refuses any other team server, `servers.allowed` bounds the servers reached. Prefilling the team entry screen with these values is coming; the refusal is in place.

## What remains a matter of trust

- **Escouade’s code on each device**, and its updates: it is what checks the signatures, the chains and the rules. The code is open.
- **openmls and the cryptography libraries**, and the way they are put together, which the external audit hasn’t read yet.
- **The administrators**: they can restrict, set values and propose commands, and an administrator online lets invitees in. The quorum bounds what one alone can do.
- **The members** and their machines, with their agents.
- **The system keychain** of each device.
- **The holder of the escrow key**, if the team has one, and that of the recovery key.
- **The server’s host**, for availability and metadata, not for content.
- **The code host** (GitHub, GitLab…), which keeps the code: the team server gives no access to it.

## What is not there yet

- The external audit of the cryptography (coming, before the server is called stable).
- The live team Kanban, and a ticket taken by a single agent (coming).
- The spec, decisions, documents and memory on the server; for the memory, the signed provenance of each memory and human approval of those an agent writes (coming). Today, the shared memory stays in each person’s repository (see [the shared memory](shared-memory)).
- The screen of members and devices, the team’s place as a separate space, the owner and reader roles (coming).
- Entry through the company directory (OIDC): today, you only get in by invitation.
- Per-project rights on the server: it serves a project to every device of the team; only encryption protects a project kept to a few members.
