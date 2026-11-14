# Audit de sécurité d'Escouade 2.2.0, novembre 2026

L'état de la sécurité d'Escouade 2.2.0 (étiquette `v2.2.0`, commit `ab351a00`) au 14 novembre 2026 : le rapport, sa source, ses essais et leurs résultats bruts. C'est un audit interne, mené par un agent d'IA pour l'éditeur ; il dit l'état d'une version à une date, pas celui des versions suivantes.

| Fichier ou dossier | Ce que c'est |
| --- | --- |
| `escouade-audit-securite-2026-11.pdf` | Le rapport : résumé, périmètre, architecture, inventaires des flux réseau et des données, 18 constats, contrôles en place, annexes |
| `rapport.md` | Sa source. Tout ce qui est écrit dans le PDF vient d'ici et des fichiers de `annexes/` qu'il cite |
| `build.mjs` | Produit le PDF (Markdown → HTML → PDF par le Chromium de Playwright) ; `--check` cherche ce qui ne doit pas sortir |
| `annexes/` | Sorties brutes des outils sur la 2.2.0 (`cargo audit`, `npm audit`), résumés de `cargo deny`, `cargo geiger` et `semgrep`, dépendances directes |
| `annexes/essais/` | Résultats des essais : boîte blanche, boîte noire (une note JSON par scénario), plugin malveillant seul, tests du projet rejoués, contrôle des binaires publiés (signatures, attestation de provenance) |
| `poc/whitebox/audit_2_2_0.rs` | 38 tests Rust d'attaque, compilés avec les tests de l'app (`src-tauri/src/plugins/mod.rs` les nomme), tous ignorés par défaut |
| `poc/blackbox/` | 16 scénarios Playwright contre l'app lancée isolée (la fixture des tests de bout en bout) |
| `poc/malicious-plugin/` | Le plugin d'essai malveillant : il tente ce que ses permissions ne couvrent pas, puis un déni de service |
| `tools/annexes.mjs` | Fait `annexes/` à partir des sorties brutes des outils et des essais |
| `tools/references.mjs` | Contrôle chaque `fichier:ligne` du rapport au commit audité (`check`) ; pour auditer une autre version, le retrouve dans son code (`locate <commit>`, `move <commit>`) |
| `tools/verify-update-signature.mjs`, `tools/inspect-macos-package.mjs`, `tools/verify-provenance.mjs` | Les contrôles d'une version publiée, sans rien lancer : signature de mise à jour, paquet macOS, attestation de provenance de chaque binaire (`gh attestation verify`) et commit qu'elle nomme |

## Refaire le PDF

```
node docs/security/audit-2026-11/build.mjs
```

Le script lit d'abord `rapport.md`, ce fichier et les annexes à la recherche de ce qui ne doit pas sortir : une adresse e-mail, un dossier personnel, un dossier du poste qui a mené l'audit, un jeton, un webhook, une clé privée, le nom de l'utilisateur du poste, et les mots qui diraient pour qui ou pour quelle occasion un audit a été fait. S'il en trouve, il s'arrête sans rien écrire. Il garde aussi `build/rapport.html` (hors de git), pour regarder la mise en page dans un navigateur.

Les `fichier:ligne` du rapport citent le commit de la version auditée : `node docs/security/audit-2026-11/tools/references.mjs check` le contrôle.

## Rejouer les essais

Les commandes sont dans l'annexe F du rapport. À savoir :

- Les essais sont écrits pour le code de la 2.2.0. Les tests Rust sont ignorés par défaut : ni la CI ni un `cargo test` ordinaire ne les lancent. `holds_*` : une attaque repoussée. `limit_*` : une garde qui tient pour ce qu'elle nomme, et ce qu'elle laisse passer, que le test imprime. `finding_*` : une faiblesse du rapport, montrée telle qu'elle est (le test passe tant qu'elle est là). `fuzz_*` : des entrées générées, `AUDIT_FUZZ_SECONDS` secondes par cible.
- L'hôte des plugins est une crate à part (`crates/escouade-plugins`) : un test de l'app ne l'atteint que par ce qu'elle rend public. Un test qui a besoin d'une fonction privée de l'hôte passe par la porte publique qui l'appelle (le contrôle d'un `version.json`, par `Seed::take`), ou demande au plugin comme le cœur le fait (la vue Copilot CLI, par `terminal_launch`).
- Les scénarios en boîte noire lancent l'app sur un dossier de données jetable, jamais sur `~/.escouade`. Leurs ports se donnent en variables (`ESCOUADE_E2E_CDP_PORT`, `AUDIT_PORT_BASE`), jamais dans les fichiers.
- Ils ne se lancent que sur un binaire de développement (`npx tauri build --debug --no-bundle`). Un binaire publié ignore les variables par lesquelles la fixture remplace les connexions du poste par des fichiers d'essai : il lirait les vraies.
- `cargo geiger` nettoie les binaires de `src-tauri/target/debug` : reconstruire l'app avant la boîte noire.
- Aucun essai n'emploie de charge utile réutilisable : chaque « exploitation » écrit un fichier témoin dans un dossier de l'essai, ou mesure la mémoire de l'instance de test.

## Refaire l'audit sur une autre version

1. Rejouer la boîte blanche et la boîte noire. Un test `finding_*` qui échoue dit que sa faiblesse n'est plus là ; un test `holds_*` qui échoue, qu'un contrôle a cédé.
2. Relancer les outils, et refaire les annexes : `node docs/security/audit-2026-11/tools/annexes.mjs <dossier des sorties brutes>`.
3. Contrôler les binaires publiés par leurs signatures et leur attestation de provenance (annexe F) : téléchargés dans un dossier neuf hors du dépôt, jamais lancés.
4. Retrouver dans le nouveau code chaque `fichier:ligne` du rapport (`tools/references.mjs locate <commit>`, puis `move <commit>` pour réécrire celles qui ont seulement changé de ligne), relire ce que l'outil ne retrouve pas, puis mettre le commit de la version dans `AUDITED` et lancer `check`.
5. Reconstruire le PDF.
