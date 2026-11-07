# Permissions

What a plugin asks for in its manifest, what each request opens to it, and what Escouade enforces or only shows. The install window says them in words, the strongest first.

## Each permission

| In the manifest | What it opens | What Escouade enforces |
| --- | --- | --- |
| `run.js`, `run.wasm` | A program in JS, TypeScript or Rust, inside the app, in WASM. | All of it: 64 MB of memory, delays, and nothing beyond what it declares here: no network, no file, no program. |
| `run.command` | A program started in a process, with your rights: it can read your files, start other programs, connect anywhere. The store publishes none: it only comes from a folder or a git repository. | Nothing beyond its other permissions: Escouade starts it, stops it with everything it started, caps its memory at 1 GB, but does not lock it in. |
| `permissions.decide` | Allowing or denying what your agents ask for, before you. | An answer within 500 ms, else the request comes to you; a denial wins; each decision is in the journal. Never given by a project. |
| `permissions.commands` | The programs of your computer it can run, line by line. | Nothing else runs, without a shell, and each run goes to the journal. But the program it runs has your rights. |
| `permissions.files` | The folders it reads or writes: its own, a folder of its own, your agents’. | A closed list of roots, resolved by Escouade, links followed, never a private project. Each write goes to the journal. |
| `limits` | More time or bytes for a call. | Capped by Escouade: 120 s, 32 MB, 4 MB for a file. |
| `permissions.events` | The facts it receives: a ticket that changes columns, an agent waiting… | A fact outside the list is never sent to it. |
| `permissions.network` | The hosts it connects to, the instances you give it (`{account}`), a harness’s local server (`local:shared`). | Enforced for a program in WASM and for webhooks: nothing else goes out. Only shown for a `run.command` program, whose network Escouade does not filter. |
| `permissions.secrets` | The secrets it keeps in the system’s keychain, under a name of its own. | It gets only those of its list, never another plugin’s, an account’s or a harness’s; they are masked in its journal. |

For each capability, what it allows, what Escouade refuses and what the window says: [the host’s capabilities](plugins/capabilities).

The extension points add their own lines to the window: a harness says the program its agents will run, a theme where to find it.

## What stays in your hands

- **Taking a permission back**: the Permissions tab of its page, one switch per permission. Its program starts again without it (see [installing a plugin](plugins/installer#taking-a-permission-back)).
- **Switching it off for a project**: the Projects tab. A private project is off from the start for any plugin you installed.
- **An update that asks for more** asks for your consent again, and the window marks each extra permission “new”.

> [!WARNING]
> A `run.command` program has your rights. Even switched off for a private project, it could read its files on the disk: Escouade sends it nothing of that project, but cannot stop it. Install such a program only from an author you trust, or a version “Verified by Escouade”.

## For the author

Ask for as little as you can: each extra permission is one more line in the window, and one more reason for the review to refuse it ([plugin security](plugins/security)). A plugin is written as `run.js` or `run.wasm`: the store takes nothing else.
