# Permissions

What a plugin asks for in its manifest, what each request opens to it, and what Escouade enforces or only shows. The install window says them in words, the strongest first.

## Each permission

| In the manifest | What it opens | What Escouade enforces |
| --- | --- | --- |
| `run.command` | A program started in a process, with your rights: it can read your files, start other programs, connect anywhere. | Nothing beyond its other permissions: Escouade starts it, stops it with everything it started, caps its memory at 1 GB, but does not lock it in. |
| `run.js` | A program in JS or TypeScript, inside the app, in WASM. | All of it: 64 MB of memory, delays, no program started, no file read outside its folder, the network through Escouade only. |
| `permissions.decide` | Allowing or denying what your agents ask for, before you. | An answer within 500 ms, else the request comes to you; a denial wins; each decision is in the journal. Never given by a project. |
| `permissions.commands` | Command lines run on a fact, in your shell. | Nothing runs before you accept those commands as they are; a changed command asks again. |
| `permissions.events` | The facts it receives: a ticket that changes columns, an agent waiting… | A fact outside the list is never sent to it. |
| `permissions.network` | The hosts it connects to. | Enforced for a `run.js` program and for webhooks: nothing else goes out. Only shown for a `run.command` program, whose network Escouade does not filter. |
| `permissions.secrets` | The secrets it keeps in the system’s keychain, under a name of its own. | It gets only those of its list, never another plugin’s, an account’s or a harness’s; they are masked in its journal. |

The extension points add their own lines to the window: a harness says the program its agents will run, a theme where to find it.

## What stays in your hands

- **Taking a permission back**: the Permissions tab of its page, one switch per permission. Its program starts again without it (see [installing a plugin](plugins/installer#taking-a-permission-back)).
- **Switching it off for a project**: the Projects tab. A private project is off from the start for any plugin you installed.
- **An update that asks for more** asks for your consent again, and the window marks each extra permission “new”.

> [!WARNING]
> A `run.command` program has your rights. Even switched off for a private project, it could read its files on the disk: Escouade sends it nothing of that project, but cannot stop it. Install such a program only from an author you trust, or a version “Verified by Escouade”.

## For the author

Ask for as little as you can: each extra permission is one more line in the window, and one more reason for the review to refuse it ([plugin security](plugins/security)). A plugin that needs only the network and its files is better written as `run.js` than as `run.command`.
