---
number: 100
id: "la-relecture-d-une-pr-par-un-agent-un-br"
title: "La relecture d'une PR par un agent : un brouillon, un agent sans outil, un seul chemin vers l'hébergeur"
date: "2026-11-06"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-66"
pages: ["git-fichiers/relecture-par-un-agent","git-fichiers/vue-pull-requests","git-fichiers/pull-requests"]
tickets: ["ESC-66"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/relecture-par-un-agent","added":47,"removed":0},{"page":"git-fichiers/vue-pull-requests","added":4,"removed":4},{"page":"git-fichiers/pull-requests","added":3,"removed":3}]
accepted_at: "2026-11-07T00:10:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision du 2026-10-16 veut qu'une relecture rédigée par un agent attende la validation de l'utilisateur. Il restait à dire comment l'agent lit une PR, y compris celle d'un inconnu sur un dépôt public, comment rien ne touche au dépôt, et où se fait la validation. Le protocole des hébergeurs publie une relecture (`review`) mais ne donne ni le diff ni le texte de la PR, et ni `crates/` ni `plugins/` ne devaient changer. Un diff hostile peut demander au modèle de lire `~/.ssh/id_rsa` ou un `.env` : le texte reviendrait dans un commentaire que l'utilisateur pourrait garder et publier. Le poids `Reading` donne des outils de lecture, et un dossier vide ne protège pas partout : Codex en `--sandbox read-only` lit tout le disque.

## Décision

Le diff vient de git, récupéré dans des références `refs/escouade/review/<n>/…` supprimées aussitôt : aucune branche ne bouge. L'agent est une question ponctuelle d'un troisième poids, `Weight::Given` : le modèle de `Reading`, aucun outil, comme `Light`, car tout est dans la question. Il tourne en plus dans un dossier vide à lui : deux barrières, et une consigne n'en est pas une. Un harness doit dire qu'il sait répondre sans outil (`Engine::answers_without_tools`, faux par défaut) ; sinon la relecture est refusée, d'emblée et dans `oneshot::ask`. C'est le cas de Cursor CLI, dont le mode Ask lit toujours le disque, et des harness de plugin, que le protocole ne permet pas de vérifier. Titre, branches, diff et commentaires arrivent balisés, comme des données. La réponse ne donne que du texte et des commentaires : le verdict est celui de l'utilisateur, « Commenter » au départ. La relecture est un brouillon gardé avec l'état de l'app (une par PR). Elle se valide dans la vue « Pull requests », en place de la liste : une relecture demande plus de temps qu'une modale n'en mérite et ne doit pas bloquer les agents qui attendent. Chaque commentaire se tranche un par un, sans « tout garder ». La publication est une commande de la fenêtre et aucun outil MCP ne l'atteint. Elle relit la PR : des commits arrivés depuis la relecture retiennent tout jusqu'à une confirmation, et un commentaire dont la ligne n'est pas dans le diff part dans le texte plutôt que de faire refuser toute la relecture.

## Conséquences

Rien à publier sur le store pour ce ticket : `crates/` et `plugins/` ne changent pas. L'agent ne voit que le diff et son contexte, jamais le reste du code ni le texte de la PR, que le protocole ne donne pas : y ajouter ce texte demandera une version du protocole. Pas de relecture sur Cursor CLI ni sur un harness de plugin ; le dire pour ces derniers demanderait un champ du protocole. Sur GitLab, Bitbucket et Azure DevOps, les commentaires partent un par un : un échec au milieu peut en laisser partir une partie, ce que le message d'erreur dit. La question ponctuelle coûte le modèle de lecture du harness des textes rédigés.
