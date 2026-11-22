---
number: null
id: "un-harness-tiers-n-est-lance-nulle-part"
title: "Un harness tiers n'est lancé nulle part tant qu'un projet privé existe"
date: "2026-11-22"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-623"
pages: ["projets/projets-prives","plugins/harness-en-process"]
tickets: ["ESC-623","ESC-590"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/projets-prives","added":10,"removed":9},{"page":"plugins/harness-en-process","added":5,"removed":4}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-590 (SEC-02) refuse le harness d'un plugin tiers dans un projet privé, parce que le cœur ne peut juger sa clôture qu'en cherchant chaque chemin comme un mot de son lancement (test W-04 de l'audit de la 2.2.0). Mais la même clôture est ce qui tient les dossiers d'un projet privé hors de portée d'un agent d'un projet non privé : un agent tiers d'un projet public, détourné, pouvait lire le projet privé. Le choix a-28 proposait de garder la règle au seul projet privé (a), de refuser partout dès qu'un projet privé existe (b), ou d'attendre un contrat qui prouve la clôture (c). Tranché le 2026-11-22 par le chef, sur délégation de l'utilisateur (« la sécurité qui prime ») : option b.

## Décision

Tant qu'un projet privé existe sur le poste (ouvert dans Escouade), un harness d'un plugin tiers (ni le plugin qui le sert ni celui qui le décrit n'est dev.escouade.*) n'est lancé dans aucun projet, qu'il y soit activé ou non. Le refus le dit : « Un projet privé existe sur ce poste : un harness d'un plugin tiers ne peut pas garantir sa clôture, <harness> n'est lancé dans aucun projet. Prends un harness d'Escouade, ou retire « Projet privé » de tes projets. » (fr et en). Rendre un projet privé, ou ajouter un projet privé, arrête les agents tiers de tous les projets (au repos aussitôt, au travail à la fin de leur tour), avec ce mot dans leur conversation. Quand plus aucun projet n'est privé (rendu public ou retiré), les harness tiers redeviennent permis ; rien ne repart tout seul. Menus « Harness », Kanban, API distante, serveur MCP et lancement même du harness suivent ce jugement (harness_in, check_harness_in, Spawn::private).

## Conséquences

Ferme W-04 pour tous les projets. Un utilisateur qui a un seul projet privé perd les harness tiers dans tous ses projets : c'est dit dans les menus, les conversations, les tickets bloqués et sous « Projet privé ». Les harness d'Escouade ne changent pas. La règle pourra s'assouplir quand le contrat des plugins permettra à un tiers de prouver sa clôture (option c, à concevoir avec le kit et le SDK).
