# Plugin security

A plugin from the store goes through three guards: the store’s eleven checks on submission, the “Verified by Escouade” review, made by an AI agent then signed, and the app’s checks on every install. This page says what each one holds, what the review refuses, and what the seal does not promise.

## What the app checks

On install and on every update, Escouade downloads `version.json`, the archive and its WASM module, then checks:

- **the fingerprints**: `version.json` against the catalog, the archive, the module and the manifest against `version.json` (SHA-256). One byte changed, on the store, on the way or in a cache, stops the install;
- **the signature**, for a verified version: `version.json` signed with minisign by a key of Escouade whose role covers this plugin (see [the keys of the seal](#the-keys-of-the-seal)), the signature’s comment naming this id and this number. The public keys are in the app; the secret keys live on one machine, never on the store’s server, never in the CI or in a repository. So the store cannot make a seal;
- **the archive**, read in memory, without a symbolic link or a path that leaves the plugin’s folder;
- **the folder** `version.json` names for a plugin kept in a subfolder of its repository: a plain relative path, without `..`, an absolute path or a backslash; otherwise nothing is installed. The store, for its part, refuses at submission a folder that goes through a symbolic link.

An installed plugin’s seal is checked again on your machine, without the store. Then the install window shows its permissions in words, before anything runs ([installing a plugin](plugins/installer)).

## What the seal covers

“Verified by Escouade” stands for **one version**: its code at the published commit, its archive and its module, recompiled by the review, named by the fingerprint of its `version.json`. A new version is “not verified” until its own review, even if the one before was: an update is where an honest plugin can change hands. **Install verified versions only**, in the Plugins view, refuses any version without a seal.

The seal says that an AI agent of Escouade read this code against what it declares, that the bench ran it without network, then that the version was signed. It is not a human review: the plugin’s page, on the store and in the Plugins view, says so with the key that signed. It does not promise there is no bug, nor the quality of the third-party service the plugin reaches.

## The keys of the seal

Each key of Escouade has a role, written in the code of the app, of the store and of their tools: a signature out of its role is refused, with its reason.

| Key | What it seals | Who signs |
| --- | --- | --- |
| **The key of Escouade’s plugins** | the `dev.escouade.*` plugins, and nothing else | the squad’s chief, the agent that made the review; the key has no password |
| **The key of third-party plugins** | the plugins of other authors, and the revocation list | the publisher, who types the key’s password at each signature |
| **The backup key** | everything, the day another key is lost or stolen | the publisher; it is kept offline |

Each seal says its key: its id is in the signature’s file (`version.json.minisig`), in the store’s catalog, on the plugin’s page and in the app’s Plugins view (“Signed with the key of Escouade’s plugins · E0516468FFDCBDD5”). So a key without a password stolen from the publisher’s machine seals no plugin of another author, nor a revocation list.

## What the review refuses

The review reads each version with a script that prepares leads for it, file and line, and an agent that reads the code as data. Its outcomes: “Verify”, “Request changes” (the version stays published, not verified, the message says what to fix) or “Refuse”.

**Refused outright**, without reading further:

- an archive or a module whose fingerprint is not the one in `version.json`;
- a version that is no longer the one the store froze.

**No seal until it is fixed**:

- a host reached that is not in `permissions.network` (a hard-coded address, a `fetch`, a `WebSocket`);
- reading secrets or personal folders beyond what the plugin declares: environment variables, `~/.ssh`, `~/.claude`, the keychain outside `secrets/get`;
- files read or written outside the plugin’s folder and what its permissions open;
- a program started that the permissions do not announce (`child_process`, `spawn`, `exec`);
- dynamic code: `eval`, `new Function`, an `import()` of an address;
- unreadable code: minified without its source, text hidden in base64, binary blobs;
- unpinned dependencies, a `postinstall` script, a dependency pulled from a git repository;
- instructions added to the agents that turn them from their task (a prompt injection);
- a module the recompilation does not give back, to the byte;
- a Rust dependency that cannot be justified: what `Cargo.lock` adds is code that runs; `unsafe` code, an `extern` or a `#[link]` without a reason;
- `agentHome`, or an entry in `*` of `permissions.commands`, without a reason that was read;
- a tag moved since the submission, an archive entry refused (link, `..`, absolute path);
- a conformance bench that fails, or that tries the network or a write the sandbox refuses.

**Withdrawn**: a malicious behavior found (sending data elsewhere, hiding what the code does, lying about its permissions) leads to the version’s withdrawal, and the plugin’s if need be.

## Withdrawal and the revocation list

Escouade reads a **revocation list** signed by the key of third-party plugins and of the revocation list (or the backup key), at startup then every 6 hours, and refuses a list older than the last one read. It has two kinds of entries:

- **remove the seal**: the plugin keeps running, a banner says why;
- **emergency withdrawal**: the app stops the plugin, disables it and says why, with “Uninstall”. It deletes nothing on its own: your data is yours.

The list is also copied to escouade.dev, an hour at most after the store: if the store is down, the app reads it there. The last list read applies offline.

## Reporting a plugin

“Report”, on a plugin’s page on the store or in the app, is open to anyone with an email address, without a GitHub account: malicious, does not do what it says, misleading permissions, license, other. A person reads every report and answers you. The author sees that a report is open, never who made it.

## A plugin from outside the store

A plugin installed from a folder or a git repository has passed neither the checks nor the review: it is **not verified**, and the install window says so first. Its permissions apply the same way.
