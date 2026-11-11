---
number: 144
id: "le-retrait-du-natif-plus-de-plugin-donne"
title: "Le retrait du natif : plus de plugin donné par l'app, plus de run.command lancé (Z3)"
date: "2026-11-10"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-355"
pages: []
tickets: ["ESC-355"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-11T02:02:49+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-355, Z3 du plan de docs/PLUGINS.md §8.10. Tous les plugins de première main sont des modules publiés, signés et dans l'amorce (Z2) ; il restait le natif : escouade-plugins, le transport par process, run.command, l'origine builtin et ses contrôles, BUILTINS du store, les attestations des binaires. Décision de l'utilisateur du 2026-11-04 : tous les plugins en WASM en 2.2, ceux de première main sont des plugins à part entière.

## Décision

- Escouade ne lance plus aucun programme natif de plugin, quelle que soit l'origine : un manifeste à run.command se lit, le plugin se charge coupé et sa fiche, la fenêtre d'installation et le banc disent pourquoi. Le store le refuse à la soumission (contrôle n° 10, déjà).
- escouade-plugins, le transport par process de runtime.rs (Job, mémoire lue, pid), check_program, {escouade}/…, builtin:<version>, la main Builtin du manifeste et « l'identifiant de l'app l'emporte » disparaissent ; l'app ne télécharge plus le programme d'une release.
- Le registre de builtin.rs reste, comme liste des identifiants de l'amorce et source de leurs manifestes. Une build de développement et les tests lancent les modules de scripts/plugins-wasm.sh que l'amorce n'a pas, sous une origine workspace qu'une release ne fait jamais ; ce qui est installé d'un identifiant garde sa place.
- Une entrée « builtin » d'un plugins.json de 2.1 n'est plus un plugin : ses choix sont gardés comme « Garder ses réglages », et le tri de l'amorce l'installe avec eux s'il le désigne. Une entrée illisible ne fait plus perdre les autres.
- La langue est dite à tout plugin par initialize (plus d'environnement de process) ; un changement relance ceux qui tournent.
- Au store : BUILTINS et la confiance « Livré avec Escouade » partent ; la revue locale ne lance plus gh attestation verify et n'écrit plus binaries/. Le refus à la signature d'une version à binaire non attesté reste, inerte faute de binaire.

## Conséquences

- Les tests qui passaient par un process passent sur des modules : le faux plugin (tests/fixtures/plugins/fake/plugin.mjs, run.js) et la démonstration minimal (run.wasm commité, harness tests/fixtures/fake-minimal.mjs). Les tests propres au process (Job, pid, chemin du programme) sont partis. Les tests run.js demandent escouade-js construit, comme la CI.
- Un auteur dont le plugin est en run.command, installé d'un dossier, doit le porter en run.js ou run.wasm.
- L'installeur perd escouade-plugins (la démonstration seule, environ 1 Mo en clair).
- ESC-227 (la doc de référence) peut partir : les pages du site disent déjà que run.command ne démarre plus.
