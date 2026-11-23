---
number: null
id: "ce-qu-un-depot-apporte-est-lu-sans-lien"
title: "Ce qu'un dépôt apporte est lu sans lien et avec un plafond, sous Windows comme ailleurs"
date: "2026-11-23"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-653"
pages: ["kanban/tickets-partages","projets/escouade-yml","harness/memoire-partagee","docs/specification","kanban/workflows"]
tickets: ["ESC-653"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/tickets-partages","added":7,"removed":0},{"page":"projets/escouade-yml","added":4,"removed":0},{"page":"harness/memoire-partagee","added":1,"removed":0},{"page":"docs/specification","added":2,"removed":0},{"page":"kanban/workflows","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'audit Mantis d'un fork d'Escouade (OshOEz, mode serveur) a relevé dans le code commun à la 2.2.7 des lectures qui suivaient un lien commité ou lisaient sans plafond ce qu'apporte un dépôt cloné ou tiré : tickets partagés et leurs médias, .escouade.yml, mémoire, import de spec, fichier d'état d'un workflow, sortie de git show et git diff. Un collaborateur du dépôt pouvait ainsi faire copier un fichier du poste dans un ticket, ou figer et saturer l'app. Sous Windows, O_NOFOLLOW n'existe pas.

## Décision

Tout fichier qu'apporte un dépôt est lu comme fichier ordinaire, jamais à travers un lien (une jonction sous Windows en est un ; l'ouverture prend FILE_FLAG_OPEN_REPARSE_POINT à la place d'O_NOFOLLOW), jamais sur un tube nommé, et avec un plafond : 64 Kio par ticket partagé et 2 000 fichiers par dossier, 5 Mo par média, 1 Mio pour .escouade.yml et .gitmodules, 2 Mio par page et 32 Mio par import de spec, 500 fichiers et 8 Mio pour la mémoire, 2 Mo pour un fichier d'état de workflow, 64 Mo pour la sortie d'un diff ou d'un commit, 4 Mo pour un blob montré par lignes, 2 Mio pour la base d'un fichier ouvert dans l'éditeur. Rien n'est écrit ni effacé à travers un lien. Le mot de passe du proxy gardé hors trousseau ne va plus à la fenêtre.

## Conséquences

Un fichier au-delà de son plafond est dit illisible ou trop gros, au lieu d'être lu ; un dossier de tickets de plus de 2 000 fichiers n'est plus suivi. Les correctifs d'OshOEz sont repris tels quels (cherry-pick, auteur gardé), complétés pour Windows et étendus aux lectures du même motif (.gitmodules, lines_at, diff du worktree, base de l'éditeur, médias écrits dans le dossier partagé, « Prendre le sien », fichier de config d'un environnement de test).
