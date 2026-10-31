---
number: 22
title: "L'index du code est retiré"
date: "2026-10-31"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-310"
pages: ["docs/index-du-code","docs/recherche-par-le-sens","docs/spec-a-jour","docs/planifier","docs/discussions","docs/documents","harness/memoire-partagee"]
tickets: ["ESC-310","ESC-306"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/index-du-code","added":9,"removed":91},{"page":"docs/recherche-par-le-sens","added":13,"removed":0},{"page":"docs/spec-a-jour","added":3,"removed":1},{"page":"docs/planifier","added":1,"removed":1},{"page":"docs/discussions","added":1,"removed":1},{"page":"docs/documents","added":1,"removed":1},{"page":"harness/memoire-partagee","added":1,"removed":1}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur avait fixé la règle le 2026-10-30 : si IDX-6 montre que l'index ne sert pas à grand-chose, il sort d'Escouade et chaque harness trouve le code lui-même. IDX-6 (ESC-306, docs/archive/codeindex/CODEINDEX-PROBES.md, « Changements à plusieurs endroits ») : avec de vrais agents, l'index + hook fait +1 % de coût à Sonnet (sous le bruit), +12 % à Opus (indicatif), −23 % à Haiku seulement, sans un oubli de moins (130 runs sur 130 justes dans tous les modes). Le seuil (−15 % avec Sonnet, ou moins d'oublis ou plus de réussites) n'est pas atteint. Décidé par l'utilisateur le 2026-10-31 : « Allez vire l'index ».

## Décision

L'index du code est retiré complètement : le module, ses six outils MCP index_*, la réponse du hook des vues et la règle des consignes, les réglages (par projet, sur tous les projets, l'assistant « Nouveau projet »), la partie des Stats, les bancs des sondes et des agents, les grammaires tree-sitter. Chaque harness trouve le code par ses propres outils ; pas de plugin d'indexation à la place. La comparaison de la spec au code et le planificateur nomment à leur agent les fichiers suivis par git qui contiennent les mots de code des pages (git grep, clôture respectée). La recherche par le sens de la mémoire et des docs reste, réglée dans Réglages › Projets › Tous les projets.

## Conséquences

À la mise à jour, une fois : les bases code-index.db du dossier de données sont effacées, rien d'autre ; la table des économies de l'index sort de la base des statistiques. Aucun hook propre à l'index n'avait été écrit dans les fichiers des harness (ceux des vues sont donnés à chaque lancement) : rien à y retirer. Un .escouade.yml qui porte code_index ou codeIndex se lit sans avertissement et ne l'a plus une fois réécrit. Les rapports de mesure restent dans docs/archive/codeindex. Le site perd sa page « Index du code » (redirigée vers « Mémoire partagée ») ; THIRD-PARTY-NOTICES ne porte plus de code de tokensave.
