# Plugin security

A plugin from the store goes through three guards: the store’s ten checks on submission, the “Verified by Escouade” review done by hand, and the app’s checks on every install. This page says what each one holds, what the review refuses, and what the seal does not promise.

## What the app checks

On install and on every update, Escouade downloads `version.json`, the archive and your system’s program, then checks:

- **the fingerprints**: `version.json` against the catalog, the archive, the program and the manifest against `version.json` (SHA-256). One byte changed, on the store, on the way or in a cache, stops the install;
- **the signature**, for a verified version: `version.json` signed with minisign by Escouade’s key, the signature’s comment naming this id and this number. The public keys are in the app; the secret key lives on one machine, encrypted, never on the store’s server, never in the CI or in a repository. So the store cannot make a seal;
- **the archive**, read in memory, without a symbolic link or a path that leaves the plugin’s folder.

An installed plugin’s seal is checked again on your machine, without the store. Then the install window shows its permissions in words, before anything runs ([installing a plugin](plugins/installer)).

## What the seal covers

“Verified by Escouade” stands for **one version**: its code at the published commit, its archive and its attested binaries, named by the fingerprint of its `version.json`. A new version is “not verified” until its own review, even if the one before was: an update is where an honest plugin can change hands. **Install verified versions only**, in the Plugins view, refuses any version without a seal.

The seal says that a person read this code against what it declares, and that the bench ran it without network. It does not promise there is no bug, nor the quality of the third-party service the plugin reaches.

## What the review refuses

The review reads each version with a script that prepares leads for it, file and line, and an agent that reads the code as data. Its outcomes: “Verify”, “Request changes” (the version stays published, not verified, the message says what to fix) or “Refuse”.

**Refused outright**, without reading further:

- an archive or a binary whose fingerprint is not the one in `version.json`;
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
- a binary without a provenance attestation tying it to the commit read;
- a tag moved since the submission, an archive entry refused (link, `..`, absolute path);
- a conformance bench that fails, or that tries the network or a write the sandbox refuses.

**Withdrawn**: a malicious behavior found (sending data elsewhere, hiding what the code does, lying about its permissions) leads to the version’s withdrawal, and the plugin’s if need be.

## Withdrawal and the revocation list

Escouade reads a **revocation list** signed by the same key, at startup then every 6 hours, and refuses a list older than the last one read. It has two kinds of entries:

- **remove the seal**: the plugin keeps running, a banner says why;
- **emergency withdrawal**: the app stops the plugin, disables it and says why, with “Uninstall”. It deletes nothing on its own: your data is yours.

The list is also copied to escouade.dev, an hour at most after the store: if the store is down, the app reads it there. The last list read applies offline.

## Reporting a plugin

“Report”, on a plugin’s page on the store or in the app, is open to anyone with an email address, without a GitHub account: malicious, does not do what it says, misleading permissions, license, other. A person reads every report and answers you. The author sees that a report is open, never who made it.

## A plugin from outside the store

A plugin installed from a folder or a git repository has passed neither the checks nor the review: it is **not verified**, and the install window says so first. Its permissions apply the same way.
