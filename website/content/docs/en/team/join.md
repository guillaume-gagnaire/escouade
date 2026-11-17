# Join a team

For a member: join a team through the link you were sent, add your other devices, keep the recovery key, clone the team’s projects and answer the settings it asks you to accept.

> [!NOTE]
> The team server is a preview of 2.3. Today, a team shares its members, its project list, its settings and the Kanban of its projects. The spec, decisions, documents and memory on the server are coming.

## A team, in a few words

A team shares, on a server it hosts, what Escouade keeps on your machine today. Everything there is end-to-end encrypted on your machine: the server reads nothing. Your harness accounts, your secrets, your agents’ conversations and your code never go there (see [Where does my data go?](team/data)).

Each member has a role: **Administrator** (they invite, keep the project list and sign the team’s settings) or **Member**.

## Open the teams

The teams window opens:

- from Settings › Advanced › “Team server (preview)” › “Open the teams…”;
- from “Teams…” of the projects rail’s “+” button, when it shows a menu (it does when you have closed projects);
- from an invitation link opened in your browser.

It lists the teams of this device, and offers “Create a team”, “Join by a link”, “Add this device by a code” and “Recover a team”.

## Join by a link

An administrator sends you a link like `https://team.example.org/join/…#s=…&t=…`. It serves only once, and expires (72 h by default). What follows the `#` is a secret that never goes through the server.

1. Open the link in your browser: the server’s page hands it to Escouade. Or paste it in “Join by a link”.
2. Escouade shows the server and the six words of the team’s fingerprint. **Check them with the person inviting you**: they are their team’s. If they differ, don’t join.
3. Give the team a name on this device, then “Join”.
4. The request goes out. An administrator online lets you in: nothing to do, the window says so (“You are in the team …”).

If the app says the invitation was already used, someone used it before you: the link may have leaked, and its administrator is told; ask them for a new link. If the server doesn’t know the invitation, it expired or no administrator has let it through yet: try again in a moment, or ask for a new link.

## Add one of your devices

On a device already in the team, open its card and “Add a device…”.

- **QR code**: have the other device scan it, or paste its link in “Join by a link” on the other device. It is valid for an hour.
- **Short code**, without a camera: the device in the team shows an 8-character code (`K7Q4-M2XZ`). On the new device, “Add this device by a code”, with the server’s address and this code. A code has one try and 10 minutes: a wrong code makes it unusable, show a new one.

If you are not an administrator, your new device waits for an administrator of the server to let it through: as soon as one is online.

A lost device costs you nothing: the content belongs to the team, and an administrator invites you again.

## The team’s card

Each team has its card in the teams window:

- **Team fingerprint**: six words, the same for every member. Compare them out loud to be sure.
- **Team head**: the six words of its current state, the same for a member who is up to date.
- **The escrow key**, if the team has one: the card says the organization can read the team’s content, and names the holder.
- **This device’s keys in a file**, when the system keychain refused them (always on Linux).
- For an administrator: “Invite…”, the open, used or expired invitations, and the reminder to name a second administrator.
- “Team projects”, “Team settings”, “Read again” and “Remove from this device”.

## The recovery key

Whoever creates the team becomes its owner and gets its **recovery key**: 24 words, shown only once, three of which they type back. If every administrator loses their devices, these words and a token from the server’s host bring the team back (“Recover a team”).

- Write them down at creation: print them, or keep them in the organization’s password manager. Never in a message.
- As long as they are not confirmed, the card says so. If the app was closed before, the words are lost: “New recovery key” makes another one.
- A recovery gives a new recovery key; the old one no longer opens anything.

A plain member has no recovery key to keep.

## Team projects

“Team projects” shows the team’s project list, encrypted: the server reads neither the names nor the addresses.

- **On this device**: the projects of the list one of whose repositories is already cloned here, recognized by their repository’s address. “Attach to the team” makes it a team project, with no new clone: what the team imposes and recommends then holds for it. Your name and logo stay.
- **To clone**: the others, each with its full addresses, its branch and who has access. “Clone” opens the “New project” assistant on the clone. **Check the addresses before cloning.** The clone goes through your accounts on the code host (your git credentials, your ssh keys); without access, the assistant says so and tells you whom to ask on which host.
- A project kept to other members is marked “Kept to other members of the team.”
- An administrator adds, edits and removes the list’s projects. Only `https://` and ssh addresses get in.

## The team’s Kanban and the tickets of before

- **No id to copy.** A project’s entry in the list carries its team Kanban, encrypted like the rest. Attaching the project, or cloning it from the list, joins that Kanban: you see its tickets at once. Under each project, the screen says “The team’s Kanban joined”, “The team’s Kanban written” or “No Kanban of the team yet”.
- **Who writes it.** An administrator who shares the Kanban of a project of the list writes it there. A member proposes it: the administrator sees “A member proposes their Kanban for this project” and “Write the Kanban”. A project outside the list is still joined by its Kanban’s id.
- **Attaching writes the `team` key** of the repository’s `.escouade.yml` (the server’s address, the team’s and the project’s ids, the team’s name; nothing secret). Commit it: whoever opens the repository will see the project is the team’s. Escouade commits nothing.
- **“Pour into the team”**, on the Kanban, puts the tickets of before on the team’s Kanban: those of the `.escouade/tickets/` files ([Sharing the Kanban](shared-board)) and those of this machine you check (“Check all” is there). A preview first says which tickets leave, what leaves (key, title, description, criteria, column…) and what stays on your machine (images, joined files, account, model, harness, and the shared memory). Nothing leaves before “Pour”.
  - While the team does not have its docs on the server yet, the preview also names the spec, decisions and documents of `.escouade/`: “Also pour the spec, the decisions and the documents” sends them with the tickets. Attaching the project or joining its Kanban never sends them for you. When they do not go (on the server already, kept in the repository by the team, a private project), the preview says why.
  - The tickets of the files keep their id: poured twice, or by two teammates at once, they are never doubled.
  - A ticket poured takes a key of the team (`ACM-4`); a ticket imported from Jira, GitHub… keeps its own. Its card says “Former key: DEM-12”, and it can still be found by it.
  - The files are then frozen: Escouade no longer writes them, and the screen offers to remove them from the repository (`git rm -r .escouade/tickets`), removing and committing nothing.

## A repository that names a team

When you open a repository whose `.escouade.yml` carries the `team` key, a band at the top of the window says so:

- **You are not in that team**: “This project belongs to the team Acme on team.acme.fr, according to the repository.” “Ask for an invitation” opens the “Join a team” screen, with the server’s address: ask an administrator of the team for a link, then paste it.
- **You are**: “Attach to the team” attaches the project, as in “Team projects”.
- “Don’t offer again for this project” hides the band, until the file names another team.

The name and the address come from the repository, not from the team: the band says so. Escouade never connects to that address by itself, neither when it shows the band nor when it opens the join screen; an address that is not a team server’s (`file://`, a password in the address…) shows no band. A repository therefore cannot plug your machine into a server.

## The team’s settings

The team recommends and imposes settings on its projects through its team `.escouade.yml`, signed by its administrators (see [the security officer’s guide](team/security#what-a-team-can-impose)). “Team settings” shows it:

- the version in force, who signed it and when, and the quorum rule;
- **what the team imposes**, with the reason the administrator wrote, and **what it recommends**;
- **the history** of each version and its change, key by key.

In the settings of a team project, what it imposes is grayed out, with a padlock and “Imposed by the team …”; a forbidden choice says “not allowed by the team …”.

### The consent you are asked for

What touches the network or runs a command never applies without your consent, on this device:

- **A sensitive key** (a proxy’s address, its exclusions, a certificate authorities file) comes with what it allows, said plainly: “This certificate lets its holder read Escouade’s TLS traffic.” “Accept on this device” applies it.
- **A proposed command** (the worktree setup, `launch`, the test command) comes with its text and its fingerprint. Read it: it will run on this device, with your rights, in the team’s projects. “I read it, accept” accepts it.
- A change of the value or of the command asks for your consent again. “Withdraw my consent” withdraws it.

A band, at the top of the window, announces each new version (“The team Acme changed 2 settings · Claire Martin, 5 min ago · View”) and each consent waiting (“Acme asks your consent for 1 setting”).

## If the app says the server lies

If the server shows two versions of the team, goes back, or changes what your device had already read, the team’s card says “The server shows two versions of the team: contact an administrator.”, and why. Your device then stops reading the team and sending it anything.

- Compare the head’s six words with an administrator: if they differ, the server lies to one of you.
- “Try again” reads again: a reading that passes lifts the alarm.
- “Remove the team from this device” erases its state and keys from this machine.

## What is not there yet

- The spec, decisions, documents and memory on the server (coming).
- The team as a separate space in the window, with its members, devices and their roles, and the key-by-key settings editor (coming).
