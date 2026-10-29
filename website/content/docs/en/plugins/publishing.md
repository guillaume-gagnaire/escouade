# Publishing on the store

A plugin is published on store.escouade.dev from its public GitHub repository and a git tag. This page says how to submit a version, what the store checks, and how to fix a rejection.

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

## The binaries

A plugin that runs a compiled program attaches to the tag’s release one archive per system, whose name says the system and the architecture: `…-windows-x64.zip`, `…-macos-arm64.tar.gz`, `…-linux-x64.tar.gz` (50 MB at most each). Each system that runs a program missing from the repository must have its own. The store downloads them once, keeps them and computes their fingerprint.

Build them in a public GitHub Actions workflow with `actions/attest-build-provenance`: the provenance attestation ties each file to the tag’s commit. A binary without an attestation is published unverified and cannot be verified; a binary attested from another commit is refused.

## Fixing a rejection

Each check that fails says what is wrong, where (the field or the file, such as `permissions.network[0]`), and how to fix it. If you gave an e-mail address in your account, the same text goes there.

To fix it: commit, then push a **new tag**. The store sees it through the App and runs the checks again by itself. If the failure came from the store or from GitHub, the page offers “Run the checks again”.

## New tags

Once the plugin is published, each new tag of the repository becomes a version checked by itself, which waits for your click to appear. The “Publish new tags by themselves” box publishes it without waiting, if every check passes.

A tag moved after publication changes nothing to the published version: the store keeps the commit and the archive it froze, marks it “tag moved on the repository” and tells you. To publish this new code, push a new tag.
