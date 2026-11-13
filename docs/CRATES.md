# Le découpage du crate de l'app (ESC-408, ESC-410, ESC-411, ESC-412, ESC-413, ESC-414)

Demande de l'utilisateur (2026-11-11) : « découpe le crate de l'app ». La compilation du crate `escouade` (`src-tauri/`) en mode test est le long pôle de la CI : 12 à 27 min sur le Mac, l'essentiel de l'archive nextest sous Windows et Linux. Ce document mesure ce qui coûte, propose un découpage, dit ce qui est fait (§ 4) et ce qui reste, dans l'ordre (§ 5).

## En bref

- **Ce qui coûte** : l'unité « lib (test) » du crate, qui compile tout le code de l'app (≈ 200 000 lignes) *et* ses tests (≈ 140 000 lignes) en un seul rustc. Sur le poste (32 cœurs), 128 s à froid, dépendances en cache, contre 58 s pour la lib seule. Son front-end, sa collecte de monomorphisation et sa génération d'IR sont en série ; seul LLVM profite des cœurs.
- **La mémoire, surtout, pour le Mac** : « lib (test) » monte à **8,8 Go** (à 3 jobs) et la lib, qui compile en même temps, à **4,3 Go**. Un runner macOS arm64 de GitHub a 3 vCPU et **7 Go**. Les deux compilations ensemble y dépassent la mémoire de près du double : le Mac pagine. C'est l'explication la plus probable de ses 12 à 27 min (contre ≈ 2 min 15 ici à 3 jobs) et de leur variation.
- **Fait dans ce ticket** : quatre crates du workspace, sans changement de comportement : `escouade-base` (les langues du moment, les gardes de chemin, la ligne visible), `escouade-voice-local` (micro, whisper.cpp, VAD, Piper, eSpeak NG, ONNX Runtime), `escouade-vectors` (découpe, embeddings, index SQLite), `escouade-plan` (l'avancée d'un agent et ses fichiers de plan). 13 600 lignes sortent du crate (4 %).
- **Le gain est à la mesure des lignes sorties** : − 3 % sur « lib (test) » (128 → 124 s) et − 6 % sur la lib (58 → 55 s) sur le poste ; pic mémoire de « lib (test) » de 9,0 à 8,8 Go (§ 3) ; nul à 3 jobs, où le travail total ne baisse pas. Une recompilation après un changement local ne change pas (≈ 30 s) : un changement dans une crate sortie recompile aussi l'app.
- **`escouade-model` (ESC-410, § 9)** : les types de données (réglages, projets, tickets, agents) et ce qu'ils nomment, `paths`, `proc`, `job`, `wsl`, `heat`, `pricing` et la partie données de `fence` sortent de l'app (≈ 9 300 lignes, 2,9 %) ; en moyenne de deux passes, « lib (test) » 176 → 139 s et pic mémoire 8,2 → 7,6 Go (§ 9), un gain de temps bien au-delà des lignes sorties et à confirmer sur la CI, le poste ayant varié de ± 50 % ce jour-là. `model` n'est plus dans le cycle de `core` : ce qui ne lit que les données peut sortir.
- **`escouade-plugins` (ESC-411, § 10)** : l'hôte des plugins (registre, runtime, WASM, store, graine, banc, TLS) et ses tests qui ne démarrent pas de `core` sortent de l'app (≈ 22 100 lignes, 7 %) ; en moyenne, « lib (test) » 117 → 108 s (− 8 %) et son pic à 3 jobs 7,8 → 7,1 Go, mais l'app commence 6 s plus tard (elle attend la crate) et l'archive a pris 20 s de plus ici (un binaire de test de plus à lier), à confirmer sur la CI. Restent dans l'app les harness en processus, le puits des faits, les vues, les comptes et adaptateurs des hébergeurs.
- **`escouade-engines` (ESC-412, § 11)** : seules les parties des harness sans lien sortent (où est le programme de chaque harness, les configs de l'utilisateur qu'Escouade écrit, le serveur MCP tel qu'un harness le reçoit, la connexion de Claude Code, le transcript d'un sous-agent, ce qu'un item d'outil garde : 4 145 lignes, dont 1 326 de tests, 1,3 %) ; aucun gain mesurable, ni en temps ni en mémoire. Le reste ne se tient pas sans l'app : le trait `Engine` prend `AgentRt` et `Effects`, et la lecture des flux (`agent/*`) est faite de blocs `impl AgentRt` ; il faut d'abord sortir l'état d'un agent (§ 11).
- **`escouade-forge` et `escouade-updates` (ESC-414, § 12)** : les hébergeurs (contrat, comptes, clients), les sources (comptes, secrets du trousseau, registre), les adaptateurs des plugins qui les servent, avec leurs tests et ceux des plugins de tickets qui ne démarrent pas de `core` ; et ce que l'app apprend d'une requête de l'updater qui a échoué (17 200 lignes, 5,9 %). En moyenne de deux passes, « lib (test) » 124 → 107 s (− 13 %) et son pic à 3 jobs 7,2 → 6,8 Go, sous les 7 Go d'un runner macOS pour la première fois ; l'archive ne gagne rien ici (deux binaires de tests de plus).
- **`escouade-mcp` (ESC-413, § 13)** : seul ce qui se tient sans le cœur sort (le transport HTTP et ses gardes, sur un trait de l'hôte que `Core` implémente ; les jetons et le port ; le journal ; les arguments des outils et leurs schémas ; la partie données de l'installation chez les harness : 1 832 lignes, 0,6 %) ; outils, schémas et textes identiques octet pour octet. « lib (test) » − 2 %, pic mémoire − 144 Mo, archive + 9 s ici. Les outils et leurs 22 000 lignes de tests restent : ils agissent sur `Core` (79 méthodes et champs, 30 modules de l'app) et les 19 fichiers de tests démarrent un vrai cœur ; ce qu'il faudrait est au § 13.
- **La suite** : les gros morceaux (hôte des plugins, serveur MCP, harness) sont pris dans le cycle `core` ↔ `model` ↔ le reste. Il faut d'abord sortir les types de données (`escouade-model`), puis l'hôte des plugins (≈ 40 000 lignes avec ses tests, le meilleur rapport), puis le serveur MCP. Chaque morceau a son ticket (§ 5). Deux leviers plus simples, à mesurer sur la CI, réduisent le travail ou la mémoire tout de suite (§ 6).
- **Les leviers de la CI (ESC-409, § 8)** : gardés, les tests sans informations de débogage (les trois OS : Linux − 14 % sur l'archive, le Mac − 4 %, Windows =) et la lib du Mac compilée avant ses tests (son archive de 13 min 11 à 11 min 52, son job de 16 min 01 à 14 min 13 : un seul run, à confirmer par ceux de `main`). Écarté : l'archive du Mac compilée sur un Mac Intel pour Apple Silicon (job de 24 min 26, cache chaud : les runners Intel sont lents).

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
| 1 | `escouade-model` | **fait (ESC-410, § 9)** | `UiEvent`, `UsageSnapshot`, `AccountUsage` ; le lien de `fence` avec le cœur | — | 9 300 (2 760) | mesuré : « lib (test) » − 21 %, pic − 0,57 Go (§ 9) |
| 2 | `escouade-plugins` | **fait (ESC-411, § 10)** ; prévu : l'hôte : `manifest`, `runtime`, `wasm`, `beside`, `socket`, `store`, `store_keys`, `files`, `procs`, `journal`, `seed`, `sort`, `decide`, `view`, `reader`, `bench`, et les ≈ 18 000 lignes de `plugins_*_tests.rs` qui ne démarrent pas de `core` | `plugins/harness.rs`, `plugins/events.rs`, `plugins/builtin.rs` (le lien avec le cœur, les agents, les terminaux) | `integrations::secrets::SecretStore` en trait de la crate ; `forge::{RateLimit, bare_host, host_of, remote::parse}` dans la crate ; revu après ESC-410 : `files.rs` prend déjà `Fence` dans `escouade-model`, restent `procs.rs` (`isola::cli`), `mod.rs` (`git::run`, `engine::registry::Version`), `shared.rs` (`engine::Program`), `reader.rs` (`workflow`), `bench` (`facts::NAMES`) et `adapters.rs` (`integrations::source`, `text::default_states`) | ≈ 34 000 (18 000) ; sorti : 22 100 (6 800 de fichiers de tests) | mesuré : « lib (test) » − 8 %, pic − 0,7 Go ; archive + 20 s ici (§ 10) |
| 3 | `escouade-forge` | **fait (ESC-414, § 12)** ; prévu : `forge/` (contrat des hébergeurs, comptes, `remote`), `integrations/{source, secrets, fake}` et leurs tests | `forge/connect.rs`, `integrations/sync.rs`, `integrations/text.rs` (le cœur) | après 2 | ≈ 12 000 (8 000) ; sorti : 14 900 (9 100 de fichiers de tests) | mesuré : « lib (test) » − 13 %, pic − 0,36 Go (§ 12) |
| 4 | `escouade-mcp` | **en partie (ESC-413, § 13)** : le transport, les jetons, le port, le journal, les arguments des outils ; prévu : le serveur MCP et ses outils, sur un trait de l'hôte que `Core` implémente | l'implémentation du trait | `Core` en paramètre partout ; ses 19 fichiers de tests démarrent un `core` : il faut d'abord une crate de soutien des tests (`core_tests::harness`) | ≈ 35 000 (22 000) ; sorti : 1 832 (5 tests) | mesuré : « lib (test) » − 2 %, pic − 144 Mo, archive + 9 s ici (§ 13) ; le reste demande des traits par famille d'outils (§ 13) |
| 5 | `escouade-engines` | **en partie (ESC-412, § 11)** : les parties sans lien ; restent les harness et la lecture de leurs flux | `engine/mod.rs` (le registre et le lien avec le cœur) | il faut d'abord sortir l'état d'un agent (`AgentRt`, `Effects`), que le trait `Engine` et `agent/*` prennent, sur un trait de l'hôte (§ 11) ; puis les comptes et le lancement par les plugins, en trait | fait : 4 145 (1 326) ; reste ≈ 21 000 (≈ 11 500) | mesuré : rien ; reste ≈ 10 % |
| 6 | `escouade-updates` | **fait (ESC-414, § 12)** : `updates/net.rs`, `updates/fake_server.rs`, `net_tests.rs` (`flow_tests.rs` teste `updates.rs` : il reste) | `updates.rs` | possible dès maintenant : revu après ESC-410, `net.rs` ne prend plus que `i18n` (`escouade-base`) et `model` (`escouade-model`) ; ses tests prennent `integrations::fake::FakeServer`, à sortir avec lui ou dans une aide de test | ≈ 3 300 (2 600) ; sorti : 2 400 (1 400) | mesuré avec la ligne 3 |

Les tickets, créés le 2026-11-12 : **ESC-410** (`escouade-model`, ligne 1), **ESC-411** (`escouade-plugins`, ligne 2, après ESC-410 et ESC-359, qui touche aussi l'hôte), **ESC-414** (`escouade-forge` et `escouade-updates`, lignes 3 et 6, après ESC-411), **ESC-412** (`escouade-engines`, ligne 5, après ESC-410), **ESC-413** (`escouade-mcp`, ligne 4, après ESC-411 et ESC-412). Les leviers du § 6 : **ESC-409**.

Ce qui ne se découpe pas sans changer la façon de tester : les ≈ 40 000 lignes des tests du cœur (`core_tests`, `tickets_tests`, `tui_tests`, `closed_tests`…), qui démarrent un `Core` et lisent ses internes.

## 6. Deux leviers plus simples pour la CI, mesurés ici

Ils réduisent le travail total ou la mémoire, ce que le découpage ne fait pas sur une machine pleine. À essayer sur la CI avant le reste du plan (ticket à part) :

1. **Les tests sans informations de débogage** (`CARGO_PROFILE_DEV_DEBUG=0` sur l'étape de l'archive) : à 3 jobs, 136 → 124 s (− 9 %), pic mémoire de « lib (test) » de 8,8 à 8,1 Go. Un échec garde son fichier et sa ligne (le message de panique les porte) ; une trace d'appels les perd.
2. **Le Mac sans les deux grosses compilations à la fois** : la lib (≈ 4,3 Go) et « lib (test) » (≈ 8,8 Go) compilent en même temps sur 7 Go. Les compiler l'une après l'autre (`cargo build --lib` puis l'archive) ou sans la lib (`--lib` seul pour `escouade`, si les binaires qu'il faut à côté des tests, `escouade-hook` et `fake_codex`, sortent dans une crate à eux : à 3 jobs, 136 → 130 s ici) évite la pagination. C'est le seul levier qui vise directement les 12 à 27 min du Mac.

ESC-409 les a préparés, avec un troisième (l'archive du Mac compilée sur un Mac Intel) : § 8.

## 7. Pour découper un morceau

- Une crate dans `crates/`, `workspace = "../../src-tauri"`, `publish = false`, ajoutée à `members` **et** à `default-members` de `src-tauri/Cargo.toml` (l'archive nextest, clippy et `cargo build --tests` ne prennent que les seconds).
- L'app la réexporte où le code était (`pub use escouade_x::{a, b};` dans le module, ou `use escouade_x::{a, b};` à la racine) : aucun appelant ne change.
- `cfg!(test)` et `#[cfg(test)]` d'une dépendance sont faux quand l'app compile ses tests. Ce qui en dépend reste dans l'app (`i18n::system`, `i18n::set`), ou passe par la feature `testing` d'`escouade-base`.
- Un statique partagé (la langue du moment) ne se copie jamais : il vit dans une crate et l'app le réexporte.
- Une struct que l'app écrit en littéral avec `..Default::default()` doit avoir tous ses champs publics.
- Les tests suivent leur code ; ceux qui démarrent un `core` (ou un `AgentRt`) restent dans l'app.
- Un bloc `impl` d'un type de l'app dans un fichier sorti (`impl AgentRt` de `agent/sidechain.rs`) reste dans un fichier de l'app.
- `tr!` est une macro de l'app : la crate a la sienne, sur `escouade_base::i18n::ui()` (`escouade-engines`).
- `env!("CARGO_PKG_VERSION")` dans une crate est la sienne : ce qui disait la version de l'app la lit autrement (le script de build de `escouade-plugins`). `env!("CARGO_MANIFEST_DIR")` d'un test déplacé : `concat!(env!("CARGO_MANIFEST_DIR"), "/../../src-tauri")` garde ses chemins relatifs.
- Les aides qu'un test déplacé prête aux tests restés : le fichier entier aussi sous la feature `testing` (ses `#[test]` en sont absents), et `use escouade_x::fichier_tests;` à la racine de l'app.
- Une mesure de mémoire (`RUSTC_BOOTSTRAP=1`) fait recompiler les dépendances à la compilation suivante : une compilation de chauffe avant chaque mesure de temps.
- Un fichier resté sous un `mod.rs` sorti (`use super::*`) prenait ses `use` privés, que `pub use escouade_x::module::*;` ne réexporte pas : le nouveau `mod.rs` de l'app les reprend. Deux globs qui donnent le même nom (`model::*` et la crate) : un `pub use` explicite tranche (`integrations::Account`).
- Les fixtures d'un test déplacé (`include_str!("../../../tests/…")`) prennent un niveau de plus depuis `crates/x/src/<dossier>`.
- Compter les `#[test]` du workspace avant et après (`git archive` de la base) : même total, aucun test perdu.
- Les deux verrous : `cargo metadata --locked` dans `src-tauri` **et** dans `plugins`.
- Mesurer dos à dos, même charge (`git checkout` de la base, `cargo clean -p` des crates du workspace, `--timings`), jamais contre une mesure d'un autre moment : le poste varie de ± 10 %.

## 8. Les leviers de la CI (ESC-409)

Trois leviers, un commit chacun, poussés et mesurés un à la fois sur la CI (deux CI à la fois se gênent). Gardés : 1 et 2. Écarté : 3 (§ 8.3), que la branche retire.

| Ordre | Commit | Levier | OS touchés |
|---|---|---|---|
| 1 | `ci(tests): l'archive nextest sans informations de débogage…` | `CARGO_PROFILE_DEV_DEBUG=0` sur l'étape de l'archive | les trois |
| 2 | `ci(macos): la lib de l'app compilée seule, avant ses tests` | `cargo test --no-run --bins` avant l'archive, sur le Mac | macOS |
| 3 (écarté) | `ci(macos): l'archive des tests compilée sur un Mac Intel…` | `macos-build` sur `macos-15-intel`, `--target aarch64-apple-darwin` ; retire l'étape du levier 2 | macOS |

### 8.1 Les mesures locales

Sur le poste (Windows, 32 cœurs, d'autres worktrees qui compilent à côté), à 3 jobs (`CARGO_BUILD_JOBS=3`, comme un runner macOS), `CARGO_INCREMENTAL=0`, dépendances en cache, les crates du workspace à froid (`cargo clean -p`), `cargo nextest archive --profile ci`. Deux passes par cas, en alternance. La mémoire est le pic de l'ensemble de travail des `rustc` de ce worktree, échantillonné toutes les 0,5 s.

| Cas | Passe 1 | Passe 2 | Moyenne | Pic de « lib (test) » | Pic des `rustc` ensemble |
|---|---|---|---|---|---|
| avant (`line-tables-only`) | 170 s | 155 s | 162 s | 8,4 à 8,6 Go | 9,1 Go |
| levier 1 (`DEBUG=0`) | 148 s | 138 s | **143 s (− 12 %)** | 7,9 à 8,2 Go | 8,7 Go |
| levier 2 (la lib d'abord, `line-tables-only`) | 66 + 133 s | 66 + 143 s | 205 s (+ 26 %) | 8,6 Go | 8,6 Go |
| leviers 1 et 2 | 61 + 123 s | 62 + 130 s | 188 s (+ 31 % sur le levier 1 seul) | 7,9 à 8,1 Go | 7,9 à 8,1 Go |

Ce qu'elles disent :

- **Le levier 1 gagne partout** : − 12 % de temps et ≈ − 0,5 Go, sans rien changer d'autre. Les dépendances gardent leurs tables de lignes (`[profile.dev.package."*"]`), donc rien ne se recompile hors du workspace.
- **Le levier 2 gagne peu de mémoire** : la lib finit pendant le front-end de « lib (test) », avant son pic. Les deux ensemble montent à 9,1 Go, à peine plus que « lib (test) » seule (8,6 Go). En série, il perd ≈ 40 s ici (sur 32 cœurs, sans pagination). Sur le Mac il retire ≈ 0,5 Go à une compilation qui dépasse 7 Go de toute façon : il ne gagnera que si ce demi-giga fait la différence entre paginer un peu et paginer beaucoup. À mesurer, sans trop en attendre.
- **Le seul levier qui fait tenir la mémoire est le 3** : « lib (test) » seule, même sans débogage, prend 7,9 à 8,2 Go, au-delà des 7 Go d'un runner macOS arm64. Le levier 3 ne se mesure pas ici (il faut un Mac) ; sur la CI, la lenteur des runners Intel l'emporte sur la mémoire gagnée (§ 8.3).

`cargo build --lib` ne convient pas pour le levier 2 : l'archive recompile alors la lib (vu ici), parce qu'elle la veut au profil `test` et avec les fonctionnalités que les dev-dependencies ajoutent. `cargo test --no-run --bins`, sur les mêmes paquets que l'archive, construit exactement la lib qu'elle prend (vérifié : l'archive ne recompile ensuite que « lib (test) » et les binaires).

### 8.2 Les mesures avant, sur la CI

Les cinq derniers runs de `main` (11 et 12 novembre), le temps de l'étape `Rust tests, built once (nextest archive)` et celui du job `… (build)` :

| Run (commit) | macOS, archive / job | Windows, archive / job | Linux, archive / job |
|---|---|---|---|
| 5f4f97b3 | 11 min 50 / 14 min 18 | 6 min 49 / 14 min 53 | 3 min 32 / 6 min 38 |
| af64aa78 | 20 min 45 / 24 min 27 | 6 min 47 / 15 min 03 | 3 min 12 / 6 min 37 |
| a9326dfd | 14 min 21 / 18 min 05 | 5 min 40 / 13 min 07 | 2 min 41 / 5 min 48 |
| 449064f6 (ESC-408) | 16 min 00 / 20 min 29 | 7 min 14 / 19 min 56 | 3 min 22 / 6 min 57 |
| a182e670 (la base des leviers) | 13 min 43 / 17 min 04 | 6 min 32 / 14 min 55 | 2 min 56 / 6 min 19 |

Le Mac varie du simple au double d'un run à l'autre (12 à 21 min pour le même travail) : c'est la signature de la pagination. Sous Windows et Linux, l'archive est stable à ± 15 %.

### 8.3 Les mesures après, sur la CI

Un run à la fois, depuis la base a182e670 (poussés et lancés par le chef) :

| Run (commit) | macOS, archive / job | Windows, archive / job | Linux, archive / job | Gardé ? |
|---|---|---|---|---|
| levier 1 (c6cccb32, run 43158121992, vert) | 13 min 11 / 16 min 01 (− 4 % / − 6 %) | 6 min 32 / 14 min 51 (=) | 2 min 32 / 5 min 51 (− 14 % / − 7 %) | oui : il ne perd nulle part |
| leviers 1 et 2 (63b812b4, run 43161522265, vert) | 1 min 41 (la lib) + 10 min 11 = 11 min 52 / 14 min 13 (− 10 % / − 11 % sur le levier 1) | 4 min 21 / 11 min 16 (levier 2 sans effet ici : du bruit) | 3 min 09 / 6 min 14 (idem) | **oui** |
| leviers 1 et 3, 1er essai (b1822ac2, run 43164641510, annulé) | 24 min 00 / 31 min 59, **cache froid** (clé `macos-intel` neuve : toutes les dépendances compilées, 3 min 24 pour sauver le cache) ; tests Apple Silicon : 292 échecs, délai de 40 min dépassé | 6 min 26 / 15 min 23 | 2 min 35 / 5 min 48 | refait avec 64444fcc |
| leviers 1 et 3, 2e essai (64444fcc, run 43173620742, cache chaud, annulé pendant les tests du Mac) | 15 min 04 / 24 min 26 (restauration du cache 2 min 24, modules des plugins 5 min 20, contre 1 min 08 sur Apple Silicon) | 5 min 11 / 13 min 03 | 2 min 33 / 6 min 09 | **non** : + 10 min sur le job du Mac |

Le levier 1 gagne sous Linux ce qu'il gagnait ici ; sous Windows rien ; sur le Mac 30 s, dans le bruit d'un Mac qui varie de 12 à 21 min : la pagination, que la mémoire gagnée (≈ 0,5 Go) ne suffit pas à éviter, domine toujours.

Le levier 2 gagne sur le Mac plus que les mesures d'ici ne le laissaient attendre (un run seul : à confirmer par les suivants de `main`) : sans la lib en même temps, « lib (test) » pagine moins. Sous Windows et Linux, qu'il ne touche pas, les écarts d'un run à l'autre (± 2 min sous Windows) disent le bruit.

Le premier essai du levier 3 a montré deux choses. L'archive se compile sur l'Intel pour Apple Silicon, et ses binaires tournent sur les runners arm64 (927 tests passés avant le délai). Mais avec `--target`, les tests sont dans `target/aarch64-apple-darwin/debug` et cherchent les modules des plugins un niveau au-dessus (`app_dir/../plugins-wasm`, `plugins/builtin.rs`) : ils n'y étaient pas, le faux Claude ne démarrait pas, et chaque échec retenté deux fois a fait dépasser les 40 min. 64444fcc (le chef ; 9022e358 ici, équivalent) les copie là et les met dans l'archive (`.config/nextest.toml`) ; la même disposition, simulée sous Windows avec `--target x86_64-pc-windows-msvc`, passe ses 4 554 tests. Le 2e essai doit partir **sur la même branche** (`ci/esc-409-levier-3`) : GitHub ne donne à une branche que ses caches et ceux de `main`, et le cache `macos-intel` n'existe que là.

Le second essai du levier 3, cache chaud, perd : le runner Intel n'a pas attendu (parti avec les autres jobs), mais tout y est plus lent. L'archive y prend 15 min 04 (11 min 52 lib comprise sur Apple Silicon), les modules des plugins 5 min 20 (1 min 08), la restauration du cache 2 min 24 ; le job 24 min 26 contre 14 min 13. La mémoire gagnée ne rattrape pas des cœurs Intel plus lents. Le run a été annulé pendant les tests du Mac pour libérer les runners : on ne sait pas si l'archive croisée passe tous ses tests (au 1er essai, 927 étaient passés avant le délai). Le levier est écarté ; la branche revient à `macos-latest`, sans `--target`, sans la copie des modules ni la ligne `aarch64-apple-darwin/plugins-wasm` de `.config/nextest.toml`.

Un seul run par levier : les prochains runs de `main` diront si le levier 2 tient, le Mac variant de 12 à 21 min d'un run à l'autre.

### 8.4 Ce que chaque levier change à ce qu'un échec montre

- **Levier 1.** Un test qui échoue montre son message de panique avec son fichier, sa ligne et sa colonne (`panicked at src/x.rs:12:5`) : ils viennent de `Location`, compilé dans le binaire, pas des informations de débogage. La sortie de nextest et les annotations (`scripts/nextest-annotations.mjs`) ne changent donc pas. Ce qui se perd : les fichiers et lignes d'une trace d'appels (`RUST_BACKTRACE=1`). Sans informations de débogage, cargo retire aussi celles des dépendances au lien (`strip = "debuginfo"` par défaut) : sous Linux et macOS une trace garde les noms des fonctions, sous Windows (pas de PDB) elle n'en a plus. La CI ne demande jamais de trace (aucun `RUST_BACKTRACE` dans les workflows ni dans `.config/nextest.toml`) : ce qu'un échec y montre ne change pas, d'où le levier sans question à l'utilisateur. Pour une trace, on relance le test en local, avec le profil de développement.
- **Levier 2.** Rien : mêmes binaires.
- **Levier 3 (écarté).** Rien de ce que vérifient les tests : les mêmes binaires pour Apple Silicon (`aarch64-apple-darwin`), exécutés sur les mêmes runners Apple Silicon (`macos-rust`). Ce qui change : les scripts de build et les macros procédurales tournent sur Intel (whisper.cpp et ggml compilés par cmake pour arm64 depuis un Intel, `GGML_NATIVE=OFF` de `.cargo/config.toml` les garde du processeur de la machine ; ONNX Runtime téléchargé pour la cible) ; le cache Rust de ce job est à lui (`macos-intel`), le bundle garde `macos`. Ce que la CI y perd : la dépendance à une image que GitHub retire (`macos-15-intel`, sa dernière image Intel, jusqu'à l'automne 2027, août selon certaines sources) et des runners Intel moins nombreux (une attente possible). Tout faire sur l'Intel, tests compris, perdrait davantage : les tests ne tourneraient plus sur Apple Silicon, la machine des utilisateurs (ggml en NEON et Metal, ONNX Runtime lié en statique, que l'Intel ne prend pas : il charge la bibliothèque de Microsoft) ; écarté.

### 8.5 Écarté : l'archive sans la lib non-test

Sortir `escouade-hook` et `examples/fake_codex` dans une crate à eux (et `test = false` sur les autres binaires et exemples de l'app) retirerait la lib de l'archive. Mais Tauri ne met dans le paquet que les binaires du paquet de l'app : `escouade-hook` deviendrait un `externalBin` (nommé par cible, `scripts/universal-hook.sh` et `scripts/windows-signing.mjs` à revoir, le Release à refaire). Pour un gain que les mesures du § 8.1 bornent : la lib ne coûte que ≈ 0,5 Go au pic, et ≈ 60 s de CPU sur 3 cœurs. Le levier 2, qui compile la lib à part, en prend déjà l'essentiel sur le Mac, sans toucher au paquet.

## 9. `escouade-model` (ESC-410)

Ligne 1 du § 5. La crate `crates/escouade-model` (mêmes règles qu'au § 4 : `publish = false`, GPL-3.0-or-later, membre et `default-member`) prend ce qui n'est que des données et ce qui ne lit que des données. L'app réexporte tout à ses chemins : `pub use escouade_model::model::*;` dans `model.rs`, `pub use escouade_model::paths::*;` dans `paths.rs`, `pub use escouade_model::docs::{…}` dans `docs.rs`, de même dans les autres modules, et `use escouade_model::{heat, job, pricing, proc, wsl};` à la racine.

| Module de la crate | Ce qu'il contient | Réexporté par l'app à |
|---|---|---|
| `model` | `model.rs` : réglages, comptes, projets, tickets, agents, état sauvé, quotas, pauses, `Hold::{of, set, limit}`, `ConvOp` | `crate::model::*` |
| `paths` | tout `paths.rs` (dont `DataDir`, `worktrees_root`, les migrations) | `crate::paths::*` |
| `proc`, `job`, `wsl`, `heat` (et ses tests), `pricing` | les modules entiers | `crate::{proc, job, wsl, heat, pricing}` |
| `agentgroups` (ESC-417, venu de `main` à la fusion du 2026-11-13) | les groupes de processus des agents notés pour la prochaine ouverture : `job` les note, il suit donc `job` | `crate::agentgroups` |
| `tls` | `Net`, `Net::of`, `unchecked_by_proxy`, `proxy_url` (ESC-425) : `Settings::claude_env` les lit ; la configuration rustls reste dans le `tls` de l'app, qui les réexporte | `crate::tls::{Net, proxy_url}` |
| `fence` | la partie données : `Fence`, `Privacy`, `Side`, `Home`, `Ground`, `compute`, `claude_project_name` et leurs tests | `crate::fence::*` |
| `docs`, `docsplan`, `discussion`, `branchci`, `prreview` | les types qu'un ticket ou un agent garde (`Ref`, `Page`, `Gap`, `Decision`, `State`, `Change`, `slug`, `line_delta` ; `Plan`, `Made`, `Skipped` ; `Discussion`, `Kind`, `PageChange`, `Poured`, `Settles` ; `BranchCi` ; `PrReview`, `ReviewNote`, `ReviewPhase`, `NoteChoice`, `DiffLine`) et leurs méthodes | les mêmes modules |
| `registry` | `ToldModel`, le catalogue que `initialize` de Claude Code donne (`claude_catalog`) | `engine::registry::ToldModel` |
| `projectfile`, `workflowfix`, `notify`, `accounts`, `repos`, `attachfiles` | `ProjectShared`, `SharedFile`, `Pending`, `Warning` ; `Fix`, `FixStep`, `Fixes`, `Signal` ; `NotifyKind` ; `PRINCIPAL`, `ACCOUNTS_VERSION` ; `join` ; `same_file` | les mêmes modules |

Ce qui reste dans l'app, et pourquoi :

- `UiEvent` (il nomme `mcp`, `voice`, `devpage`, `deploys`, `workflow_track`, `docsshare`, `resources`…), `UsageSnapshot`, `AccountUsage` et `SignInProblem` (leurs méthodes sont dans `usage.rs`, qui lit `stats` et `accounts`) ; les tests qui les nomment (`model.rs` de l'app).
- `Hold::lift` devient le trait `board::LiftHold` : il lit l'usage des comptes et les seuils de `board`. Son appelant hors de `board` (`tickets.rs`) importe le trait : c'est le seul appelant qui change.
- `SpeechSettings::voice` devient le trait `model::SpeechVoice` : il lit les voix de `escouade-voice-local`, dont la crate ne dépend pas (sans quoi les crates suivantes tireraient whisper.cpp et ONNX Runtime). Son appelant importe `crate::model::*` : il ne change pas. Le modèle Whisper par défaut est une constante de la crate (`DEFAULT_WHISPER_MODEL`), qu'un test de l'app garde égale à `voice::models::DEFAULT_MODEL`.
- `Service::label` et son ordre lisaient le registre des sources, qui est fixe (les sept sources de cette version) : ils lisent la table `model::SOURCES`, que le test du registre (`integrations::source`) garde identique. `Registry::position`, qui ne servait qu'à cet ordre, est retiré.
- `fence` : ce qui lit le cœur, les comptes et les harness (`Core::project_fence`, les dossiers des sessions, les règles de chaque harness).

Ce qui a dû s'ouvrir : `branchci::BranchCi::shown`, `docs::State::{name, parse}`, `Hold::set`, `paths::{IDENTIFIER, folder_key_in, strip_base}`, `wsl::share_path`, `fence::{under, Side, Home, Ground, compute, claude_project_name}` (l'app les appelle d'un autre crate). Ce qui ne sert qu'aux tests (`NotifyFor::all_but`, `NotifyKind::ALL`, `EngineId::BENCH`, `QuotaMeter::credits`, `proc::output_in_files`) passe par la feature `testing`, que l'app prend en dev-dependency.

Ce qui n'a pas bougé : `canon.rs` et les fichiers hachés, les versions des crates publiées, les deux verrous (`cargo metadata --locked` dans `src-tauri` et `plugins`). La fixture que lisent les tests de `heat` est lue de `../../tests/fixtures`.

Ce qui s'ouvre : `model` n'est plus dans le cycle de `core`. L'hôte des plugins (ligne 2) a déjà `Settings` et `Fence` hors de l'app ; `updates/net.rs` (ligne 6) ne tient plus à l'app que par l'aide de ses tests (§ 5).

### Les mesures (dos à dos, même charge)

Sur le poste, aux conditions de la CI depuis ESC-409 (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`), dépendances en cache, crates du workspace à froid (`cargo clean -p`), avant = `main` (b0738835), après = cette branche fusionnée. Deux passes, dans un ordre puis dans l'autre (avant puis après, après puis avant), car le poste a beaucoup varié ce jour-là (d'autres worktrees compilaient) : une même mesure de la branche a pris 113 s puis 181 s.

| Mesure | Avant, passe 1 / passe 2 | Après, passe 1 / passe 2 | Moyennes |
|---|---|---|---|
| `cargo build --tests` | 152 / 235 s | 113 / 181 s | 194 → 147 s (− 24 %) |
| « lib (test) » de `escouade` | 141,2 / 211,1 s | 106,8 / 171,7 s | 176 → 139 s (− 21 %) |
| lib de `escouade` | 62,3 / 90,1 s | 46,4 / 65,0 s | 76 → 56 s (− 27 %) |
| `escouade-model` (lib, tests) | — | 3,1 et 5,3 s / 3,4 et 6,2 s | en parallèle de l'app |
| même chose à 3 jobs (`-j3`) | 176 / 250 s | 122 / 193 s | 213 → 158 s (− 26 %) |
| « lib (test) » à 3 jobs | 163,4 / 242,3 s | 113,2 / 184,1 s | 203 → 149 s |
| `cargo nextest archive --profile ci` | 218 / 157 s (18 binaires) | 117 / 205 s (19 binaires) | 188 → 161 s : le sens s'inverse d'une passe à l'autre, du bruit |
| pic mémoire de « lib (test) », 3 jobs (`-Z time-passes`) | 8 225 / 8 204 Mo | 7 691 / 7 603 Mo | − 568 Mo (− 7 %) |
| pic mémoire de la lib, 3 jobs | 3 965 / 4 096 Mo | 3 974 / 4 003 Mo | = |

Ce qu'elles disent :

- **La mémoire baisse sans ambiguïté** : − 0,57 Go au pic de « lib (test) », dans les deux passes, à 0,1 Go près. C'est plus que la part des lignes (2,9 %) : `model.rs` et ce qui est sorti avec lui sont surtout des types à `#[derive(Serialize, Deserialize)]`, dont le code généré (et ses monomorphisations) pèse bien plus que leurs lignes.
- **Le temps baisse dans les deux ordres**, mais d'un quart, ce que les lignes n'expliquent pas et que le § 3 n'a pas vu à 3 jobs pour ESC-408. La même raison (le code que génère serde) en explique une part ; la charge du poste, qui a varié de ± 50 %, empêche d'en dire plus ici. Les runs de `main` sur la CI, plus réguliers sous Windows et Linux (± 15 %, § 8.2), le diront.
- **L'archive ne se lit pas ici** : elle prend aussi l'édition des liens de 19 binaires, très sensible à la charge du disque.
- Une recompilation après un changement de la crate recompile aussi l'app, comme au § 3.

## 10. `escouade-plugins` (ESC-411)

Ligne 2 du § 5. La crate `crates/escouade-plugins` (mêmes règles qu'au § 4 : `publish = false`, GPL-3.0-or-later, membre et `default-member`) prend l'hôte des plugins : le registre des plugins installés (l'ancien `plugins/mod.rs`, devenu son `lib.rs`), `manifest` (réexporté de `escouade-plugin-cli`, comme avant), `runtime`, `wasm`, `beside`, `socket`, `store`, `store_keys`, `files`, `procs`, `journal`, `seed`, `decide`, `bench`, la partie données de `builtin` et de `sort`, et `tls` (le client TLS que l'hôte, l'app et les mises à jour partagent). L'app la réexporte à ses chemins : `pub use escouade_plugins::*;` dans `plugins/mod.rs`, `pub use escouade_plugins::tls::*;` dans `tls.rs` ; ses modules à elle masquent ceux du même nom de la crate et réexportent leur partie (`pub use escouade_plugins::builtin::*;`…). Aucun appelant ne change.

| Reste dans l'app | Pourquoi |
|---|---|
| `plugins/harness.rs`, `shared.rs` | les harness en processus : `Engine`, `AgentRt`, `Core`, les `prepare` des harness du cœur |
| `plugins/events.rs` (`Dispatch`) | le puits des faits du bus (`facts`) ; ce qu'un manifeste dit de ses faits (webhooks, commandes, gabarits) et `Place` sont dans la crate |
| `plugins/view.rs` | les vues des terminaux passent par `harness::plugged` ; il s'inscrit auprès de la crate pour savoir qu'un plugin s'arrête (`stops::on_stop`) |
| `plugins/reader.rs` | il construit des `workflow::Workflow` |
| `plugins/accounts.rs`, `adapters.rs`, `hosted.rs` | les comptes, `Forge` et `TicketSource` : avec `escouade-forge` (ligne 3) ; les noms d'un secret et d'un budget et les codes d'erreur sont dans la crate |
| `plugins/sort.rs` (`Config`, `sort`, `of_project`, `remote_hosts`) | ils lisent `integrations::Accounts` et le registre des hébergeurs ; les tables sont dans la crate |
| `plugins/builtin.rs` (`served_forge*`) | une aide de test qui sert un hébergeur par son plugin |

Les liens coupés d'abord :

- `integrations::secrets::SecretStore` devient le trait `escouade_plugins::secrets::SecretStore` (le trousseau de l'app l'implémente) ; `MemoryStore` le suit sous la feature `testing`.
- `forge::{host_of, bare_host}` vont dans `escouade_plugins::hosts` ; `RateLimit` était déjà du protocole. `forge::remote::parse` et `Registry::standard` restent : seul le tri de l'app les lit.
- Le lanceur des commandes git (`git::{command, run, run_hooks, text, output, …}`, leurs délais, `is_read`) va dans `escouade-model::git` : le registre clone et lit les étiquettes avec, l'app le réexporte. `engine::registry::Version` était `manifest::Version`.
- `isola::cli` (et le faux CLI des tests) va dans `escouade_plugins::isola` : `proc/*` le lance.
- `pty::ShellInfo` et `facts::NAMES` vont dans la crate (`events::ShellInfo`, `facts::NAMES`).
- `harness::{harnesses_of, described_lines}` vont dans `escouade_plugins::described` ; la liste des plugins des harness du cœur (`harness::BUILTIN_HARNESSES`, qui porte des fonctions du cœur) a ses ids dans `builtin::HARNESSES`, qu'un test de l'app garde égaux.
- Deux appels de l'hôte vers l'app deviennent des inscriptions : `view::forget` à l'arrêt d'un plugin (`stops::on_stop`, que `view` fait quand il crée le fil des vues d'un plugin ; un plugin sans vue n'avait rien à oublier) ; les ports des processus partagés que `local:shared` atteint (`shared_ports::note`, fait par `shared` quand il garde un processus, retiré quand il le lâche).
- `tauri::async_runtime::spawn` (le démarrage d'un plugin) reste : la crate prend `tauri` en dépendance, avec les mêmes fonctionnalités que l'app (déjà compilé, rien ne se recompile), pour démarrer ses plugins sur le même runtime qu'avant.
- `env!("CARGO_PKG_VERSION")` aurait donné la version de la crate : son script de build lit celle de l'app dans `src-tauri/Cargo.toml` (`ESCOUADE_APP_VERSION`, `escouade_plugins::APP_VERSION`) ; l'agent des requêtes (`escouade/<version>`), le banc et les manifestes du workspace disent toujours celle de l'app.

wasmtime et wasmtime-wasi passent dans la crate ; leurs `opt-level` du profil de développement restent dans `src-tauri/Cargo.toml`, un profil n'étant que celui du workspace. L'app les garde en dev-dependency (avec `flate2` et `tar`) pour les tests restés qui fabriquent des modules et des archives à la main.

Les tests suivent leur code : `plugins_runtime_tests`, `plugins_protocol_tests`, `plugins_wasm_tests`, `plugins_store_tests`, `plugins_seed_tests`, `plugins_reactor_tests`, `plugins_bench_tests`, `plugins_bench_wasm_tests`, `plugins_rust_build_tests`, `beside_tests` et les `mod tests` des modules sortis (≈ 6 800 lignes de fichiers de tests). Les six premiers prêtent leurs aides aux tests restés (`warm`, `fake_module`, `ready`, `holds`, le faux store, `fixture`…) : ils sont aussi compilés sous la feature `testing` (leurs `#[test]` en sont absents), et l'app les nomme à sa racine (`use escouade_plugins::plugins_runtime_tests;`), sans changer ses appelants. Leurs noms de modules ne changent pas : le filtre `plugins_rust_build_tests::` de `.config/nextest.toml` les prend toujours. Restent dans l'app ceux qui démarrent un `core` ou lisent les hébergeurs, les sources et leurs faux services (`integrations::fake`) : `plugins_tests`, `plugins_acp_tests`, `plugins_harness_*`, `plugins_decide_tests`, `plugins_accounts_tests`, `plugins_adapters_tests`, `plugins_net_tests`, `plugins_sort_tests`, `plugins_reader_tests`, les parités (`plugins_wasm_parity_tests` et ses sous-modules) et les tests des plugins de tickets ; ils suivront `escouade-forge` (ligne 3). D'où moins que les ≈ 18 000 lignes de tests attendues.

En tout, 22 100 lignes sortent de l'app (316 821 → 294 704 lignes de Rust dans `src-tauri/src`, − 7 %). Les tests sont les mêmes (4 628 → 4 629, le test des harness en plus) ; l'archive a un binaire de plus (20, dont celui de la crate).

Ce qui n'a pas bougé : `canon.rs` et les fichiers hachés (aucun fichier de `crates/escouade-plugin-cli`, des goldens, des fixtures ni de `plugins/`), les versions des crates publiées, les deux verrous (`cargo metadata --locked` dans `src-tauri` et `plugins`), `archive.include` (les modules des plugins sont dans l'archive). `escouade_base::testing::test_dir` retire à son tour du `PATH` les dossiers de whisper-rs-sys (`trim_test_path`), comme celui de l'app.

### Les mesures (dos à dos, même charge)

Sur le poste, aux conditions de la CI (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`), dépendances en cache, crates du workspace à froid (`cargo clean -p`), avant = `main` (976c5915), après = cette branche (41cbd313). Ordre des passes : avant, après, après, avant, avant, après ; un autre worktree mesurait aussi ce jour-là. La première passe « après » est écartée : elle suivait une mesure de mémoire, dont `RUSTC_BOOTSTRAP=1` fait recompiler les dépendances à la compilation suivante (60 s de cranelift, windows, wasmparser…) ; chaque passe de temps est désormais précédée d'une compilation de chauffe.

| Mesure | Avant (3 passes) | Après (2 passes) | Moyennes |
|---|---|---|---|
| `cargo build --tests` | 121 / 127 / 123 s | 121 / 119 s | 124 → 120 s (− 3 %) |
| « lib (test) » de `escouade` | 114,5 / 120,5 / 116,1 s | 109,3 / 106,8 s | 117,0 → 108,1 s (− 8 %) |
| lib de `escouade` | 52,6 / 52,8 / 50,7 s | 49,2 / 46,1 s | 52,0 → 47,7 s (− 8 %) |
| début de la compilation de l'app | 6,0 à 6,8 s | 12,0 et 12,2 s | + 6 s : elle attend les métadonnées de `escouade-plugins` (lib 5,2 s, ses tests 9,0 s, en parallèle) |
| même chose à 3 jobs (`-j3`) | 126 / 142 / 125 s | 126 / 132 s | 131 → 129 s (=) |
| « lib (test) » à 3 jobs | 118,8 / 133,0 / 117,4 s | 112,6 / 117,3 s | 123,1 → 115,0 s (− 7 %) |
| `cargo nextest archive --profile ci` | 117 / 127 / 119 s (19 binaires, 233,0 Mo) | 146 / 142 s (20 binaires, 245,3 Mo) | 121 → 144 s (+ 19 %) |
| pic mémoire de « lib (test) », 3 jobs (`-Z time-passes`) | 7 806 / 7 822 Mo | 6 974 / 7 233 Mo | − 710 Mo (− 9 %) |
| pic mémoire de la lib, 3 jobs | 3 810 / 3 786 Mo | 3 407 / 3 367 Mo | − 411 Mo (− 11 %) |
| pic mémoire de « lib (test) » de `escouade-plugins` | — | 920 / 889 Mo | à côté de l'app |

Ce qu'elles disent :

- **La mémoire baisse nettement** : − 0,7 Go au pic de « lib (test) » (7,8 → 7,1 Go), − 0,4 Go pour la lib. Avec ESC-410, « lib (test) » est passée de 8,2 à 7,1 Go à 3 jobs ; elle reste au-dessus des 7 Go d'un runner macOS arm64, de peu, et la lib et « lib (test) » ensemble bien au-delà.
- **Le temps de l'app baisse de 8 %**, à la mesure des lignes sorties (7 %) ; le build entier ne gagne que 3 %, car l'app commence 6 s plus tard : elle attend la crate, qui attend `escouade-model`. À 3 jobs le travail total ne baisse pas (le § 2 le prévoyait).
- **L'archive est plus lente ici de 20 s** dans les deux passes « après » (123 s dans la passe écartée) : un binaire de test de plus à lier, qui embarque wasmtime, reqwest et tauri (+ 12 Mo ; la crate, elle, n'est compilée qu'une fois, avec la feature `testing`). Le poste variant d'une passe à l'autre, c'est à confirmer sur la CI ; si elle le confirme, la piste est de lier moins (les tests de la crate sans tauri, son démarrage passé par l'app).
- Une recompilation après un changement de la crate recompile aussi l'app, comme au § 3 ; un changement dans l'app ne recompile plus l'hôte.

## 11. `escouade-engines` (ESC-412)

Ligne 5 du § 5. Les harness et la lecture de leurs flux ne se tiennent pas sans l'app (ci-dessous) : la crate `crates/escouade-engines` (mêmes règles qu'au § 4) n'en prend que les parties sans lien, comme le ticket le prévoyait dans ce cas. L'app réexporte tout à ses chemins.

| Module de la crate | Ce qu'il contient | Réexporté par l'app à |
|---|---|---|
| `program` | `Program`, `Located`, `Launch` : le programme d'un harness, du système ou de WSL | `engine::{Program, Located, Launch}` |
| `locate::{agy, claude, codex, copilot, cursor, gemini, opencode, vibe}` | où est le programme de chaque harness (le réglage, le PATH, les dossiers de ses installeurs) et leurs tests | `engine::<harness>::locate` (`use escouade_engines::locate::codex as locate;` à la place de `mod locate;`) |
| `which` | le programme d'un nom sur le PATH | `crate::which` |
| `userconfig`, `jsonc` | les configs de l'utilisateur qu'Escouade écrit lui-même (TOML, JSON avec commentaires) | `engine::userconfig` (ses tests, qui lisent les `USER_CONFIG` des harness, restent dans l'app) |
| `mcp` | `Escouade` (le serveur donné ou refusé), `SERVER`, `tool_name` ; `Entry`, `Standing`, `MASK`, `mask_bearer`, venus de `mcp::install` | `engine::mcp`, `mcp::install::{Entry, Standing, MASK, mask_bearer}` |
| `signin` | la connexion de Claude Code (son fichier, le trousseau de macOS, l'empreinte) | `engine::claude::signin` |
| `sidechain` | le transcript d'un sous-agent : le trouver, le lire, comment il a fini ; `INTERRUPTED` (venu de `agent/transcript.rs`) | `agent::sidechain` |
| `shape` | ce qu'un item d'outil garde : `tool_result`, `item_input`, `trim_strings`, les limites (`MAX_TEXT`…), venus de `agent.rs` | importés par `agent.rs` |

Ce qui a dû bouger ou s'ouvrir :

- `impl AgentRt { end_subagent, subagent_model }`, qui était dans `agent/sidechain.rs`, reste dans l'app (`agent/subagents.rs`), avec le test qui compare un transcript au flux (il démarre un `AgentRt`).
- `CONFIG_DIR_VAR` va dans `escouade_model::accounts` (le programme le lit), `text::truncate` dans `escouade_base::text` (le transcript d'un sous-agent coupe ses textes) ; l'app les réexporte.
- Les `cfg!(test)` des `locate` (ne pas chercher le programme de la machine dans les tests, ni `ESCOUADE_<HARNESS>`), de `signin` (`KeychainGate`, `keychain_answer` compilés pour les tests) et les aides des tests (`Found::len`, `mcp::tool_name`) passent par la feature `testing`, que l'app prend en dev-dependency : ses tests ont le même comportement qu'avant.
- `tr!` est celui de l'app : la crate a le sien, sur `escouade_base::i18n::ui()`.
- `locate::*::find` et les autres `pub(super)` des fichiers sortis deviennent `pub`.

Ce qui n'a pas bougé : `canon.rs` et les fichiers hachés, les versions des crates publiées, les deux verrous (`cargo metadata --locked` dans `src-tauri` et `plugins`), les fixtures (`tests/fixtures/real-sidechain.*`, lues à la même profondeur).

### Pourquoi le reste ne sort pas

- **Le trait `Engine` prend l'état d'un agent.** `engine/mod.rs`, qui reste à l'app, définit `Engine` ; chaque harness l'implémente. Ses méthodes nomment `AgentRt` et `Effects` (`translate`, les vues terminal `TerminalView` avec `HookOutcome` et `tui::Registered`), `core::Attachment`, `mcp::install::Target`, `job::JobUsage`. Un harness dans une crate doit avoir le trait dans la même crate ou au-dessous : il faut sortir le trait, donc ce qu'il nomme.
- **La lecture des flux est l'état d'un agent.** Les fichiers de `agent/` (sauf `sidechain`) sont des blocs `impl AgentRt` sur `use super::*` : ils écrivent dans l'agent (ses items, son plan, ses demandes en attente). `agent.rs` (≈ 6 200 lignes de code, ≈ 4 000 de tests) tient `board::TurnEnd`, `conv::Conv`, `core::naming_text`, `facts`, `launchenv`, `notify`, `pty`, `sentfiles`, `tui`, `voice::spoken`, `plugins::decide`.
- **Les harness lancent leur programme par l'hôte des plugins** (`plugins::harness`, `plugins::builtin`, 68 citations) et lisent les comptes (`accounts::config_dir`, les connexions, `by_key`…), `usage`, `workflow`, `fence::claude_settings`, `memory::claude_dir`, `board::protocol_prompt`. Leurs bancs (`engine/*/bench.rs`) démarrent le runtime des plugins de test (`plugins_acp_tests`) et le faux `claude` de `core_tests`.

L'ordre qui le permettrait, un ticket chacun : (1) l'état d'un agent (`AgentRt`, `Effects`, `conv`) dans une crate, ce qu'il demande au cœur passé par un trait de l'hôte que l'app implémente (les mots d'un nom, la fin d'un tour, les sons, le terminal) ; (2) le trait `Engine` avec lui, `Attachment` et `Target` en données ; (3) les harness, les comptes et le lancement par les plugins passés par un trait, sur `escouade-plugins` (ESC-411, § 10), qui a déjà sorti l'hôte. Environ 21 000 lignes de code et 11 500 de tests restent ainsi dans l'app.

### Les mesures (dos à dos, même charge)

Sur le poste, aux conditions de la CI (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`), dépendances en cache, crates du workspace à froid (`cargo clean -p`), avant = `main` (976c5915), après = cette branche, avant qu'elle reprenne ESC-411 (les mesures ne sont pas refaites après). Deux passes par cas, en alternance (après, avant, après, avant). Le pic mémoire est le plus grand `rss` de `-Z time-passes` (`cargo rustc -j3 --lib --profile test -p escouade`), mesuré après tous les builds : `RUSTC_BOOTSTRAP` change l'empreinte des dépendances, et un build qui suit les recompile (une première série, où ils alternaient, a perdu ses temps à tous les cœurs ; ses temps à 3 jobs restent bons).

| Mesure | Avant, passe 1 / passe 2 | Après, passe 1 / passe 2 | Moyennes |
|---|---|---|---|
| `cargo build --tests` | 129 / 124 s | 124 / 134 s | 127 → 129 s (=) |
| « lib (test) » de `escouade` | 121,8 / 117,5 s | 115,3 / 126,7 s | 120 → 121 s (=) |
| lib de `escouade` | 51,6 / 49,8 s | 49,5 / 54,4 s | 51 → 52 s (=) |
| `escouade-engines` (lib, tests) | — | 0,9 et 1,4 s / 0,9 et 1,9 s | en parallèle de l'app |
| même chose à 3 jobs (`-j3`) | 128 / 127 s | 121 / 127 s | 128 → 124 s (− 3 %) |
| « lib (test) » à 3 jobs | 120,0 / 117,7 s | 113,2 / 118,0 s | 119 → 116 s (− 3 %) |
| pic mémoire de « lib (test) », 3 jobs | 7 724 / 7 758 Mo | 7 820 / 7 791 Mo | 7 741 → 7 806 Mo (+ 65 Mo) |

Ce qu'elles disent : rien de mesurable. 1,3 % des lignes sortent, sous le bruit du poste (± 5 % d'une passe à l'autre ce jour-là). Les 3 % gagnés à 3 jobs sont de l'ordre de l'écart entre deux passes d'un même cas. Le pic mémoire monte de 65 Mo dans les deux passes, à peine plus que l'écart entre elles (35 Mo) : ce qui sort ici est surtout du code de fonctions (localiser, lire un fichier), pas de types à `derive` comme au § 9, et l'app lit les métadonnées d'une crate de plus. Le gain que la ligne 5 promettait (≈ 8 %) viendra du reste, une fois l'état d'un agent sorti.

## 12. `escouade-forge` et `escouade-updates` (ESC-414)

Lignes 3 et 6 du § 5. Deux crates (mêmes règles qu'au § 4 : `publish = false`, GPL-3.0-or-later, membres et `default-members`). L'app réexporte tout à ses chemins : `pub use escouade_forge::forge::*;` dans `forge/mod.rs`, `pub use escouade_forge::integrations::*;` dans `integrations/mod.rs`, `pub use escouade_forge::plugins::{accounts, adapters, builtin, hosted};` dans `plugins/mod.rs`, `use escouade_updates::net;` dans `updates.rs`. Aucun appelant ne change.

| Crate | Ce qu'elle contient | Reste dans l'app | Lignes (dont fichiers de tests) |
|---|---|---|---|
| `escouade-forge` | `forge` : le contrat des hébergeurs, leur registre, `accounts`, `remote`, les clients de GitHub, GitLab, Bitbucket, Gitea et Azure DevOps et leurs tests ; `integrations` : les comptes des sources, `secrets` (le trousseau), `source` (le registre des sources), `jira`, `fake` (les faux services des tests, feature `testing`), les états par défaut (`text::default_states`) et les tests des sources ; `plugins` : `accounts`, `adapters`, `hosted`, `builtin::served_forge*` ; les tests des plugins de tickets qui ne démarrent pas de `core` (`plugins_adapters_tests`, `plugins_jira_tests`, `plugins_linear_tests`, `plugins_trello_tests`, `plugins_clickup_tests`) | `forge/connect.rs` (les formulaires et la santé des hébergeurs), `forge/tests.rs` (il démarre un `core`), `integrations/sync.rs`, `integrations/text.rs` (les textes que la synchro écrit), `plugins/sort.rs`, et les tests qui démarrent un `core` ou lisent l'hôte en processus (`integrations_tests`, `plugins_accounts_tests`, `plugins_net_tests`, `plugins_sort_tests`, `plugins_loans_tests`, les parités) | 14 893 (9 100) |
| `escouade-updates` | `net` (ce que l'app apprend d'une requête de l'updater qui a échoué : la sonde, les mots), `fake_server` (feature `testing`), `net_tests` | `updates.rs` (l'updater) et `flow_tests.rs`, qui teste son `Checker` sur l'app de test de Tauri | 2 411 (1 404) |

Ce qui a dû bouger ou s'ouvrir :

- Les `cfg(test)` qui décident sous test passent par la feature `testing`, que l'app prend en dev-dependency : surtout `integrations::secrets` (la mémoire au lieu du trousseau, `file_keeps_copy`), puis `fake`, `memory_of`, `served_forge`, `Registry::with`, `forge::Pace`. Les tests de l'app gardent ainsi le même comportement ; sans cela, ils auraient lu le vrai trousseau.
- `core::slugify` et `board::encode` (fonctions pures) vont dans `escouade_base::text` (`slugify`, `slug_within`, `percent_encode`) ; l'app les réexporte. Les états par défaut d'un ticket externe (`default_states` et ses motifs), que les adaptateurs lisent, vont dans `escouade_forge::integrations::text` avec leur test.
- `keyring` (Windows et macOS) passe de l'app à la crate, aux mêmes versions et fonctionnalités.
- `env!("CARGO_PKG_VERSION")` de l'agent de la sonde aurait dit la version de la crate : il lit `escouade_plugins::APP_VERSION`, celle de l'app.
- `connect.rs` et `sync.rs` prenaient par `use super::*` les `use` privés de l'ancien `mod.rs` : le nouveau les a. `Account` est dans les deux globs de `integrations/mod.rs` (`model::*` et la crate) : un `pub use` explicite garde celui des sources, comme quand il était défini là.
- `plugins_trello_tests` et `plugins_clickup_tests` prêtent leurs faux services aux parités de l'app : compilés aussi sous `testing`, nommés à la racine de l'app. `fake_server` de même pour `flow_tests`.
- Le test de `connect::expires_soon` (dans `azure_tests`) reste dans l'app, à `forge/tests.rs`.
- `pub(crate)` et `pub(super)` des fichiers sortis deviennent `pub`.

En tout, 17 200 lignes sortent de l'app (290 638 → 273 432 lignes de Rust dans `src-tauri/src`, − 5,9 %). Les tests sont les mêmes (4 629 attributs de test dans le workspace avant et après ; 301 passent de l'app aux deux crates ; nextest : 4 546 passés, 55 ignorés) ; l'archive a deux binaires de plus (23).

Ce qui n'a pas bougé : `canon.rs` et les fichiers hachés (aucun fichier de `crates/escouade-plugin-cli`, des goldens, des fixtures ni de `plugins/`), les versions des crates publiées, les deux verrous (`cargo metadata --locked` dans `src-tauri` et `plugins` ; le verrou de `src-tauri` n'a que les deux crates en plus, et `keyring` qui change de crate).

### Les mesures (dos à dos, même charge)

Sur le poste, aux conditions de la CI (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`), dépendances en cache, crates du workspace à froid (`cargo clean -p`), avant = `main` (abfe2979), après = cette branche (1b9750cb). Passes dans l'ordre avant, après, après, avant, une compilation de chauffe après chaque changement de commit ; les pics mémoire (`-Z time-passes`, le plus grand `rss`) après tous les builds, dans le même ordre. La seconde passe « avant » a été plus chargée que les autres (à 3 jobs surtout : 179 s contre 140) : les moyennes à 3 jobs exagèrent le gain.

| Mesure | Avant, passe 1 / passe 2 | Après, passe 1 / passe 2 | Moyennes |
|---|---|---|---|
| `cargo build --tests` | 130 / 145 s | 123 / 115 s | 138 → 119 s (− 13 %) |
| « lib (test) » de `escouade` | 114,0 / 133,5 s | 111,9 / 102,9 s | 123,8 → 107,4 s (− 13 %) |
| lib de `escouade` | 52,2 / 54,8 s | 48,9 / 45,1 s | 53,5 → 47,0 s (− 12 %) |
| début de la compilation de l'app | 14,1 / 9,8 s | 10,4 / 11,2 s | = : elle n'attend pas les nouvelles crates (`escouade-forge` 1,5 à 1,8 s, ses tests 6,2 à 6,5 s ; `escouade-updates` 0,6 s, ses tests 2,4 à 2,7 s, en parallèle) |
| même chose à 3 jobs (`-j3`) | 140 / 179 s | 131 / 129 s | 160 → 130 s (− 18 %, voir plus haut) |
| « lib (test) » à 3 jobs | 126,2 / 159,2 s | 116,9 / 107,9 s | 142,7 → 112,4 s |
| `cargo nextest archive --profile ci` | 132 / 117 s (21 binaires, 245,9 Mo) | 123 / 120 s (23 binaires, 260,5 Mo) | 125 → 122 s (=) |
| pic mémoire de « lib (test) », 3 jobs | 7 158 / 7 175 Mo | 6 813 / 6 809 Mo | − 356 Mo (− 5 %) |
| pic mémoire de la lib, 3 jobs | 3 417 / 3 381 Mo | 3 366 / 3 309 Mo | − 61 Mo (− 2 %) |

Ce qu'elles disent :

- **Le temps de l'app baisse de 12 à 13 %**, plus que la part des lignes (5,9 %), dans les deux passes ; les deux passes « après » sont plus rapides que les deux « avant », l'écart entre elles (≈ 10 s) restant sous le gain (≈ 16 s). Une part vient sans doute, comme au § 9, des types à `derive` sortis (comptes, états, requêtes des hébergeurs) ; la charge du poste empêche d'en dire plus.
- **La mémoire baisse sans ambiguïté** : − 0,36 Go au pic de « lib (test) », à 20 Mo près dans les deux passes. Avec ESC-410 et ESC-411, « lib (test) » est passée de 8,2 à 6,8 Go à 3 jobs : **sous les 7 Go** d'un runner macOS arm64 pour la première fois, seule ; avec la lib à côté (3,3 Go), toujours au-delà, d'où le levier 2 (§ 8), qui la compile avant.
- **L'archive ne gagne rien ici** : deux binaires de tests de plus à lier (+ 14,6 Mo), qui reprennent ce que l'app gagne. Comme au § 10, la CI dira si le Mac, qui pagine, y gagne davantage.
- Une recompilation après un changement des crates recompile aussi l'app, comme au § 3 ; un changement de l'app ne recompile plus les hébergeurs.

## 13. `escouade-mcp` (ESC-413)

Ligne 4 du § 5. Le serveur MCP ne sort pas en entier sans réécrire ses tests (ci-dessous) : la crate `crates/escouade-mcp` (mêmes règles qu'au § 4 : `publish = false`, GPL-3.0-or-later, membre et `default-member`) prend ce qui s'en tient sans l'app, comme le ticket le prévoyait dans ce cas : le transport HTTP, les jetons et le port, le journal, les arguments des outils et leurs schémas, la partie de l'installation chez les harness qui ne lit pas le registre des harness. L'app réexporte tout à ses chemins.

| Module de la crate | Ce qu'il contient | Réexporté par l'app à |
|---|---|---|
| racine | `Caller`, `McpStatus`, `PORTS`, `PATH`, `Tokens` (la comparaison en temps constant), `new_token` | `mcp::*` |
| `port` | `bind`, `bind_within`, `worth_waiting_for`, `wait_closed`, `STOP_WAIT` et leurs tests | importés par `mcp/mod.rs` |
| `http` | les gardes (`Host`, `Origin`, jeton, chemin, outil inconnu, taille), `Limits`, `Reply`, `serve` ; `Gate` est générique sur un trait `Host` (qui est un jeton, une requête refusée notée, ce que reçoit un outil) et sur le gestionnaire de rmcp | `mcp::http::*` ; `impl Host for Core<R>` reste dans `mcp/http.rs` |
| `activity` | `Activity`, `ActivityEntry`, `Outcome`, `KEPT`, `clip`, `entry` (une entrée du journal, le nom de l'appelant donné) et son test | `mcp::activity::*` ; `record` et `caller_name` (ils lisent le cœur et ses agents) restent |
| `args` | les arguments des 59 outils et ce que le modèle en lit (`#[schemars(description)]`), `SPLIT_MAX`, `ColumnArg` → `Column` | `mcp::tools::*` |
| `memory` | `Kind`, ce dont parle un souvenir (l'argument `type` de `write_memory`) | `memory::Kind` |
| `install` | `Target`, `Declared`, `Declaration`, `wanted`, `run`, `mask_secrets`, `COMMAND_TIMEOUT` (et `Entry`, `Standing`, `MASK`, `mask_bearer` d'`escouade-engines`) | `mcp::install::*` ; `current`, `declare`, `withdraw` (ils prennent `&dyn Engine`) et la déclaration compte par compte (`impl Core`) restent |

Les outils, leurs schémas et leurs textes sont les mêmes octet pour octet : la liste que donne le routeur (`Tools::router().list_all()`, noms, descriptions, schémas d'entrée, avec l'outil des tests : 60 outils) a été écrite en JSON avant et après, 131 788 octets identiques (`cmp`). Les descriptions des outils et les instructions du serveur sont des attributs du bloc `#[tool_router]`, qui reste à l'app ; les textes des refus et des erreurs (`tr!`) ont été déplacés tels quels.

Ce qui a dû bouger ou s'ouvrir :

- `Gate::new` prend le routeur et de quoi faire le gestionnaire de chaque requête (`Tools::<R>::router()`, `Tools::<R>::new`) : c'est le seul appelant qui change (`McpServer::launch`).
- `serve` lance chaque connexion par `tokio::spawn` au lieu de `tauri::async_runtime::spawn` : il tourne déjà sur le runtime de l'app (`McpServer::launch` le lance par `tauri::async_runtime::spawn`), où `tokio::spawn` les met aussi ; la crate n'a pas besoin de `tauri`.
- `STOP_WAIT` (30 s dans les tests, 2 s sinon) passe par la feature `testing`, que l'app prend en dev-dependency.
- Les champs de `Tokens`, `ColumnArg::name`, `Kind::{as_str, parse}`, `install::run` et `mask_secrets` deviennent `pub` ; `port::held_port` (une aide des tests) est sous `testing`.
- `subtle` sort des dépendances de l'app (seule la crate compare les jetons).

Ce qui n'a pas bougé : `canon.rs` et les fichiers hachés, les versions des crates publiées, les deux verrous (`cargo metadata --locked` dans `src-tauri` et `plugins` ; celui de `plugins` ne change pas), les tests restés (aucun n'a été réécrit : seuls leurs `use` nomment la crate).

En tout, 1 832 lignes sortent de l'app (290 638 → 288 806 lignes de Rust dans `src-tauri/src`, − 0,6 %, comptées et mesurées sur abfe2979, avant ESC-414), dont 5 tests ; la crate en a 1 986.

### Pourquoi le reste ne sort pas

- **Les outils agissent sur le cœur, pas sur une interface.** Les 11 fichiers des outils (`act`, `read`, `wait`, `files`, `images`, `docs`, `memory`, `prs`, `workflow`, `background`, `resolve` : ≈ 11 000 lignes) prennent `&Core<R>` et en lisent 79 méthodes et champs différents (`core.data`, `core.settings`, `core.agents` et le verrou de chaque `AgentRt`, `core.tickets`, `core.hub`…), et appellent 30 modules de l'app (`tickets`, `board`, `git`, `folders`, `sharing`, `agent`, `engine`, `forge`, `prview`, `docs`, `discussion`, `ticketfiles`, `workflow`, `facts`, `vectors`, `usage`…). Un trait de l'hôte devrait porter tout cela : ce n'est plus un trait, c'est l'app.
- **Les 19 fichiers de tests démarrent un vrai cœur** (`core_tests::harness`, 22 100 lignes) et lisent ses internes (`h.core.settings.write()`, les tickets, les agents, `h.core.mcp`). Une crate de soutien des tests qui démarrerait un cœur hors de l'app devrait contenir `Core`, donc l'app. Un faux cœur derrière un trait obligerait à réécrire ces tests, qui vérifient justement que les outils font ce que font les boutons de la fenêtre, par les mêmes fonctions du cœur : on y perdrait ce qu'ils prouvent.

L'ordre qui le permettrait, un ticket chacun : (1) un trait de l'hôte par famille d'outils (lire les projets et tickets ; agir sur le tableau ; les agents et leurs demandes ; les documents, la spec, la mémoire), dont les arguments et les réponses sont des données de `escouade-model` ; (2) les outils sur ces traits dans la crate, avec des tests unitaires sur un faux hôte pour ce qui est du protocole (refus, résolution des noms, textes), les 19 fichiers de tests actuels restant dans l'app comme tests d'intégration sur le vrai cœur. Le gain en compilation ne viendrait que de (2), et les tests restés gardent ≈ 22 000 lignes dans « lib (test) ».

### Les mesures (dos à dos, même charge)

Sur le poste, aux conditions de la CI (`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`), dépendances en cache, une compilation de chauffe avant chaque passe, crates du workspace à froid (`cargo clean -p`), avant = `main` (abfe2979), après = cette branche (7a1034e1). Ordre des passes : après, avant, avant, après (puis à 3 jobs : après, avant) ; tous les builds d'abord, les mesures de mémoire ensuite (même ordre), l'archive enfin (même ordre).

| Mesure | Avant, passe 1 / passe 2 | Après, passe 1 / passe 2 | Moyennes |
|---|---|---|---|
| `cargo build --tests` | 129 / 128 s | 127 / 123 s | 128,5 → 125 s (− 3 %) |
| « lib (test) » de `escouade` | 117,5 / 116,5 s | 116,6 / 112,7 s | 117,0 → 114,7 s (− 2 %) |
| lib de `escouade` | 55,3 / 52,4 s | 51,8 / 49,3 s | 53,9 → 50,6 s (− 6 %) |
| début de la compilation de l'app | 9,5 et 9,9 s | 9,4 et 8,9 s | = (`escouade-mcp`, lib 1,2 s et tests 1,6 s, finit avant `escouade-plugins`, qu'elle n'attend pas) |
| même chose à 3 jobs (`-j3`) | 131 s | 132 s | = |
| « lib (test) » à 3 jobs | 116,2 s | 117,2 s | = |
| `cargo nextest archive --profile ci` | 115 / 114 s (21 binaires, 245,96 Mo) | 118 / 129 s (22 binaires, 246,22 Mo) | 114,5 → 123,5 s (+ 8 %) |
| pic mémoire de « lib (test) », 3 jobs (`-Z time-passes`) | 7 186 / 7 166 Mo | 7 029 / 7 035 Mo | 7 176 → 7 032 Mo (− 144 Mo, − 2 %) |

Ce qu'elles disent :

- **Le gain est à la mesure des lignes sorties, et petit** : − 2 % sur « lib (test) », − 6 % sur la lib, − 3 % sur le build entier, dans le bruit d'une passe à l'autre (± 3 % ce jour-là) mais dans le même sens aux deux passes. Rien à 3 jobs, où le travail total ne baisse pas (§ 2).
- **La mémoire baisse un peu, sans ambiguïté** : − 144 Mo au pic, à 20 Mo près entre les passes. Plus que la part des lignes (0,6 %) : les arguments des outils sont des types à `derive(Deserialize, JsonSchema)` (§ 9), dont le code généré pèse plus que leurs lignes. « lib (test) » reste au niveau des 7 Go d'un runner macOS arm64, et la lib en même temps bien au-delà.
- **L'archive est plus lente de 9 s** : un binaire de test de plus à lier (celui de la crate, qui embarque rmcp, hyper et tokio ; 5 tests), comme au § 10 ; la seconde passe « après » (129 s) s'écarte de la première de 11 s, à confirmer sur la CI. Si elle le confirme, la piste est `test = false` sur la lib de la crate, ses 5 tests revenant à l'app.
- Une recompilation après un changement de la crate recompile aussi l'app, comme au § 3 ; un changement dans l'app ne recompile pas la crate.
