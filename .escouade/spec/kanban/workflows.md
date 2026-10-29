---
title: "Workflows"
subject: "Kanban"
sources: ["docs/SPEC.md"]
tickets: ["ESC-195"]
decisions: []
compared: null
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
- Restent sur le poste, dans `~/.escouade/projects/<id>/` : ce qui dit si la lecture est périmée (les dates des sources) et l'historique des agents.

### Le dossier fait foi

- Les workflows montrés et suivis (réglages, onglet, suivi, outil `get_workflow` du serveur MCP) sont ceux des fichiers, avec leurs corrections.
- Une lecture de la machine ajoute un workflow nouveau et met à jour celui qui a changé (« mis à jour par la relecture », avec les étapes ajoutées ou disparues). Elle ne supprime jamais un fichier.
- « Ce n'est pas un workflow » le garde, marqué comme tel. « Retirer du dépôt » supprime le fichier d'un workflow que la machine ne lit pas.
- Les fichiers sont relus au démarrage et quand le dossier change, comme les tickets partagés. Un fichier illisible est dit avec son chemin et sa raison, sans toucher aux corrections gardées.
- Escouade ne commite ni ne pousse jamais ces fichiers ; l'utilisateur peut les commiter.

### Dépôt ou machine

- Ne vont dans le dépôt que les workflows tirés de sa configuration : `.claude/`, `.agents/` en entier, `.github/`, `.opencode/`, `.cursor/`, `PROMPT.md`. On regarde leur porte d'entrée, même si une étape appelle une skill de l'utilisateur.
- Ceux des dossiers de l'utilisateur (`~/.claude`, `~/.codex`, GSD ou Superpowers installés là) sont lus, montrés et suivis, mais leurs fichiers restent dans `~/.escouade/projects/<id>/workflows/`, au même format.

## Réglages

« Où garder la configuration des workflows », réglage du projet : « Dans le dépôt » par défaut. Il ne porte que sur les workflows du dépôt ; changer de choix déplace leurs fichiers, sans perte ni doublon.
