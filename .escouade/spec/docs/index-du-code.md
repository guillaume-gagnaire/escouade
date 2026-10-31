---
title: "Index du code (retiré)"
subject: "Docs"
sources: ["docs/archive/codeindex/CODEINDEX-PROBES.md","src-tauri/src/paths.rs","src-tauri/src/projectfile.rs","src-tauri/src/stats.rs","src-tauri/src/tui.rs"]
tickets: ["ESC-251","ESC-253","ESC-293","ESC-306","ESC-307","ESC-310"]
decisions: []
compared: "2026-10-30T21:05:05+01:00"
---

## Ce que c'est

Escouade n'a plus d'index du code : il a été retiré le 2026-10-31 (ESC-310). **Chaque harness trouve le code par ses propres outils** (grep, lecture des fichiers, ses outils de recherche) ; Escouade n'en fournit pas d'autre, ni de plugin d'indexation à la place.

Pourquoi : mesuré avec de vrais agents sur des changements à plusieurs endroits (IDX-6, ESC-306, `docs/archive/codeindex/CODEINDEX-PROBES.md`), l'index et son hook coûtaient +1 % à Sonnet (sous le bruit), +12 % à Opus (indicatif) et −23 % à Haiku seulement, sans un oubli de moins (130 runs sur 130 justes dans tous les modes). Le seuil fixé d'avance (−15 % avec Sonnet, ou moins d'oublis, ou plus de réussites) n'était pas atteint.

## Comportement

- Plus d'outils MCP `index_*` (`index_search`, `index_context`, `index_body`, `index_callers`, `index_callees`, `index_impact`), plus de règle sur l'index dans les consignes des agents.
- Les hooks des vues restent (ils disent à Escouade ce que fait l'agent) ; le serveur des hooks répond `{}` à tout événement, une recherche de symbole comprise. Escouade n'avait écrit aucun hook propre à l'index dans les fichiers des harness : ceux des vues sont donnés à chaque lancement (`--settings` de Claude Code, options `-c` de Codex, plugin de l'agent pour Copilot CLI), il n'y a donc rien à y retirer.
- **À la mise à jour, une fois** : les bases de l'index (`projects/<id>/code-index.db`, avec ses `-wal`, `-shm`, `-journal`) sont effacées du dossier de données, et rien d'autre ; un marqueur dans le dossier de données empêche de le refaire. Les réglages enregistrés de l'index (« Index du code sur tous les projets », le choix d'un projet) sont ignorés à la lecture.
- **`.escouade.yml`** : la clé n'a jamais fait partie du fichier ; un fichier qui porte `code_index` ou `codeIndex` se lit sans erreur ni avertissement (la clé est ignorée, dite une fois au journal), et le fichier réécrit par Escouade ne l'a plus.
- **Statistiques** : la partie « Index du code » n'existe plus ; la table de ce qu'il disait faire économiser est supprimée de la base des statistiques, le reste gardé.
- La comparaison de la spec au code et le planificateur nomment à leur agent les fichiers suivis par git qui contiennent les mots de code des pages (voir *Spec tenue à jour* et *Planifier*).
- La recherche par le sens de la mémoire et des docs reste (voir *Recherche par le sens*).
