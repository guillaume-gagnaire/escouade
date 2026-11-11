---
title: "Le retrait du natif (Z3)"
subject: "plugins"
sources: ["docs/PLUGINS.md §8.8, §8.10, §8.33","src-tauri/src/plugins/runtime.rs","src-tauri/src/plugins/mod.rs","src-tauri/src/plugins/builtin.rs","src-tauri/src/plugins/store.rs","crates/escouade-plugin-cli/src/manifest.rs","store/scripts/plugin-review/review.ts","store/server/utils/checks.ts","tests/fixtures/plugins/fake/plugin.mjs","tests/fixtures/plugins/minimal/README.md"]
tickets: ["ESC-355"]
decisions: []
compared: null
---

# Le retrait du natif (Z3, ESC-355)

Le dernier pas de *Tous les plugins en WASM* : ce que cette page appelait « l'état de fin » est l'état de l'app. Chaque plugin qui a un programme tourne en WASM dans Escouade (`run.js`, `run.wasm`) ; il ne reste aucun programme natif par système.

## Ce qui a disparu

- **`escouade-plugins`**, le programme des plugins que l'app donnait : plus construit, plus signé avec l'app, plus dans les installeurs (Windows, macOS, Linux), plus vérifié par la CI ni par `release.yml`.
- **Le transport par process** du runtime : plus de process de plugin, de Job, de mémoire lue toutes les deux secondes, ni de `pid` dans l'état d'un plugin. L'arrêt « trop de mémoire » d'un process n'existe plus : une instance qui dépasse sa mémoire (64 Mio, `limits.memoryBytes` jusqu'à 256 Mio) échoue comme un plantage et redémarre dans le même budget.
- **L'origine `builtin`**, `{escouade}/…`, `check_program` et sa vérification de signature, l'empreinte `builtin:<version>`, la règle « l'identifiant de l'app l'emporte » (un plugin du même identifiant installé d'ailleurs n'est plus refusé ni remplacé), la main `Builtin` du lecteur de manifeste et l'option `--builtin` d'`escouade-plugin`.
- **Le téléchargement du programme d'une release** à l'installation depuis le store : l'app ne lit plus `binaries` de `version.json`.
- Au store : `BUILTINS`, la confiance « Livré avec Escouade » ; dans la revue locale, `gh attestation verify` et le dossier `binaries/`. Le contrôle n° 10 refusait déjà `run.command` et ne demande plus de binaire par système.

## Ce qui reste

- Le contrat JSON-RPC, le cœur réduit, les harness décrits (`contributes.harnesses[].acp`, Goose) : le cœur lance le CLI d'un harness, jamais le programme d'un plugin.
- `slack` 1.1.0 et `team-rules` 1.1.1 restent au store pour les apps d'avant (`escouade.max`).
- Le registre de `builtin.rs`, comme liste des identifiants de l'amorce (voir *Les plugins d'Escouade*).

## Ce que l'utilisateur voit

- **Un plugin `run.command`**, quelle que soit son origine, se charge coupé : sa fiche et la fenêtre d'installation disent « Escouade ne lance plus `run.command` : un plugin se publie en `run.js` ou `run.wasm`, que le store compile pour tous les systèmes. » Le banc de conformité échoue à l'initialisation avec la même raison.
- **Une mise à jour depuis la 2.1** : les plugins que la 2.1 donnait sont installés depuis l'amorce par le tri, leurs choix gardés ; les autres sont proposés.
- **Une build de développement** lance les modules du dépôt que l'amorce n'a pas (origine `workspace`, « Du dépôt » dans la vue Plugins) ; une release jamais.
- La langue de l'interface est dite à chaque plugin à `initialize` ; la changer relance les plugins qui tournent.

## Les tests

Le faux plugin des tests est un module `run.js` (`tests/fixtures/plugins/fake/plugin.mjs`, ses options dans `args.json`) ; la démonstration `dev.escouade.minimal` un module `run.wasm` commité, son harness `tests/fixtures/fake-minimal.mjs`. Les tests qui se servaient du process passent sur eux ; ceux qui ne valaient que pour un process (le Job qui tue l'arbre, le `pid`, le chemin absolu d'un programme) sont partis. Les mesures, le poids de l'installeur et `bench:int0` sont dans `docs/PLUGINS.md` §8.33.
