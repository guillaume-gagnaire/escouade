---
number: 108
id: "sur-opencode-seuls-les-git-add-et-git-co"
title: "Sur OpenCode, seuls les git add et git commit simples passent d'office, et .git reste à la carte"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-364"
pages: ["plugins/opencode-en-plugin","projets/projets-prives","plugins/claude-code-en-plugin"]
tickets: ["ESC-364"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/opencode-en-plugin","added":10,"removed":0},{"page":"projets/projets-prives","added":5,"removed":0},{"page":"plugins/claude-code-en-plugin","added":2,"removed":0}]
accepted_at: "2026-11-07T09:19:07+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

OpenCode n'a pas de bac à sable : ses règles sont sa seule protection. En « Accepter les modifications », `git add *` et `git commit *` couvraient une redirection (`git commit -m x >> fichier`, essayé sur la 1.19.3) et `-F` ; `edit *` couvrait `.git`, y compris en « Demander » ; une copie de session partait sans attendre ses règles ; les règles `edit` de la clôture, en absolu, ne servaient pas ; « Toujours » passait après les `deny` (essayé). Le matcher d'OpenCode ignore la casse et ne connaît que `*` et `?` : une liste de formes dangereuses ne peut pas dire « un mot qui finit par F » sans prendre aussi chaque `-m "fix`.

## Décision

Les formes permises de `git add` et `git commit` sont listées, plutôt que les formes dangereuses : `git add <chemins>`, `-A`, `-u`, `--all`, `--update`, `git commit -m`, `-am`, sans seconde option, redirection ni développement de shell ; tout le reste va à la carte. `.git`, `.opencode`, `opencode.json`, `opencode.jsonc`, `.husky` et `.githooks` vont à la carte hors « Sans permissions », sous-agents compris. Les règles d'une session sont attendues avant son ouverture, et leur échec refuse la session ; un changement de mode les attend aussi. Sous clôture : `edit` est écrit depuis le `worktree` d'OpenCode, « Toujours » n'est pas offert, et les options qui lisent un fichier sont refusées dans tous les modes. Une discussion de Claude Code refuse aussi `PowerShell`, `Monitor` et `EnterWorktree`.

## Conséquences

En « Accepter les modifications », `git commit --amend`, `--no-verify`, `git add -f`, et un message qui contient ` -`, `>`, `<`, `$` ou `%` demandent une carte de plus. Le mot de passe du serveur reste lisible par les commandes d'un agent : le README et la spec le disent ; le masquer demanderait que le cœur écrive un plugin en fichier d'OpenCode (`shell.env`), hors de ce ticket. Un `core.hooksPath` qui pointe ailleurs que `.husky` ou `.githooks` n'est pas couvert. Un « Toujours » hors clôture garde ce que la carte annonce (`*` pour une écriture), donc lève ces gardes pour le reste de l'instance.
