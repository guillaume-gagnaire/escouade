---
number: 178
id: "les-projets-d-equipe-se-synchronisent-pa"
title: "Les projets d'équipe se synchronisent par un journal d'opérations signées et chaînées, avec un groupe MLS par projet limité"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-466"
pages: ["serveur-equipe/synchronisation","serveur-equipe/client","serveur-equipe/serveur"]
tickets: ["ESC-466"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/synchronisation","added":42,"removed":0},{"page":"serveur-equipe/client","added":7,"removed":3},{"page":"serveur-equipe/serveur","added":6,"removed":4}]
accepted_at: "2026-11-15T17:39:15+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-5 (`docs/TEAM-SERVER.md` §2.4, §2.5, §3.8, §4.3) devait dire comment les projets d'une équipe se partagent sur le serveur d'équipe, avec les choix C2-A (journal d'opérations, champ par champ, le dernier qui écrit gagne sous une horloge hybride), C9-B (MLS) et C10-A (bout en bout seul mode). Restaient ouverts : comment garder un projet à quelques membres, ce que voit le serveur d'une opération, comment un serveur menteur se voit, et comment compacter le journal.

## Décision

- Chaque opération porte son auteur, son rang et l'empreinte de l'opération d'avant du même appareil, la tête du journal des membres connue de l'auteur et son horodatage hybride. Elle est signée par la feuille MLS de l'auteur, puis chiffrée. Le serveur n'en voit que l'époque, la taille, l'heure et l'appareil qui l'envoie.
- Un projet de toute l'équipe prend la clé de l'équipe ; un projet gardé à quelques membres a son propre groupe MLS (ses feuilles : les appareils de ces membres, entrés par leur KeyPackage de dernier recours publié dans le flux d'équipe ; seul un administrateur change qui a accès ; ses commits dans le flux du projet, le premier d'une époque gagnant).
- La fusion : le dernier qui écrit gagne, champ par champ ; une écriture perdante faite depuis la même valeur que la gagnante est gardée comme conflit ; une suppression est définitive ; une page de spec va en versions entières, deux versions depuis la même base étant un conflit gardé (la fusion à trois voies attend TS-9).
- La copie locale est SQLite, en clair comme le Kanban local, un fichier par projet ; l'interface la lit, jamais le serveur. Ce qui s'écrit hors ligne attend dans une file et part dans l'ordre, chiffré à l'époque du moment.
- Une opération rejouée, réordonnée, retirée, tue ou forgée arrête l'équipe sur l'appareil, comme une fourche (alarme « journal »).
- Au-delà de 10 000 entrées, un appareil écrit un instantané chiffré et signé, avec la chaîne de chaque appareil ; le serveur oublie les opérations d'avant, pas les commits.
- L'app tient la connexion en direct de chaque équipe ; la relecture passe de 3 s à 15 s et ne sert que si le direct ne passe pas.

## Conséquences

- TS-6 n'a plus qu'à brancher le Kanban sur les opérations des tickets et l'état « Hors ligne · N changements en attente » (`team_sync_status`) ; TS-9 fait la fusion à trois voies des pages sur le conflit gardé.
- Le serveur gagne des commits par projet et des instantanés (SQLite et PostgreSQL, colonnes ajoutées par la migration).
- Pour l'audit (TS-20) : la copie locale en clair, la confiance faite aux membres pour les instantanés et pour l'horodatage (une horloge en avance gagne sur les autres), la politique des groupes de projet évaluée contre la vue de l'équipe du vérificateur.
- Restent : les droits par projet sur le serveur (il sert aujourd'hui un projet à tout appareil de l'équipe, la clé seule le protège), l'écriture des liens d'historique manquants d'un projet, le long fuzzing (TS-18).
