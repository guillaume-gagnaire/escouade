---
number: null
id: "les-workflows-dans-le-workspace-l-onglet"
title: "Les workflows dans le Workspace : l'onglet Workflow retiré du panneau de droite, l'étape et le diagramme sur la carte d'un agent, « Workflows » dans la toile ; les commandes gardées par le résumé de toile masquées"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-689"
pages: ["fenetre/workspace","git-fichiers/git-fichiers","kanban/workflows","fenetre/reglages"]
tickets: ["ESC-689"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/workspace","added":25,"removed":4},{"page":"git-fichiers/git-fichiers","added":3,"removed":3},{"page":"kanban/workflows","added":11,"removed":11},{"page":"fenetre/reglages","added":2,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le Workspace d'OshOEz (PR #45, décision proposée « le-workspace-une-toile-des-agents-de-leu ») montre les agents d'un projet sur une toile, ouverte par le menu du bouton « Éditeur » et par la Vue d'ensemble. Les workflows du projet avaient leur propre onglet dans le panneau de droite (le rail des étapes, l'historique d'un agent, le diagramme), à côté de « Non commités », « Historique » et « PR et CI ». Le ticket proposait de faire de cet onglet un « Workspace » compact ; l'utilisateur l'a corrigé le 2026-11-26 : pas d'onglet Workspace, l'onglet Workflow disparaît sans remplaçant, l'entrée de la toile reste où la PR l'a mise, et les workflows entrent dans la toile, chaque agent portant le sien. Par ailleurs, le résumé de toile garde les 5 dernières commandes de chaque fil dans le fichier de l'agent et les envoie à la fenêtre : une commande tapée par un agent peut contenir un secret en clair.

## Décision

Le panneau de droite a trois onglets : « Non commités », « Historique » et « PR et CI » ; sous 420 px ils se resserrent et tiennent tous les trois. Le menu « Éditeur » / « Workspace » du bouton « Éditeur » et le bouton « Workspace » de la Vue d'ensemble restent.

Dans la toile, la carte d'un agent qui suit un workflow du projet dit son étape (« GSD · Planifier · 2ᵉ passage », « en cours »), et en rouge le retour qui l'y a renvoyé avec sa raison. Un clic ouvre le diagramme de l'ancien onglet, réutilisé tel quel, en grand sur le panneau de travail avec le chemin de cet agent. Une pastille « Workflows », en haut à droite de la toile d'un projet (à part de la barre flottante, qui garde ainsi sa largeur à 1024 px), ouvre ce que l'onglet offrait pour tout le projet : la liste de ses workflows avec « Tickets », leurs agents et où ils en sont, le diagramme de chacun sans agent avec le choix de la porte d'entrée, « Copier en Mermaid », « Réglages du workflow », et les états sans workflow. Échap ferme le diagramme, puis « Workflows », puis la toile. Le séparateur d'étape de la conversation ouvre la toile et le diagramme avec le chemin de l'agent. La vue « Colonne » de l'onglet (rail, historique, portes à mettre dans la saisie) part avec lui.

Les commandes gardées par le résumé de toile sont masquées avant d'être coupées, gardées et envoyées (`escouade_base::secrets::mask_secrets`) : la valeur d'un nom qui dit un secret, le mot de passe d'une adresse et de `curl -u`, les clés et jetons de forme connue, un bloc de clé privée ; une commande gardée en clair avant est masquée à sa relecture.

## Conséquences

La lecture des workflows, `.escouade/workflows/`, Réglages › Projets › Kanban › Workflow, l'outil MCP `get_workflow` et les faits `workflow.*` ne changent pas. `WorkflowPanel` et ses tests partent ; les e2e de l'onglet passent par la toile (workspace-workflows, aux deux tailles) ; « PR et CI » ne quitte plus la bande à 340 px. Le rail, l'historique en lignes et les portes à mettre dans la saisie n'existent plus : le chemin d'un agent se lit sur le diagramme, en dessin et en mots. Le masquage reconnaît des formes : un secret sans nom ni forme connue (`mysql -phunter2`) reste lisible ; masquer trop n'est pas grave, la commande n'étant que montrée.
