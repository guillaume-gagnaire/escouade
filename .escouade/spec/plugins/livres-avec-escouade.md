---
title: "Les plugins d'Escouade (la première main)"
subject: "plugins"
sources: ["docs/PLUGINS-DESIGN.md §8.7, §8.8, §8.10, §8.33","src-tauri/src/plugins/builtin.rs","src-tauri/src/plugins/mod.rs","src-tauri/src/plugins/runtime.rs","src-tauri/src/plugins/seed.rs","src-tauri/src/plugins/sort.rs","src-tauri/src/plugins_builtin_tests.rs","src-tauri/src/plugins_seed_tests.rs","src-tauri/src/plugins_wasm_tests.rs","plugins/seed.json","tests/fixtures/plugins/minimal/README.md","docs/security/audit-2026-11/rapport.md (branche ticket/esc-418), fiche SEC-27"]
tickets: ["ESC-210","ESC-221","ESC-285","ESC-217","ESC-56","ESC-58","ESC-280","ESC-281","ESC-327","ESC-282","ESC-225","ESC-223","ESC-224","ESC-283","ESC-226","ESC-338","ESC-339","ESC-344","ESC-351","ESC-349","ESC-348","ESC-345","ESC-350","ESC-367","ESC-370","ESC-373","ESC-354","ESC-394","ESC-404","ESC-355","ESC-435"]
decisions: [80,92,110,113,117,142,"la-premiere-main-vient-de-l-origine-et-l"]
compared: null
---

# Les plugins d'Escouade (la première main)

Depuis la 2.2 (ESC-355, Z3), l'app ne **donne** plus aucun plugin. Les plugins de première main (`dev.escouade.*`, `plugins/<id-court>/` du dépôt) sont des plugins à part entière : publiés, relus et signés sur le store, portés par l'installeur (l'amorce), installés par le chemin du store (origine `store`, marque `seed` pour ceux de l'amorce), rangés dans « Ajoutés » comme les autres. Ils se coupent, se mettent à jour sur un clic et se désinstallent. L'origine `builtin`, ses programmes `{escouade}/…`, le contrôle `check_program`, l'empreinte `builtin:<version>`, la règle « l'identifiant de l'app l'emporte » et le programme `escouade-plugins` ont disparu.

## Qui est de la première main (audit SEC-27, ESC-435)

- **La première main, c'est l'origine et l'identifiant ensemble** : un identifiant `dev.escouade.*` venu de l'amorce ou d'une version du store **signée** (le store ne prend ces identifiants que du dépôt de l'app, puis les signe). Une build de développement y ajoute les modules du dépôt (origine `workspace`) et ceux de l'amorce non signés de `scripts/seed-dev.sh` ; une release, jamais.
- **Un dossier en développement, un dossier copié ou un dépôt cloné sont toujours tiers, quel que soit leur identifiant** : leur manifeste est lu comme celui de n'importe qui, et l'identifiant `dev.escouade.*` leur est refusé (« identifiant réservé »). Une version du store vérifiée d'un autre auteur est lue comme un tiers aussi.
- Une entrée déjà installée en développement sous `dev.escouade.*` par une version d'avant reste dans la liste, mais ne se charge pas (son manifeste ne tient pas) : rien d'elle ne tourne.
- Ainsi, à l'exécution, l'identifiant `dev.escouade.*` suffit à dire la première main (variables refusées même déclarées, interpréteurs d'un harness décrit, parole sur la clôture) : seul ce qui vient de l'amorce ou du store signé peut le porter.
- Les plugins de `plugins/` restent jugés de la première main dans les tests : les e2e et les tests Rust les lancent depuis le dépôt (origine `workspace`), et le banc de conformité (`plugins/ci.mjs`) les joue avec `--first-party`.

## Le registre

- `src-tauri/src/plugins/builtin.rs` garde la liste des identifiants de la première main avec leur manifeste tel que le dépôt le tient (embarqué) : c'est **la seule liste des identifiants de l'amorce** (`builtin::seeds`, plus Goose et ceux sur demande), et ce qui nomme un plugin de l'amorce non installé ou dit ce qui manque au formulaire d'un de ses comptes. Rien ne tourne de ce registre dans une release.
- Chaque dossier `plugins/<id-court>/` en `run.wasm` y est, avec son `escouade-plugin.json`, son README (un seul ou un par langue), sa `LICENSE` (GPL-3.0-or-later) et son `CHANGELOG.md` ; un test le vérifie.
- `dev.escouade.minimal` est la démonstration des builds de développement et des tests seulement : un module (`tests/fixtures/plugins/minimal/`, son `plugin.wasm` commité, refait par `sh build.sh`), la poignée de main, deux méthodes d'essai, et le harness « Minimal » qui prouve le contrat des harness en process (voir *Le contrat des harness en process*), son programme étant `tests/fixtures/fake-minimal.mjs` dans les tests.

## Une build de développement : l'origine `workspace`

Une build de développement et les tests lancent les modules du dépôt (`scripts/plugins-wasm.sh`, `<target>/plugins-wasm/<id-court>/plugin.wasm`) pour chaque identifiant du registre dont l'amorce n'a pas les fichiers : origine `workspace`, rangés dans « Du dépôt », leur version celle de la build, leur `escouade.min` lu la sienne. Ils ne se désinstallent pas (ils reviendraient au lancement) et se coupent. Ce qui est installé d'un identifiant (l'amorce, le store) garde sa place : aucun plugin ne gagne par son identifiant. Un dossier en développement ne peut pas prendre l'identifiant d'un plugin du registre (il est réservé). Une release ne fait jamais d'entrée `workspace`.

## Une mise à jour depuis la 2.1

Une entrée « builtin » de `plugins.json` (ce que la 2.1 donnait) n'est plus lue comme un plugin : ses choix sont gardés comme « Garder ses réglages » les garde, ses secrets restent au trousseau, et l'entrée quitte la liste. Le tri de l'amorce installe ensuite ce que la configuration utilise, avec ces choix ; le reste est proposé, jamais noté désinstallé. Une entrée qui ne se lit pas ne fait plus perdre les autres.

## `run.command` ne démarre plus

Escouade ne lance plus aucun programme natif, **quelle que soit l'origine** du plugin (store, dossier, dépôt git, dossier en développement). Le manifeste reste lisible ; le plugin se charge coupé, et sa fiche dit pourquoi : « Escouade ne lance plus `run.command` : un plugin se publie en `run.js` ou `run.wasm`, que le store compile pour tous les systèmes. » La fenêtre d'installation le dit avant d'installer ; le banc de conformité (`escouade-plugin test`) échoue à l'initialisation avec la même raison. Le store le refuse à la soumission (contrôle « Binaires »). Un programme de l'ordinateur se lance par `permissions.commands`. Les versions publiées qui se lancent par Node (`slack` 1.1.0, `team-rules` 1.1.1) restent au store pour les apps d'avant, bornées par `escouade.max`.

## Un hébergeur ou une source de la première main

- Rien ne change à l'usage : les comptes, les formulaires, les PR sur la carte, l'import et la synchro des issues sont les mêmes.
- Le jeton d'un compte reste au coffre ; le cœur le prête au plugin qui le demande (`secrets/get` de `account:<hôte>`), celui de la CLI (`gh auth token`, `glab auth status --show-token`, `az account get-access-token`) compris.
- Chaque plugin reçoit la langue de l'interface à `initialize` ; un changement de langue relance ceux qui tournent. Le réseau de Réglages › Avancé › Réseau passe par l'hôte WASM (`net/fetch`), sans variable d'environnement.
- **Coupé, arrêté ou désinstallé**, il le dit : la carte de l'agent porte « Hébergeur arrêté : PR plus suivies » et garde les PR lues en dernier ; Réglages › Intégrations dit sous chaque compte que le plugin est coupé, arrêté ou absent.

## Les services de voix et les harness

- OpenAI, Groq, Mistral, ElevenLabs et Deepgram sont des modules de la première main ; Réglages › Voix se comporte comme avant (voir *Réglages › Voix*). Coupé, le service quitte la page et la dictée repasse en local. Un plugin d'un autre auteur peut servir un nouveau service ou prendre la place de l'un d'eux, à son identifiant à lui. Le plugin « Mistral » porte la voix et Mistral Vibe.
- Claude Code, Codex, Copilot CLI, Cursor CLI, Gemini CLI, OpenCode, Antigravity CLI et Mistral Vibe sont servis par leur plugin de première main ; voir leurs pages *… en plugin*. Le bouton projet par projet s'applique à leur harness (ESC-404) : coupé pour un projet, le harness n'y est plus proposé ; dans un projet privé, le premier agent demande « Activer ».
- Le mot d'un plugin qu'il tient lui-même la clôture des projets privés (`fence_by_client`) n'est pris que d'un plugin de première main (`dev.escouade.*`, qui ne vient que de l'amorce ou du store signé, jamais d'un dossier en développement), comme l'était celui d'un plugin donné par l'app.

## L'amorce (ESC-338, ESC-354)

L'installeur porte, pour chaque plugin de la première main, ce que le store a publié pour sa dernière version (`version.json`, sa signature, l'archive et `plugin.wasm`) et la liste de révocation du jour. La liste est `plugins/seed.json` (28 entrées : les harness, `dev.escouade.acp`, Goose, les sources et hébergeurs, la voix, isola, Discord, Slack, Contraste élevé, Règles de l'équipe).

- **Au premier lancement**, et après une mise à jour de l'app, Escouade installe ce que l'amorce tient, qui n'est pas installé **et que la configuration utilise** (le tri), sans réseau, par le chemin d'une installation du store : signature, empreintes, `dev.escouade.*` signé, révocation, `escouade.min` et `max`. Le module se précompile à sa première activation.
- **Une version refusée** n'installe rien, et la vue Plugins dit pourquoi sous « Proposés par Escouade ». **Une version retirée** (`disable`) s'installe coupée, sa raison dite ; un sceau retiré (`unverify`) n'installe rien.
- **Rien ne se met à jour seul** : la plus récente du store ou de l'amorce est proposée, et s'installe au clic ; on ne redescend jamais.
- **Ce qui est désinstallé** n'est pas remis au lancement : l'amorce le note, « Proposés par Escouade » le propose, et le geste qui le demande le remet, ses choix gardés.
- **Sans son plugin**, un agent dit que le plugin n'est plus installé et que la vue Plugins propose de l'installer.
- **Sans réseau ni store**, tout marche à la version installée ou à celle de l'amorce.
- La release télécharge l'amorce (`store/scripts/seed.ts`), vérifie chaque signature et échoue sinon : **le chef publie d'abord, la release ensuite**. Un build de développement accepte les modules non signés de `scripts/seed-dev.sh`.

## Le tri de l'amorce (ESC-373)

Décidé par l'utilisateur le 2026-11-07 (« je veux un tri ») : l'amorce n'installe que les plugins de ce que l'utilisateur a configuré, lu dans les réglages, les projets, les agents et les comptes, jamais sur le réseau :

- **Un harness** : un compte de ce harness, un agent sur lui (archivé compris), le harness par défaut d'un projet ou celui des textes rédigés ; `dev.escouade.acp` aussi quand un plugin installé décrit un harness par ACP.
- **Une source de tickets ou un hébergeur** : un projet qui l'a pour source, un projet dont un remote est chez lui, ou un compte de ce service déjà enregistré, sauf s'il est servi par le plugin d'un autre auteur.
- **Un service de voix** : choisi « En ligne », ou dont la clé est gardée (sa présence, jamais sa valeur).
- **isola** : un projet, ou l'un de ses dépôts, avec un `.isola.toml`.
- **Discord, Slack, Contraste élevé, Règles de l'équipe** : ce qui est installé le reste ; sinon, proposés.
- **Ce qui ne se lit pas sûrement s'installe** : le tri ne fait jamais perdre une fonction de la 2.1.

Une première installation n'installe rien d'office ; le geste qui demande un plugin absent (ajouter un compte, choisir une source ou un harness, ajouter un projet, choisir un service de voix, tester un worktree isola) l'installe depuis l'amorce, sans réseau, et le dit (« Plugin Codex installé »). Les listes de choix gardent tous les choix de l'amorce, installés ou non.

## La documentation d'un plugin de première main (ESC-370)

Le README d'un dossier de `plugins/` peut être un par langue (`README.fr.md` et `README.en.md`). La documentation du plugin est dans `docs/fr/` et `docs/en/`, les mêmes pages dans les deux langues, et le manifeste la déclare : `"store": { "docs": "docs" }`.

## Le compte des installations de l'amorce (ESC-394)

Les plugins que l'amorce installe comptent au store comme une installation, avec l'accord aux statistiques ; hors ligne, le compte attend ; il est oublié sur un refus ; une version ne compte qu'une fois. Une amorce de build de développement ne compte rien. Le détail est dans *Le store dans l'app*.
