# Publishing on the store

A plugin is published on store.escouade.dev from its public GitHub repository and a git tag. This page says how to submit a version, what the store checks, and how to fix a rejection.

## One or several plugins per repository

A plugin is **a folder of a repository**: the root, or a subfolder that holds its `escouade-plugin.json`. A repository may keep several, one per folder (`plugins/slack/`, `plugins/ntfy/`); each one is a plugin of its own, with its own id.

- **The folder** is chosen at submission, before the version: the store lists the repository’s folders that hold a manifest, and takes a typed path, relative to the root. By default, the root. A path that is absolute, has `..`, goes through a symbolic link or has no manifest is refused.
- **The tags** of a plugin in a folder name that folder: `plugins/slack/v1.2.3` (advised), or `slack@1.2.3` and `slack-v1.2.3`, the name being the folder’s last segment. A bare tag `v1.2.3` is only a plugin at the root’s: in a repository whose releases start from `v*`, it stays theirs. A tag that may be two folders’ (the same last segment) is checked for neither, and the store tells you which one to push.
- **The README and the license** of the folder come first; without them, the repository root’s. The store recognizes a folder’s `LICENSE` only if it is the root’s text, or by its `SPDX-License-Identifier: MIT` line.
- **The manifest’s `repository`** names the repository (`https://github.com/you/plugins`), or the folder (`https://github.com/you/plugins/tree/main/plugins/slack`).
- **The archive** the store keeps and the app installs is the folder’s alone; `version.json` says the folder (`path`), and the plugin’s page leads to it.
- **Changing the source**: a plugin may move from a repository of yours to another (or from one folder to another), by submitting its next version from the new source. The “Identifier” check says so; at publication, the change is noted on the plugin’s page and the version goes to the review. The versions before keep their source.

`npm create escouade-plugin` run in a folder of a git repository already there writes the plugin in it, `repository` filled in, and its README says the tag to push (`plugins/<folder>/v0.1.0`).

## Submitting a version

1. Sign in on [store.escouade.dev](https://store.escouade.dev/en/submit) with GitHub, and install the “Escouade Store” App on the plugin’s repository. It reads the code and the tags, never writing. A private repository is refused: the code of a plugin of the store is read by all.
2. Choose the repository, the plugin’s folder (the root by default), then the tag of the version (`v1.2.0` or `1.2.0` at the root, `plugins/slack/v1.2.0` in a folder). The store resolves it into its commit once, at submission, and never reads it again for this version.
3. The eleven checks run on the page, one line each. The store downloads the commit’s archive once, keeps it as it came and computes its SHA-256 fingerprint; nothing of the plugin runs on the server.
4. If everything passes, publish: a first version asks for one to three categories and keywords. The version appears **unverified**; the “Ask for the review” box (checked) puts it in the queue of the “Verified by Escouade” review, which holds for this version only.

Each published version has its `version.json`: the id, the number, the repository, the commit, the fingerprint of the archive, of the manifest and of the module. Its fingerprint is the version’s, which the app checks and the review signs. A published version no longer changes, and its number is not reused.

## The eleven checks

| Check | What it asks |
| --- | --- |
| Manifest | `escouade-plugin.json` at the plugin’s root (the repository’s, or its folder), valid against the schema: the same rules as in the app ([the manifest](plugins/manifest)). |
| Version | `version` equal to the tag (the `v` is optional; in a folder, `<folder>/v1.2.3`), greater than the last one published. |
| Identifier | Free or already yours, published from this repository and this folder, or from a new source of yours (noted, the version reviewed); `dev.escouade.*` is reserved to Escouade’s own plugins, submitted from its own repository. A name too close to another plugin’s goes through a manual review first. |
| Repository | `repository` equal to the repository submitted, `https://github.com/<owner>/<repository>`, or to its folder, `…/tree/<ref>/<folder>`. |
| License | A free license (approved by the OSI or the FSF) as an SPDX id, the same one GitHub recognizes in the `LICENSE` or `COPYING` file. |
| README | `README.md` (or `README.fr.md` and `README.en.md`) at the root, not empty: it is the plugin’s page. In a folder, the repository root’s stands in when it has none; the license too. |
| Screenshots | Those of `store.screenshots`: 6 at most, in PNG, JPEG or WebP, 1600 px wide and 1 MB each at most. The store re-encodes them, without their metadata. |
| Archive | 50 MB at most, no symbolic link, no absolute path nor `..`, no submodule nor unresolved Git LFS file. |
| Permissions | Consistent with the plugin: with no program and no webhook, it asks for no network, no events, no secrets; a secret it asks for is declared in `contributes.secrets`; a host is written alone (`hooks.slack.com`). |
| Binaries | None: `run.command`, `run.windows`, `run.macos` and `run.linux` are refused to every plugin, each at its field: every plugin of the store runs in WASM. A program of the user’s is declared by `permissions.commands`. A `run.js` or `run.wasm` plugin has nothing to provide. |
| Compilation | For a `run.js` plugin: its bundle, taken from the archive (5 MB at most, never an `npm install`), compiled to WASM twice by `escouade-plugin build`, isolated, each time in a fresh folder. For a `run.wasm` plugin: its crate, once the other checks have passed, the same way (see [below](#a-plugin-in-rust)). Two different modules refuse the version; so does an `import` other than `node:fs` and `node:readline`. The module kept is the one the app downloads and checks. |

## A plugin in JS or TypeScript

A plugin written in JS or TypeScript ships as `run.js`: its manifest names its bundle, `"run": { "js": "dist/plugin.mjs" }`, a file of its folder committed with the rest (without `node_modules`). You publish no `.wasm`: the store makes it.

- **The store compiles the bundle as the archive holds it**: never an `npm install`, never your build. It compiles it twice, each time in a fresh folder, with no cache, with no network when the server’s kernel allows it, by the pinned Javy and Escouade’s engine, deterministically; the compiler is stopped after 2 minutes or past 1 GB of memory, and the version is refused.
- **Reproducible**: the same bundle gives the same module to the byte, on Windows, macOS or Linux. Two different modules refuse the version (“the compilation is not reproducible”), with their two fingerprints. `escouade-plugin build` makes the same module on your machine: `packs/<id>-<version>.wasm` and its `.sha256`, with `--json` for the compiler (the Javy version, the engine’s fingerprint, the prelude’s).
- **What the store keeps**: the module, under its SHA-256, and what made it, in `version.json` (`wasm: { sha256, size, compiler }`); the version’s fingerprint, hence its signature, covers it. The app downloads it, compares its size and fingerprint to `version.json` and installs nothing if they differ; Javy does not run on the user’s machine.
- **The “Verified by Escouade” review** recompiles the bundle it read and compares the fingerprint to the published module’s: what was read is what runs. Another fingerprint refuses the version.
- **The plugin’s page** says “Runs inside Escouade, sandboxed · compiled to WASM by the store”, and its list row “Compiled to WASM by the store”.

A plugin of the store runs neither Node nor any program of its own: the “Binaries” check refuses `run.command`, with the reason (“`run.command` would start a native program: the store no longer publishes one, every plugin runs in WASM.”). Installed from a folder or a git repository, outside the store, a plugin started by a command is still allowed, unverified.

## A plugin in Rust

A plugin written in Rust ships as `run.wasm`: its manifest names its `Cargo.toml`, `"run": { "wasm": "Cargo.toml" }`, committed with its sources and its `Cargo.lock`. You publish no `.wasm`: the store makes it, and writes its own in place of any `plugin.wasm` the archive holds.

- **Once the other checks have passed**, the store compiles the crate as the archive holds it, alone in its folder: what surrounds the plugin in your repository, a workspace or a `[patch]`, does not count.
- **Two steps, twice**, each time in a fresh folder: `escouade-plugin fetch` downloads the dependencies from crates.io and runs nothing of the plugin, then `escouade-plugin build` compiles with no network, in a sandbox (bubblewrap) with no Node, none of the store’s variables, no `/etc` of its own. 5 minutes and 2 GiB at most.
- **Reproducible**: rustc 1.98.0, `wasm32-wasip1`, an imposed profile, and `rustc-canon`, which removes what belongs to the machine. For the same sources and the same `Cargo.lock`, the module is the same to the byte on Windows and on Linux. Two different modules refuse the version, with their two fingerprints.
- **What is refused**, before anything runs: no `Cargo.lock`, a `build.rs`, a macro of your own, a crate off the closed list, a path out of the folder. The list is in [the Rust kit](plugins/rust#what-the-compilation-refuses), with what to do. A `Cargo.lock` that does not match `Cargo.toml` or crates.io is refused too.
- **Try it before you push**: `escouade-plugin validate .` says these refusals without running anything, and `escouade-plugin build . --json` gives the module and the fingerprint the store will find.
- **What the store keeps**: the module, under its SHA-256, and what made it, in `version.json` (`wasm: { sha256, size, compiler: { rust, target, canon, lock } }`). The version’s fingerprint, hence its signature, covers it.
- **The review** recompiles in a container without network and compares the fingerprint. It also reads `Cargo.lock`, dependency by dependency, and its diff with the version before: what the lock adds is code that runs. `unsafe` code, `extern`s and `#[link]`s are to be justified.

A failure of the store, a network down for example, does not refuse the version: it waits, to run again.

## The plugin’s page

Once published, the version has its page on the store: its name, its author, “Not verified” or “In review” until it is signed, what it will be able to do in words (the strongest permission first), its repository, commit and fingerprint, its versions, and “Open in Escouade”, which opens its page in the app.

Once the review has read the version, you get its answer by e-mail and in your account. Verified, it bears the seal “Verified by Escouade” and the review’s summary (the permissions read, the hosts reached, what was checked), on its page and in the app, which checks the signature with its own keys. If the review asks for changes or refuses, the version stays published, unverified, and the message says why: a new version that takes it into account goes through the review again.

The README is rendered there once, at submission, in the visitor’s language (`README.en.md` or `README.fr.md`, else `README.md`); the markdown pages of the `store.docs` folder make the “Documentation” tab. The store keeps the markdown and a short list of HTML tags (headings, lists, links, code, tables, `details`); it removes scripts, styles, `style` and `class` attributes, frames and forms. The repository’s images (PNG, JPEG or WebP, 5 MB at most, by a relative path or by their address on GitHub) are copied to the store and re-encoded; images from elsewhere, such as badges, are removed: a visitor loads nothing from a third party. A link to a file of the repository leads to GitHub, at the published commit; a link to a page of the documentation or to the README stays on the store, in its tab.

Anyone can report a plugin from its page, without an account, with an e-mail address: a person reads every report. You see that a report is open, never who sent it.

### The documentation, by language

The folder `store.docs` names gives its pages in two languages in the subfolders `fr/` and `en/`: the visitor sees those of their language, else those of the other. A folder without `fr/` or `en/` shows all its pages in both languages. The pages come in the order of their names (prefix them: `01-install.md`, `02-set-up.md`), 20 at most per language; a tab’s title is the page’s first heading (`#`), else its name without the prefix. Images go beside them, in a folder both languages share.

```text
my-plugin/
├── escouade-plugin.json      "store": { "docs": "docs/" }
├── README.fr.md
├── README.en.md
└── docs/
    ├── fr/
    │   ├── 01-installer.md   # Installer   …   [la suite](02-regler.md)
    │   └── 02-regler.md      # Régler le sujet
    ├── en/
    │   ├── 01-install.md     # Install     …   ![The topic](../images/topic.png)
    │   └── 02-set-up.md      # Set it up
    └── images/
        └── topic.png
```

Links between pages are written as relative paths (`02-set-up.md`, `../fr/01-installer.md`), and images too (`../images/topic.png`). If `store.docs` leads to no markdown page, the README check says so as a warning: the version passes, and its page shows the README alone.

## The signature and the seal

A verified version gets the “Verified by Escouade” seal: the review signs its `version.json`, hence its fingerprints, the module’s included, with a key that does not leave its machine. The app checks this signature itself, with its own keys. The seal stands for this version only: the next one is “Not verified” until its own review. What it covers is in [plugin security](plugins/security#what-the-seal-covers).

## Updates

Publishing a version changes nothing for those who have the plugin: Escouade **offers it, and never installs it alone**, with the same window as the install. If it asks for more, the window says so, marks “new” each extra permission and asks for agreement again. See [installing a plugin](plugins/installer#updates) and [the versions](plugins/versioning).

## Preparing for the review

The “Verified by Escouade” review is done by hand, version by version, on the Escouade team’s machine: it downloads the archive the store keeps, checks its fingerprints, reads the code against the declared permissions and runs the [conformance bench](plugins/testing) in a container without network. It then signs `version.json` with a key that never leaves that machine: neither the store nor the CI has it. What makes it quick:

- **exact permissions**: every host reached in `permissions.network`, every secret in `contributes.secrets`, nothing more;
- **readable code**: a compiled file (`dist/`) has its source in the repository and rebuilds from it; no minified code without its source, no text encoded in base64;
- **few dependencies**, at fixed versions, without a `postinstall` script or a dependency pulled from a git repository;
- **a short `Cargo.lock`** for a plugin in Rust: each dependency is read;
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
