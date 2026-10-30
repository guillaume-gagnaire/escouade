# Publishing on the store

A plugin is published on store.escouade.dev from its public GitHub repository and a git tag. This page says how to submit a version, what the store checks, and how to fix a rejection.

## One plugin per repository

The store reads **one plugin at the root of a repository**: `escouade-plugin.json`, `README.md` and `LICENSE` (or `COPYING`) at the top level, and the manifest’s `repository` naming that repository. A repository that keeps several plugins in subfolders, like `plugin-examples/` in Escouade’s repository, cannot be submitted as it is: give each plugin its own repository. A plugin’s folder is copied as it is to the root of the new repository; it then needs its README, its license and its `repository`.

## Submitting a version

1. Sign in on [store.escouade.dev](https://store.escouade.dev/en/submit) with GitHub, and install the “Escouade Store” App on the plugin’s repository. It reads the code and the tags, never writing. A private repository is refused: the code of a plugin of the store is read by all.
2. Choose the repository, then the tag of the version (`v1.2.0` or `1.2.0`). The store resolves it into its commit once, at submission, and never reads it again for this version.
3. The ten checks run on the page, one line each. The store downloads the commit’s archive once, keeps it as it came and computes its SHA-256 fingerprint; nothing of the plugin runs on the server.
4. If everything passes, publish: a first version asks for one to three categories and keywords. The version appears **unverified**; the “Ask for the review” box (checked) puts it in the queue of the “Verified by Escouade” review, which holds for this version only.

Each published version has its `version.json` file: the id, the number, the repository, the commit, the fingerprint of the archive, of the manifest and of each binary. Its fingerprint is the version’s, which the app checks and the review signs. A published version never changes, and its number is not used again.

## The ten checks

| Check | What it asks |
| --- | --- |
| Manifest | `escouade-plugin.json` at the root, valid against the schema: the same rules as in the app ([the manifest](plugins/manifest)). |
| Version | `version` equal to the tag (the `v` is optional), greater than the last one published. |
| Identifier | Free or already yours, published from this repository; `dev.escouade.*` is reserved. A name too close to another plugin’s goes through a manual review first. |
| Repository | `repository` equal to the repository submitted, `https://github.com/<owner>/<repository>`. |
| License | A free license (approved by the OSI or the FSF) as an SPDX id, the same one GitHub recognizes in the `LICENSE` or `COPYING` file. |
| README | `README.md` (or `README.fr.md` and `README.en.md`) at the root, not empty: it is the plugin’s page. |
| Screenshots | Those of `store.screenshots`: 6 at most, in PNG, JPEG or WebP, 1600 px wide and 1 MB each at most. The store re-encodes them, without their metadata. |
| Archive | 50 MB at most, no symbolic link, no absolute path nor `..`, no submodule nor unresolved Git LFS file. |
| Permissions | Consistent with the plugin: with no program and no webhook, it asks for no network, no events, no secrets; a secret it asks for is declared in `contributes.secrets`; a host is written alone (`hooks.slack.com`). |
| Binaries | The program of `run` is in the repository, or in the tag’s release for each system (see below). |

## The plugin’s page

Once published, the version has its page on the store: its name, its author, “Not verified” or “In review” until it is signed, what it will be able to do in words (the strongest permission first), its repository, commit and fingerprint, its versions, and “Open in Escouade”, which opens its page in the app.

Once the review has read the version, you get its answer by e-mail and in your account. Verified, it bears the seal “Verified by Escouade” and the review’s summary (the permissions read, the hosts reached, what was checked), on its page and in the app, which checks the signature with its own keys. If the review asks for changes or refuses, the version stays published, unverified, and the message says why: a new version that takes it into account goes through the review again.

The README is rendered there once, at submission, in the visitor’s language (`README.en.md` or `README.fr.md`, else `README.md`); the markdown pages of the `store.docs` folder make the “Documentation” tab. The store keeps the markdown and a short list of HTML tags (headings, lists, links, code, tables, `details`); it removes scripts, styles, `style` and `class` attributes, frames and forms. The repository’s images (PNG, JPEG or WebP, 5 MB at most, by a relative path or by their address on GitHub) are copied to the store and re-encoded; images from elsewhere, such as badges, are removed: a visitor loads nothing from a third party. A link to a file of the repository leads to GitHub, at the published commit.

Anyone can report a plugin from its page, without an account, with an e-mail address: a person reads every report. You see that a report is open, never who sent it.

## The binaries

A plugin that runs a compiled program attaches to the tag’s release one archive per system, whose name says the system and the architecture: `…-windows-x64.zip`, `…-macos-arm64.tar.gz`, `…-linux-x64.tar.gz` (50 MB at most each). Each system that runs a program missing from the repository must have its own. The store downloads them once, keeps them and computes their fingerprint.

Build them in a public GitHub Actions workflow with `actions/attest-build-provenance`: the provenance attestation ties each file to the tag’s commit. A binary without an attestation is published unverified and cannot be verified; a binary attested from another commit is refused.

## Preparing for the review

The “Verified by Escouade” review is done by hand, version by version, on the Escouade team’s machine: it downloads the archive the store keeps, checks its fingerprints, reads the code against the declared permissions and runs the [conformance bench](plugins/testing) in a container without network. It then signs `version.json` with a key that never leaves that machine: neither the store nor the CI has it. What makes it quick:

- **exact permissions**: every host reached in `permissions.network`, every secret in `contributes.secrets`, nothing more;
- **readable code**: a compiled file (`dist/`) has its source in the repository and rebuilds from it; no minified code without its source, no text encoded in base64;
- **few dependencies**, at fixed versions, without a `postinstall` script or a dependency pulled from a git repository;
- **attested binaries** (see above): without an attestation, a version cannot be verified;
- **short updates**: a new version is read on its diff with the last verified one, the permissions that grow first.

What the review refuses, and what the seal covers: [plugin security](plugins/security).

## Fixing a rejection

Each check that fails says what is wrong, where (the field or the file, such as `permissions.network[0]`), and how to fix it. If you gave an e-mail address in your account, the same text goes there.

To fix it: commit, then push a **new tag**. The store sees it through the App and runs the checks again by itself. If the failure came from the store or from GitHub, the page offers “Run the checks again”.

## New tags

Once the plugin is published, each new tag of the repository becomes a version checked by itself, which waits for your click to appear. The “Publish new tags by themselves” box publishes it without waiting, if every check passes.

A tag moved after publication changes nothing to the published version: the store keeps the commit and the archive it froze, marks it “tag moved on the repository” and tells you. To publish this new code, push a new tag.

## Managing your plugins

Your account, at [store.escouade.dev/en/account](https://store.escouade.dev/en/account), shows your plugins, their state, their latest version and their installs (those of the users who accepted Escouade’s statistics only). A version found on a new tag waits at the top, with the permissions it asks more: “Publish” or “Dismiss”. A plugin’s “…” menu opens its versions, its installs week by week and what you can do with it:

- **Take a version back**: it is no longer offered to install nor as an update; those who have it keep it, marked “taken back by its author”. Its number does not come back.
- **Take off the list**: the plugin leaves the search and the app’s catalog; its page stays readable, installations keep their versions, without updates. One click puts it back.
- **Transfer**: to another account of the store, which accepts it from its own. If the repository changes owners on GitHub, the new owner, once the App is installed on their side, can take the plugin over. Either way, its publications freeze for 7 days, the time for a report, and its page tells the transfer.
- **Delete**, by typing its id: it leaves the catalog at once; its page and archives go after 30 days, during which you can undo it. Its id stays your account’s, and its version numbers are not used again.

Each action goes to the journal and, if you gave an address, out by e-mail: an action you did not do shows. An account that still has plugins cannot be deleted: transfer or delete them first.
