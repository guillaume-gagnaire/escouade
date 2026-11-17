---
number: 194
id: "rattacher-a-l-equipe-le-kanban-dans-la-l"
title: "Rattacher à l'équipe : le Kanban dans la liste, verser les tickets et la clé team du dépôt"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-493"
pages: ["serveur-equipe/kanban-equipe","serveur-equipe/projets-equipe","serveur-equipe/fichier-equipe","serveur-equipe/docs-equipe","kanban/tickets-partages","projets/escouade-yml"]
tickets: ["ESC-493"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/kanban-equipe","added":34,"removed":9},{"page":"serveur-equipe/projets-equipe","added":12,"removed":7},{"page":"serveur-equipe/fichier-equipe","added":3,"removed":1},{"page":"serveur-equipe/docs-equipe","added":2,"removed":1},{"page":"kanban/tickets-partages","added":6,"removed":0},{"page":"projets/escouade-yml","added":21,"removed":0}]
accepted_at: "2026-11-17T11:59:08+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-7 du serveur d'équipe (docs/TEAM-SERVER.md §2.7, C5-A choisi le 2026-11-14). Après TS-6 (ESC-479), un collègue rejoignait un Kanban d'équipe en recopiant un identifiant de 32 caractères ; après TS-8 (ESC-480), la liste chiffrée des projets ne portait pas ce Kanban. Les tickets d'un projet qui devient projet d'équipe (ceux de ce poste, ceux partagés par fichiers .escouade/tickets/*.yml d'ESC-190) restaient sur le poste. Rien dans le dépôt ne disait à quelle équipe il appartient, et un dépôt qui le dirait ne doit pas pouvoir brancher un poste sur un serveur.

## Décision

1. L'entrée d'un projet de la liste de l'équipe porte l'identifiant du journal de son Kanban (`board`, chiffré avec l'entrée ; absent ou vide pour une entrée d'avant : pas de Kanban connu). L'administrateur qui partage le Kanban l'écrit ; un membre le propose dans un flux de l'équipe à part (`escouade/propose`, scellé, signé, ouvert à toute l'équipe), qu'un administrateur inscrit par « Inscrire le Kanban ». Un projet rattaché ou cloné depuis la liste rejoint ce Kanban sans identifiant à copier ; « Rejoindre » par identifiant reste pour un projet hors de la liste. Le serveur refuse qu'on restreigne l'accès aux flux de l'équipe (liste, fichier d'équipe, propositions).
2. « Verser à l'équipe », après un aperçu (ce qui part, ce qui reste, les tickets un à un) et par le seul geste de l'utilisateur : les tickets des fichiers .escouade/tickets/*.yml entrent dans le journal sous un objet tiré de l'id de leur fichier (le même sur tous les postes : deux versements, ou deux postes en même temps, ne font jamais de doublon ; un ticket déjà là ou supprimé n'est pas réécrit) ; les tickets de ce poste choisis (tout cocher possible) entrent aussi. Un ticket versé prend une clé du compteur du serveur, sauf un ticket importé qui garde la sienne ; l'ancienne clé (DEM-12, DEM-12@alice, DEM-L3) est gardée et partagée (`formerKeys`) et trouve encore le ticket (outils MCP, liens de la spec et des décisions, carte).
3. Avec TS-9 (ESC-482) : rattacher, partager ou rejoindre un Kanban ne verse pas non plus la spec, les décisions et les documents de .escouade/. Le premier import des docs d'un projet n'arrive plus seul au premier passage : tant que l'équipe n'a rien choisi ni rien importé, ses docs restent celles du dépôt sur chaque poste ; elles partent par le réglage « Sur le serveur d'équipe » ou par « Verser à l'équipe », dont l'aperçu les nomme (pages, décisions, documents, fichier par fichier, case cochée) ou dit pourquoi elles ne partent pas (déjà sur le serveur, gardées dans le dépôt, projet privé, aucune). La mémoire partagée reste dans le dépôt (TS-10).
4. C5-A : dès que le Kanban d'un projet est celui de l'équipe, Escouade ne lit ni n'écrit plus ses .yml, met en tête de chacun la ligne commentaire « # Frozen: this project joined a team server… » (lue sans erreur par le lecteur d'ESC-190), propose de les retirer et dit lesquels ; il ne supprime ni ne commite rien.
5. La clé `team` du .escouade.yml du dépôt (server, id, project, name ; rien de secret) est écrite au rattachement et retirée au détachement. À l'ouverture d'un dépôt qui la porte : bande « Ce projet appartient à l'équipe Acme sur team.acme.fr, d'après le dépôt » avec « Demander une invitation » (ouvre l'écran d'entrée, l'adresse préremplie) et « Ne plus proposer pour ce projet » ; sur un poste de cette équipe (même id, même serveur), « Rattacher à l'équipe » à la place. Aucune requête ne part vers l'adresse du fichier ; l'adresse est vérifiée comme une adresse tapée (https, http vers cette machine seulement, sans chemin ni identifiants), le nom nettoyé (contrôle, direction du texte, 60 caractères). Le Kanban de la liste n'est rejoint que si un remote du projet est un dépôt de la liste. La clé n'est jamais écrite à travers un .escouade.yml qui n'est pas un fichier ordinaire.
6. Le MCP dit `team` (attached, board, namedByRepository) dans list_projects et get_board ; aucun outil ne rattache ni ne verse.

## Conséquences

Un collègue clone ou rattache un projet de la liste et voit le Kanban, ses tickets et les docs de l'équipe sans rien copier ; une équipe migre ses tickets de fichiers et ses docs sans doublon, même à plusieurs postes, et voit tout ce qui part avant le geste ; les postes d'avant la 2.3 lisent des fichiers figés, qui le disent. Coûts : un flux de plus sur le serveur (les propositions) ; deux postes qui versent le même ticket en même temps consomment deux numéros du compteur (un seul reste) ; l'objet d'un ticket versé est tiré de l'id public de son fichier (le serveur ne le voit pas, mais un membre qui a lu le dépôt peut y écrire avant le versement, comme sur tout ticket de l'équipe). Les docs d'un projet d'équipe dont personne n'a fait le geste restent dans le dépôt (l'ESC-482 les importait au premier passage). Un poste qui suit déjà un autre Kanban que celui de la liste n'en change pas seul : l'écran le dit. L'import de la mémoire de .escouade/ reste à TS-10.
