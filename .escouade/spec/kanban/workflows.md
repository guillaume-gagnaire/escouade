---
title: "Workflows"
subject: "Kanban"
sources: ["docs/SPEC.md","Retour d'un utilisateur sur le Discord (2026-10-30), ticket ESC-309"]
tickets: ["ESC-195","ESC-309"]
decisions: []
compared: "2026-10-31T10:04:30+01:00"
gaps: [{"text":"La page dit que rien d'une machine ni aucune empreinte n'entre dans le fichier. Quand un workflow a été affiné, le code écrit dans `workflow.refined` l'empreinte de la lecture d'origine, et cette empreinte est versionnée dans le dépôt.","code":["src-tauri/src/workflowfiles.rs","src-tauri/src/workflow.rs"]}]
---

## Ce que c'est

Escouade lit les workflows d'un projet : les enchaînements que suivent ses agents (étapes, commandes, sous-agents, portes, retours, boucles de prompt), tirés de la configuration du dépôt et de celle de l'utilisateur. Ce qu'elle en comprend est gardé dans un fichier par workflow, dans le dépôt, pour que l'équipe le partage et le corrige. L'historique de leurs exécutions reste sur le poste.

## Comportement

### Le fichier

- Un fichier par workflow détecté : `.escouade/workflows/<nom>.yml`, à côté de `.escouade/tickets/`. Pour un projet de plusieurs dépôts dont le dossier n'en est pas un, c'est dans le premier dépôt, comme les tickets partagés ; le dépôt de chaque workflow est dans son fichier.
- `<nom>` : l'id du workflow réduit à `[a-z0-9._-]` (`|claude|gsd` devient `claude-gsd`) ; l'id exact est dans le fichier.
- `workflow` : la définition lue (id, nom, sorte, harness, dépôt, portes, étapes avec leurs commandes, sous-agents, lectures et écritures, transitions et retours, fichiers d'état, sortie, boucle de prompt). `fixes` : les corrections de l'utilisateur, à part.
- Le schéma, `schemas/escouade-workflow.schema.json`, est publié par le site.
- Rien d'une machine : ni date, ni empreinte, ni chemin absolu (un chemin hors du projet et de `~` n'est que son nom), ni l'état du travail, relu sur place. L'écriture est déterministe, les clés dans un ordre fixe : une relecture sans changement ne touche aucun fichier.
- Restent sur le poste, dans `~/.escouade/projects/<id>/` : ce qui dit si la lecture est périmée (les dates des sources), les workflows supprimés (`workflows-removed.json`) et l'historique des agents.

### Le dossier fait foi

- Les workflows montrés et suivis (réglages, onglet, suivi, outil `get_workflow` du serveur MCP) sont ceux des fichiers, avec leurs corrections.
- Une lecture de la machine ajoute un workflow nouveau et met à jour celui qui a changé (« mis à jour par la relecture », avec les étapes ajoutées ou disparues). Elle ne supprime jamais un fichier, et ne recrée jamais celui d'un workflow supprimé (voir plus bas).
- « Ce n'est pas un workflow » le garde, marqué comme tel, pour toute l'équipe. « Retirer du dépôt » supprime le fichier d'un workflow que la machine ne lit pas ; il compte alors comme supprimé.
- Les fichiers sont relus au démarrage et quand le dossier change, comme les tickets partagés. Un fichier illisible est dit avec son chemin et sa raison, sans toucher aux corrections gardées.
- Escouade ne commite ni ne pousse jamais ces fichiers ; l'utilisateur peut les commiter.

### Supprimer un workflow

- « Supprimer ce workflow », sous ses étapes dans Réglages › Projets › Workflow, supprime son fichier tout de suite. Un fichier effacé à la main vaut la même chose : la machine retient les workflows qui avaient un fichier à la dernière lecture, et celui dont le fichier manque depuis est supprimé.
- Un workflow supprimé sort d'Escouade (ni montré, ni suivi) et aucune lecture ne réécrit son fichier, ni la relecture, ni le démarrage, ni la première lecture d'un projet. La machine le retient pour ce projet, avec l'empreinte de la lecture qui l'avait trouvé et son nom.
- Si ce qui l'a fait trouver change (l'empreinte de la lecture n'est plus la même), il est proposé à nouveau, jamais écrit : « supprimé, mais ce qui l'a fait trouver a changé : le reprendre ? », avec « Reprendre » et « Laisser supprimé » (qui retient la nouvelle empreinte).
- La liste « Supprimés » de Réglages › Projets › Workflow les montre, chacun « supprimé, ne sera pas recréé » avec « Rétablir », qui le relit et réécrit son fichier. Un toast dit la suppression et comment la défaire.
- Un fichier qui revient (un `git pull`, une autre branche) rend le workflow au projet. Un fichier illisible, ou un dossier refusé, ne vaut pas une suppression.
- Pourquoi : chaque lecture écrivait tout workflow lu sans fichier, si bien qu'un workflow supprimé (fichier effacé, « Retirer du dépôt ») revenait à la lecture suivante (retour d'un utilisateur, ESC-309).

### Pas de workflow pour ce projet

- Un choix du projet, « Workflows du projet » : « Lire les workflows » (par défaut) ou « Pas de workflow pour ce projet ». Dans l'assistant « Nouveau projet » (étape Kanban) et en tête de Réglages › Projets › Workflow ; partagé par la clé `workflows.enabled` de `.escouade.yml` (`false` : pas de workflow).
- Tant qu'il est choisi : aucune lecture (ni relecture, ni au démarrage, ni première lecture), aucune écriture (ni fichiers, ni corrections, ni lecture gardée), aucun workflow montré ni suivi. Ce qui est déjà dans `.escouade/workflows/` reste tel quel.
- Les réglages le disent avec comment le défaire : « Pas de workflow pour ce projet : Escouade ne lit ni n'écrit ses workflows et n'y suit aucun agent ; .escouade/workflows/ reste tel quel. Pour le défaire, choisis « Lire les workflows » et enregistre. » L'onglet Workflow aussi (« Pas de workflow pour ce projet », « Ouvrir les réglages du workflow »).
- Le défaire relit les workflows du projet, ses suppressions gardées.

### Dépôt ou machine

- Ne vont dans le dépôt que les workflows tirés de sa configuration : `.claude/`, `.agents/` en entier, `.github/`, `.opencode/`, `.cursor/`, `PROMPT.md`. On regarde leur porte d'entrée, même si une étape appelle une skill de l'utilisateur.
- Ceux des dossiers de l'utilisateur (`~/.claude`, `~/.codex`, GSD ou Superpowers installés là) sont lus, montrés et suivis, mais leurs fichiers restent dans `~/.escouade/projects/<id>/workflows/`, au même format.

## Réglages

- « Workflows du projet » : « Lire les workflows » (par défaut) ou « Pas de workflow pour ce projet » (voir plus haut).
- « Où garder la configuration des workflows », réglage du projet : « Dans le dépôt » par défaut. Il ne porte que sur les workflows du dépôt ; changer de choix déplace leurs fichiers, sans perte ni doublon.
