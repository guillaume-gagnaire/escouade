# Avis de sécurité sur les dépendances

`cargo deny check` (`deny.toml` à la racine, pour `src-tauri` et `plugins`) échoue sur tout avis RustSec nouveau. Un avis
se ferme en mettant la dépendance à jour ; quand aucune mise à jour compatible ne le ferme, il est ignoré dans
`deny.toml` avec sa raison, et l'ignore part avec la dépendance. Cette page garde ce qui ne tient pas en une ligne de
`deny.toml`.

## zip 4.6.1, par tauri-plugin-updater (ESC-517, 2026-11-17)

Avis : RUSTSEC-2026-0367, 0368, 0369, 0429 (`ZipArchive::extract`), 0545 (entrées AES), 0548 (`ZipStreamReader`). Tous
corrigés en zip ≥ 9 (≥ 9.0.5 pour 0548).

Le zip de l'app est le 9 (`src-tauri/Cargo.toml`, lu par `docsextract.rs`). Le zip 4 ne vient que de
`tauri-plugin-updater` 2.13.1, la dernière 2.x (la suivante est une 3.0 alpha), qui le demande en `zip = "4"`, sous
Windows seulement, sans les fonctions par défaut. Lu dans le code de la version verrouillée :

- **Signature d'abord.** `Update::download` (`src/updater.rs`) lit tout le corps puis appelle `verify_signature`
  (l. 837) : minisign avec la clé publique de `tauri.conf.json`, rendu en erreur avant de rendre les octets.
  Notre code (`src-tauri/src/updates.rs`, `PluginRelease::fetch` et `refused`) ne passe à `install` que les octets
  rendus par `download` ; les commandes JS du plugin ne sont pas permises (`capabilities/`).
- **Zip seulement s'il est signé et est un zip.** Sous Windows, `extract` (l. 1050) n'appelle `extract_zip` que si les
  octets sont un zip (`infer::archive::is_zip`) ; `extract_zip` (l. 1070-1075) fait `ZipArchive::new` puis
  `ZipArchive::extract`. Notre mise à jour Windows est le setup NSIS lui-même (`createUpdaterArtifacts: true`, pas
  `v1Compatible` ; la release publie `Escouade_X.Y.Z_x64-setup.exe` et son `.sig`, aucun `.zip`) : ce chemin n'est
  pas pris. Il ne le serait que pour un zip signé avec notre clé.
- **macOS et Linux** n'ont pas de zip : le `.app.tar.gz` et l'AppImage `.tar.gz` passent par `flate2` et `tar`
  (l. 1179 et suivantes), après la même signature.
- **0545 (AES)** : le zip 4 est compilé sans `aes-crypto` (ses dépendances dans `Cargo.lock` : `arbitrary`,
  `crc32fast`, `indexmap`, `memchr`) ; aucune entrée chiffrée ne peut être lue.
- **0548 (`ZipStreamReader`)** : le plugin ne l'utilise nulle part (seul `zip::ZipArchive`, l. 1074, et
  `zip::result::ZipError` dans `src/error.rs`).

Conclusion : chemin non atteint, ignores motivés. À retirer quand `tauri-plugin-updater` passe à zip ≥ 9.0.5
(`cargo tree -i zip@4.6.1` vide).

## chrono (RUSTSEC-2026-0543, ESC-517, 2026-11-17)

**Ce que dit l'avis** : `informational = "unmaintained"`, toutes versions, `patched = []`, alternatives conseillées
jiff et time. Aucune faille citée. L'avis est ignoré avec cette raison.

**Où il est** (au 2026-11-17, hors commentaires, tests compris sauf mention) :

| Où | Usages | Ce qu'on en fait |
| --- | --- | --- |
| `src-tauri/src` | 67 lignes dans 31 fichiers (44 lignes dans 21 fichiers hors tests) | `Utc::now`, `Local` (heures locales, `stats.rs` : `NaiveDate`, `Months`, `Datelike`), RFC 3339, `format("%Y-%m-%d…")`, `serde` |
| `escouade-model` | 6 lignes, 3 fichiers | sauvegardes (`backups.rs` : horodatage, `TimeZone`, `Timelike`), heure locale (`heat.rs`), `timestamp_millis` |
| `escouade-plugins` | 5 lignes, 5 fichiers | `Utc::now().to_rfc3339…` |
| `escouade-engines` | 1 ligne | `parse_from_rfc3339` |
| `escouade-plugin-kit` (publié, 0.1.2) | 1 ligne (`forge::instant`) | `parse_from_rfc3339` → `u64` : chrono n'est pas dans son API publique |
| `escouade-plugin-protocol` (publié, 0.1.3) | aucun | — |
| `plugins/copilot` | 8 lignes | `timestamp_millis`, RFC 3339, `Datelike` |
| tiers | `rmcp` 3.9 (dépendance non optionnelle, `chrono04` de `schemars` 1.2) | — |

**Coût d'un remplacement** : environ 90 lignes dans 45 fichiers, une quinzaine d'API (dont les calendaires de
`stats.rs` et les formats `strftime`, qui ont leur équivalent dans jiff, et en partie dans time). Ce qui demande le plus
de soin : l'heure locale (`Local`, fuseau du système) et les formats persistés (`backups.rs`, champs `serde`), à
garder identiques octet pour octet ; les deux crates publiés ne changent pas d'API (le kit ne rend qu'un `u64`), une
version de correctif suffit. Ordre de grandeur : un ticket d'agent, 1 à 2 h. **Mais chrono resterait dans
`Cargo.lock`** : `rmcp` (le serveur MCP) et `schemars` l'amènent sans option. L'avis resterait ignoré.

**Recommandation** : ne pas migrer maintenant. Le gain sécurité est nul tant qu'aucune faille n'est publiée et que
`rmcp` le garde ; garder l'ignore et y revenir quand `rmcp` le quitte ou qu'un avis de faille paraît. Si on migre un
jour, choisir **jiff** (fuseaux et calendrier complets, comme `Local` et `Months` le demandent ; time n'a pas de fuseau
local fiable sous Unix multi-thread) et commencer par le kit publié (une fonction) pour qu'un plugin tiers n'en hérite
pas.
