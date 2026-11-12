# Le découpage du crate de l'app (ESC-408)

Demande de l'utilisateur (2026-11-11) : « découpe le crate de l'app ». La compilation du crate `escouade` (`src-tauri/`) en mode test est le long pôle de la CI : 12 à 27 min sur le Mac, l'essentiel de l'archive nextest sous Windows et Linux. Ce document mesure ce qui coûte, propose un découpage, dit ce qui est fait (§ 4) et ce qui reste, dans l'ordre (§ 5).

## En bref

- **Ce qui coûte** : l'unité « lib (test) » du crate, qui compile tout le code de l'app (≈ 200 000 lignes) *et* ses tests (≈ 140 000 lignes) en un seul rustc. Sur le poste (32 cœurs), 128 s à froid, dépendances en cache, contre 58 s pour la lib seule. Son front-end, sa collecte de monomorphisation et sa génération d'IR sont en série ; seul LLVM profite des cœurs.
- **La mémoire, surtout, pour le Mac** : « lib (test) » monte à **8,8 Go** (à 3 jobs) et la lib, qui compile en même temps, à **4,3 Go**. Un runner macOS arm64 de GitHub a 3 vCPU et **7 Go**. Les deux compilations ensemble y dépassent la mémoire de près du double : le Mac pagine. C'est l'explication la plus probable de ses 12 à 27 min (contre ≈ 2 min 15 ici à 3 jobs) et de leur variation.
- **Fait dans ce ticket** : quatre crates du workspace, sans changement de comportement : `escouade-base` (les langues du moment, les gardes de chemin, la ligne visible), `escouade-voice-local` (micro, whisper.cpp, VAD, Piper, eSpeak NG, ONNX Runtime), `escouade-vectors` (découpe, embeddings, index SQLite), `escouade-plan` (l'avancée d'un agent et ses fichiers de plan). 13 600 lignes sortent du crate (4 %).
- **Le gain est à la mesure des lignes sorties** : − 3 % sur « lib (test) » (128 → 124 s) et − 6 % sur la lib (58 → 55 s) sur le poste ; pic mémoire de « lib (test) » de 9,0 à 8,8 Go (§ 3) ; nul à 3 jobs, où le travail total ne baisse pas. Une recompilation après un changement local ne change pas (≈ 30 s) : un changement dans une crate sortie recompile aussi l'app.
- **La suite** : les gros morceaux (hôte des plugins, serveur MCP, harness) sont pris dans le cycle `core` ↔ `model` ↔ le reste. Il faut d'abord sortir les types de données (`escouade-model`), puis l'hôte des plugins (≈ 40 000 lignes avec ses tests, le meilleur rapport), puis le serveur MCP. Chaque morceau a son ticket (§ 5). Deux leviers plus simples, à mesurer sur la CI, réduisent le travail ou la mémoire tout de suite (§ 6).

## 1. Les mesures avant

Toutes sur le poste de développement (Windows, 32 cœurs, autres worktrees qui compilent à côté : les mesures varient de ± 10 % ; les comparaisons qui comptent sont faites dos à dos, même charge). Conditions de la CI : `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, dépendances en cache, `cargo clean -p` des crates du workspace seulement.

| Mesure (`main`, a9326dfd) | Temps |
|---|---|
| `cargo build --tests`, crates du workspace à froid | 129 s (« lib (test) » 128 s, lib 58 s, en parallèle) |
| même chose à 3 jobs (`-j3`) | 135 s |
| `cargo nextest archive --profile ci` (14 binaires) | 133 s |
| recompilation après un changement d'un module (incrémental, comme en développement) : `voice/vad.rs`, `plan.rs`, `search.rs` | 31 s, 30 s, 30 s |

Le détail de « lib (test) » (`-Z time-passes`, 136 s ce jour-là) :

| Passe | Temps | En parallèle ? |
|---|---|---|
| expansion des macros | 6 s | non |
| typage (`type_check_crate`) | 22 s | non |
| borrowck (`MIR_borrow_checking`) | 35 s | non |
| collecte de monomorphisation | 17 s | non |
| génération de l'IR LLVM | 26 s | non |
| passes LLVM | 42 s | oui |
| édition des liens | 5 s | non |

Pic mémoire : ≈ 10 Go avec tous les cœurs, 8,8 Go à 3 jobs ; la lib, ≈ 4,3 Go.

## 2. Le graphe des modules

Le crate a 342 820 lignes de Rust (`src-tauri/src`), dont 140 444 de tests (`*_tests.rs`, `tests.rs`).

- **Un grand cycle** : 76 modules (105 000 lignes de code hors tests) se tiennent les uns les autres par `core` (cité par 87 modules), `model` (136), `paths` (108) et `i18n` (103). `model.rs` lui-même cite 24 modules : c'est là que vivent `UiEvent`, `AgentMeta` et les réglages, qui nomment des types de `forge`, `plan`, `voice`, `mcp`, `docs`, `workflow_track`, `branchci`, `prreview`… Tant que `model` est dans le cycle, rien de ce qui lit les réglages ou un projet ne peut sortir.
- **Les feuilles** sans lien vers le reste : `i18n`, `text`, `pricing`, `menus`, `fileorigin`, et des fichiers isolés (`agent/sidechain`, `engine/*/words`, `engine/opencode/rules`, `engine/jsonc`, `plugins/beside`, `plugins/journal`, `plugins/store_keys`…).
- **Les dépendances lourdes et qui les tire** : wasmtime (`plugins/runtime`, `plugins/wasm`), whisper-rs, cpal, espeak-ng, ort (`voice/*`), tokenizers et ort (`vectors/embed`), rusqlite (`vectors/index`, `stats`), rmcp et hyper (`mcp/*`, `updates/net`), pdf-extract, zip et quick-xml (`docsextract`), portable-pty (`pty`, `job`), keyring (`integrations/secrets`). Ces dépendances sont en cache : ce qu'elles coûtent au crate, c'est le code qui les appelle, pas leur compilation.
- **Les sous-systèmes et leurs tests** :

| Sous-système | Code | Tests | Fichiers de test qui démarrent un `core` | Ce qui le tient au reste |
|---|---|---|---|---|
| hôte des plugins (`plugins/`, `plugins_*_tests.rs`) | 20 400 | 21 500 | 3 | `plugins/harness.rs` et `plugins/events.rs` (agent, core, engine, pty, testlaunch), `model::Settings`, `integrations::secrets`, `forge` |
| serveur MCP (`mcp/`) | 13 500 | 22 000 | 19 | `Core` partout (les outils agissent sur le cœur) |
| harness (`engine/`, `agent/`) | 28 200 | 8 500 | 1 | `core`, `accounts`, `model`, `tui`, `plugins` |
| hébergeurs et suivis (`forge/`, `integrations/`) | 7 000 | 8 000 | 1 | `core`, `plugins` (les hôtes sont des plugins), `tickets` |
| mises à jour (`updates/`) | 700 (+ 3 000 dans `updates.rs`) | 2 600 | 0 | `model::Settings`, `integrations::fake` |
| voix en ligne et conversation (`voice/`) | 3 300 | 5 300 | 2 | `accounts`, `integrations::secrets`, `plugins::runtime` |
| tests du cœur (`core_tests`, `tickets_tests`, `tui_tests`…) | — | ≈ 40 000 | tous | `core` |

**Pourquoi le gain suit les lignes.** rustc compile un crate en un seul processus dont le front-end est en série. Une crate à part compile dans un autre processus, en parallèle ; l'app n'attend que ses métadonnées (cargo pipeline les crates de bibliothèque), pas son code machine. Sortir un morceau retire donc du chemin critique ses tests et la génération de son code, mais pas son front-end, qui précède l'app. Et le travail total ne baisse pas : une crate avec des tests compile, elle aussi, son code deux fois (la lib et « lib (test) »). Sur une machine qui a des cœurs libres, le découpage gagne ; sur une machine de 3 cœurs déjà pleine, il ne gagne que la mémoire.

## 3. Les mesures après (dos à dos, même charge)

| Mesure | Avant (`main`) | Après (ce ticket) |
|---|---|---|
| `cargo build --tests`, crates du workspace à froid | 129 s | 127 s |
| « lib (test) » de `escouade` | 128,0 s | 124,0 s |
| lib de `escouade` | 57,9 s | 54,5 s |
| les quatre crates sorties (lib et tests) | — | 1 à 3 s chacune, en parallèle de l'app |
| même chose à 3 jobs | 135 s | 136 s |
| `cargo nextest archive --profile ci` | 133 s (14 binaires) | 128 s (18 binaires) |
| pic mémoire de « lib (test) », 3 jobs | 9 031 Mo | 8 762 Mo |
| pic mémoire de la lib, 3 jobs | 4 481 Mo | 4 275 Mo |
| recompilation après un changement de `vad.rs` (sorti) | 31 s | 31 s |
| … de `plan.rs` (sorti) | 30 s | 29 s |
| … de `search.rs` (resté) | 30 s | 30 s |

Une recompilation ne change pas : changer une crate sortie la recompile, puis l'app, qui en dépend ; changer l'app recompile l'app. Le gain incrémental ne viendra que des crates *au-dessus* de l'app, ou d'une app plus petite.

## 4. Ce qui est fait

Quatre crates du workspace de `src-tauri`, dans `crates/` à côté de celles des plugins, chacune `publish = false`, en GPL-3.0-or-later comme l'app (eSpeak NG l'impose à la voix), membre et `default-member` du workspace (l'archive nextest et clippy les prennent sans option). L'app les réexporte aux chemins qu'elle avait : aucun appelant ne change.

| Crate | Ce qu'elle contient | Ce que l'app réexporte | Lignes (dont tests) |
|---|---|---|---|
| `escouade-base` | `i18n` : les langues du moment (`ui`, `claude`, les statiques, `store`) ; `paths` : `contained`, `outside`, `relative_slash`, `strip_base`, `safe_origin` ; `text` : `visible_line` ; `testing` (feature) : `test_dir`, `make_dir_link`, `make_file_link` pour les tests des crates | `i18n::{ui, claude}`, `paths::{contained, outside, relative_slash, strip_base}`, `taskout::safe_origin`, `mcp::act::visible_line` | 476 |
| `escouade-voice-local` | `audio`, `source` (cpal), `transcribe` (whisper.cpp), `vad`, `speech` (Piper, eSpeak NG, ONNX Runtime, `voices.json`), `models` (téléchargement vérifié), `VoiceError` | `voice::{audio, source, speech, transcribe, vad, VoiceError}`, `voice::models::*` (plus `models::client`, qui lit les réglages, resté dans l'app) | 3 532 |
| `escouade-vectors` | `chunk`, `embed` (ONNX), `index` (SQLite), `model` (téléchargement), `eval`, leurs tests | `vectors::{chunk, embed, index, model}` ; `vectors::wiring` (le cœur) et deux tests qui lisent `DataDir` et un `core` restent dans l'app | 1 902 (610) |
| `escouade-plan` | `plan` (le réducteur de l'avancée d'un agent), `planfiles` (les plans et registres de superpowers), leurs tests | `crate::plan`, `crate::planfiles` (`use escouade_plan::{plan, planfiles}` à la racine) | 7 836 (4 894) |

Ce qui a dû s'ouvrir, et pourquoi :

- `plan::BgMemory`, `plan::Seen`, `PlanState::with_memory`, les champs `PlanState::{seen, memory}` et `PlanTask::call` deviennent `pub` : l'app et ses tests écrivent des plans en littéraux (`PlanState { tasks, ..Default::default() }`), ce qu'une autre crate ne peut faire si un champ est privé. Ces champs restent hors de ce que le plan sérialise (`#[serde(skip)]`).
- `escouade-voice-local` réexporte `ort` : les vecteurs lancent leur modèle avec le même ONNX Runtime, configuré une fois (ses deux blocs par cible et la feature `ort-dynamic`, que l'app transmet : `ort-dynamic = ["escouade-voice-local/ort-dynamic"]`).
- `escouade-voice-local::install_logging_hooks` remplace l'appel direct de l'app à `whisper_rs`.

Ce qui n'a pas bougé : `canon.rs` et tout fichier dont une empreinte dépend (aucun fichier de `crates/escouade-plugin-cli`, des goldens ni des fixtures de parité n'a changé) ; les versions des crates publiées ; les sons de test (`src-tauri/src/voice/testdata/`, lus par les tests de l'app et de `escouade-voice-local`) ; `docs/SPEC.md`. Déplacés : `voices.json` (`crates/escouade-voice-local/src/speech/`, que lisent `src/lib/conversation.test.ts` et `scripts/piper-voices.mjs`).

Écarté après étude : `docsextract` (pdf-extract, zip, quick-xml : 400 lignes, et ses aides de test servent aux tests du drive et du MCP) ; `updates/net` (il prend `model::Settings` : après `escouade-model`).

## 5. La suite, dans l'ordre

Chaque ligne est un ticket. Le gain attendu est celui de « lib (test) » sur une machine qui a des cœurs libres, en proportion des lignes sorties ; la mémoire de l'unité la plus grosse baisse dans la même proportion.

| Ordre | Crate | Ce qui y va | Ce qui reste à l'app | Le lien à couper d'abord | Lignes (tests) | Gain attendu |
|---|---|---|---|---|---|---|
| 1 | `escouade-model` | les types de données de `model.rs` (réglages, projets, tickets, agents) et de ce qu'ils nomment ; `paths::DataDir`, `proc`, `job`, `wsl`, `fence` | `UiEvent` et ce qui nomme des types de l'app ; `Core` | les 24 modules que cite `model.rs` : sortir les types qu'il nomme, ou garder dans l'app les variantes qui les portent | ≈ 9 000 | faible seul (≈ 3 %) ; il ouvre le reste |
| 2 | `escouade-plugins` | l'hôte : `manifest`, `runtime`, `wasm`, `beside`, `socket`, `store`, `store_keys`, `files`, `procs`, `journal`, `seed`, `sort`, `decide`, `view`, `reader`, `bench`, et les ≈ 18 000 lignes de `plugins_*_tests.rs` qui ne démarrent pas de `core` | `plugins/harness.rs`, `plugins/events.rs`, `plugins/builtin.rs` (le lien avec le cœur, les agents, les terminaux) | `integrations::secrets::SecretStore` en trait de la crate ; `forge::{RateLimit, bare_host}` dans la crate | ≈ 34 000 (18 000) | ≈ 10 % ; le plus rentable |
| 3 | `escouade-forge` | `forge/` (contrat des hébergeurs, comptes, `remote`), `integrations/{source, secrets, fake}` et leurs tests | `forge/connect.rs`, `integrations/sync.rs`, `integrations/text.rs` (le cœur) | après 2 | ≈ 12 000 (8 000) | ≈ 3 % |
| 4 | `escouade-mcp` | le serveur MCP et ses outils, sur un trait de l'hôte que `Core` implémente | l'implémentation du trait | `Core` en paramètre partout ; ses 19 fichiers de tests démarrent un `core` : il faut d'abord une crate de soutien des tests (`core_tests::harness`) | ≈ 35 000 (22 000) | ≈ 10 % ; le plus long |
| 5 | `escouade-engines` | les harness (`engine/*`) et la lecture de leurs flux (`agent/*`) | `engine/mod.rs` (le registre et le lien avec le cœur) | après 1 ; `Core` passé aux harness par un trait | ≈ 36 000 (8 500) | ≈ 8 % |
| 6 | `escouade-updates` | `updates/net.rs`, `updates/fake_server.rs` et leurs tests | `updates.rs` | après 1 (`Settings`) | ≈ 3 300 (2 600) | ≈ 1 % |

Les tickets, créés le 2026-11-12 : **ESC-410** (`escouade-model`, ligne 1), **ESC-411** (`escouade-plugins`, ligne 2, après ESC-410 et ESC-359, qui touche aussi l'hôte), **ESC-414** (`escouade-forge` et `escouade-updates`, lignes 3 et 6, après ESC-411), **ESC-412** (`escouade-engines`, ligne 5, après ESC-410), **ESC-413** (`escouade-mcp`, ligne 4, après ESC-411 et ESC-412). Les leviers du § 6 : **ESC-409**.

Ce qui ne se découpe pas sans changer la façon de tester : les ≈ 40 000 lignes des tests du cœur (`core_tests`, `tickets_tests`, `tui_tests`, `closed_tests`…), qui démarrent un `Core` et lisent ses internes.

## 6. Deux leviers plus simples pour la CI, mesurés ici

Ils réduisent le travail total ou la mémoire, ce que le découpage ne fait pas sur une machine pleine. À essayer sur la CI avant le reste du plan (ticket à part) :

1. **Les tests sans informations de débogage** (`CARGO_PROFILE_DEV_DEBUG=0` sur l'étape de l'archive) : à 3 jobs, 136 → 124 s (− 9 %), pic mémoire de « lib (test) » de 8,8 à 8,1 Go. Un échec garde son fichier et sa ligne (le message de panique les porte) ; une trace d'appels les perd.
2. **Le Mac sans les deux grosses compilations à la fois** : la lib (≈ 4,3 Go) et « lib (test) » (≈ 8,8 Go) compilent en même temps sur 7 Go. Les compiler l'une après l'autre (`cargo build --lib` puis l'archive) ou sans la lib (`--lib` seul pour `escouade`, si les binaires qu'il faut à côté des tests, `escouade-hook` et `fake_codex`, sortent dans une crate à eux : à 3 jobs, 136 → 130 s ici) évite la pagination. C'est le seul levier qui vise directement les 12 à 27 min du Mac.

## 7. Pour découper un morceau

- Une crate dans `crates/`, `workspace = "../../src-tauri"`, `publish = false`, ajoutée à `members` **et** à `default-members` de `src-tauri/Cargo.toml` (l'archive nextest, clippy et `cargo build --tests` ne prennent que les seconds).
- L'app la réexporte où le code était (`pub use escouade_x::{a, b};` dans le module, ou `use escouade_x::{a, b};` à la racine) : aucun appelant ne change.
- `cfg!(test)` et `#[cfg(test)]` d'une dépendance sont faux quand l'app compile ses tests. Ce qui en dépend reste dans l'app (`i18n::system`, `i18n::set`), ou passe par la feature `testing` d'`escouade-base`.
- Un statique partagé (la langue du moment) ne se copie jamais : il vit dans une crate et l'app le réexporte.
- Une struct que l'app écrit en littéral avec `..Default::default()` doit avoir tous ses champs publics.
- Les tests suivent leur code ; ceux qui démarrent un `core` restent dans l'app.
- Les deux verrous : `cargo metadata --locked` dans `src-tauri` **et** dans `plugins`.
- Mesurer dos à dos, même charge (`git checkout` de la base, `cargo clean -p` des crates du workspace, `--timings`), jamais contre une mesure d'un autre moment : le poste varie de ± 10 %.
