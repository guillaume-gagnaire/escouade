---
number: 18
title: "Un workflow supprimé n'est plus recréé, et un projet peut n'avoir aucun workflow"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-309"
pages: ["kanban/workflows","projets/escouade-yml","projets/nouveau-projet"]
tickets: ["ESC-309"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/workflows","added":17,"removed":4},{"page":"projets/escouade-yml","added":4,"removed":2},{"page":"projets/nouveau-projet","added":2,"removed":2}]
accepted_at: "2026-10-31T10:03:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Retour d'un utilisateur sur le Discord (2026-10-30) : un workflow dont il effaçait le fichier, ou qu'il retirait d'Escouade, revenait à chaque lecture (ouverture du projet, relance, suivi). Cause : merge_reading (workflowfiles.rs), appelé par refresh_workflows et read_if_never_read, écrivait tout workflow lu qui n'avait pas de fichier ; aucune suppression n'était retenue. Et rien ne permettait de dire qu'un projet ne veut pas de workflow.

## Décision

1) Chaque machine retient, par projet (~/.escouade/projects/<id>/workflows-removed.json), les workflows qui avaient un fichier à la dernière lecture et ceux que l'utilisateur a supprimés (« Supprimer ce workflow », « Retirer du dépôt », ou fichier effacé à la main), avec l'empreinte de la lecture qui les avait trouvés. Une lecture n'écrit jamais un workflow supprimé ; si l'empreinte change, il est seulement proposé (« Reprendre » / « Laisser supprimé ») ; « Rétablir » le relit. Un fichier revenu (pull, branche) rend le workflow au projet ; un fichier illisible ne vaut pas suppression. 2) « Pas de workflow pour ce projet » : un réglage du projet (workflows_enabled), dans l'assistant (étape Kanban) et Réglages › Projets › Workflow, partagé par la clé workflows.enabled de .escouade.yml ; tant qu'il est choisi, aucune lecture ni écriture de workflow, rien de montré ni de suivi, les fichiers existants laissés tels quels.

## Conséquences

Les suppressions restent propres à chaque machine (rien d'une machine dans le dépôt) : un coéquipier qui veut écarter un workflow pour toute l'équipe garde « Ce n'est pas un workflow ». Au premier lancement après la mise à jour, un fichier effacé avant elle peut être réécrit une dernière fois (rien n'en était retenu). Une nouvelle clé dans .escouade.yml et son schéma.
