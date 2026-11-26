---
number: 340
id: "les-fichiers-du-serveur-d-equipe-suivent"
title: "Les fichiers du serveur d'équipe suivent les droits de leur projet, et seuls leur auteur ou un administrateur les effacent"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-638"
pages: ["serveur-equipe/serveur","serveur-equipe/passer-un-agent"]
tickets: ["ESC-638"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/serveur","added":8,"removed":3},{"page":"serveur-equipe/passer-un-agent","added":3,"removed":3}]
accepted_at: "2026-11-26T17:21:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (SR2-03) : les routes des fichiers du serveur d'équipe ne demandaient qu'une session de l'équipe. Un appareil gardé hors d'un projet lisait les fichiers chiffrés de ce projet, et tout membre effaçait définitivement n'importe quel fichier de l'équipe (documents, images, conversations passées). La défense en profondeur promise au §2.6 (« il ne sert les objets d'un projet qu'aux appareils qui y ont accès ») ne tenait pas pour les fichiers.

## Décision

Chaque fichier est rattaché à sa pose à un projet, ou à l'équipe par l'un de ses propres flux (store privé), par les routes /v1/projects/{p}/files/{f}. Il se lit selon les droits de ce projet (comme son flux ; sous un autre projet il n'est pas trouvé) et ne s'efface que par l'appareil qui l'a posé ou un administrateur. Les fichiers posés avant SR2-03, et ceux de l'ancienne route /v1/files/{f} gardée une version, ne sont rattachés à rien : seuls leur auteur et les administrateurs les lisent et les effacent, et leur lecture sous un projet les y rattache. Le dépôt d'une passation est effacé par qui passe la main, quand il voit l'offre finie.

## Conséquences

La migration n'ouvre rien de plus qu'avant : elle restreint. Un membre ne retrouve un ancien fichier d'un collègue (document, image) qu'une fois que son auteur ou un administrateur l'a rouvert dans son projet. Le client (escouade-team) nomme le projet à chaque lecture et effacement, et vérifie que le fichier ouvert est bien scellé pour ce projet. Une colonne project_id dans la table files (SQLite et PostgreSQL), nulle pour les fichiers d'avant.
