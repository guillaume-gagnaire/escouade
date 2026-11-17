---
number: 192
id: "les-docs-d-un-projet-d-equipe-sur-le-ser"
title: "Les docs d'un projet d'équipe sur le serveur, chiffrées, avec une copie sur le poste et un compteur de numéros dans le journal"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-482"
pages: ["serveur-equipe/docs-equipe","serveur-equipe/synchronisation","docs/decisions","docs/docs-dans-le-depot","docs/spec-a-jour","harness/serveur-mcp"]
tickets: ["ESC-482"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/docs-equipe","added":42,"removed":0},{"page":"serveur-equipe/synchronisation","added":5,"removed":4},{"page":"docs/decisions","added":6,"removed":0},{"page":"docs/docs-dans-le-depot","added":2,"removed":0},{"page":"docs/spec-a-jour","added":2,"removed":0},{"page":"harness/serveur-mcp","added":2,"removed":0}]
accepted_at: "2026-11-17T07:55:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-9 du serveur d'équipe (docs/TEAM-SERVER.md §2.2, §2.7) : la spec, les décisions et les documents d'un projet d'équipe doivent vivre sur le serveur, chiffrés, lisibles hors ligne, sans branche, avec les choix C2-A (versions entières des pages, fusion à trois voies sur le poste), C4-B (serveur par défaut, ou « dans le dépôt » au choix) et C5-A (pas de passerelle). Il fallait dire comment l'interface et les outils MCP, faits pour des fichiers de `.escouade/`, lisent ces docs ; comment deux acceptations au même moment n'ont jamais le même numéro ; et comment marquer pour un agent ce qu'a écrit un collègue.

## Décision

Les docs d'un projet d'équipe vont dans le journal du projet (celui de son Kanban, `Project::team_project`) : une page en versions entières, une décision en petits champs (le dernier gagne, le perdant le voit), un document en fichier de l'équipe chiffré sous une clé à lui, gardée dans le journal (20 Mo au plus). Chaque poste en tient une copie, un dossier `.escouade/` des données d'Escouade que tout lit comme avant (fenêtre, MCP, index, vecteurs), hors ligne compris ; un passage envoie ce que la copie a changé et y ramène le journal. Deux têtes d'une page sont fusionnées à trois voies sur le poste ; un vrai conflit se montre (« Garder la mienne », « Prendre la sienne », « Donner à l'agent rédacteur »). Le numéro d'une décision acceptée est la place de sa demande parmi celles du compteur de l'équipe dans l'ordre que le serveur donne au journal (opération `Number`) : le même sur chaque poste, jamais deux fois, sans que le serveur voie le compteur ; hors ligne, la décision est acceptée sans numéro jusqu'au retour. Le brouillon d'un ticket est la copie de la branche fixe (`docsticket.rs`), versée à la validation. Le choix serveur ou dépôt est un champ du journal, le même pour toute l'équipe ; passer au serveur importe `.escouade/` (ids et numéros gardés) sans rien retirer du dépôt, revenir au dépôt y écrit les docs du serveur. Ce qu'un collègue a écrit est marqué donnée de tiers par les outils MCP. Un projet privé ne part jamais.

## Conséquences

Les écrans, l'index et les outils des docs n'ont pas changé de modèle : ils lisent un dossier. Le serveur ne voit ni titre, ni chemin, ni texte, ni nom de document, ni compteur. Une décision acceptée hors ligne attend son numéro ; les pages qui la citent par son id passent à son numéro quand il vient. Aucun rôle ne restreint l'écriture de la spec (le rôle lecteur n'existe pas encore). L'auteur d'une version n'est pas encore nommé dans un conflit (annuaire chiffré, TS-4). ESC-479 (TS-6) ajoute le même champ `team_project` et la même fusion à trois voies (`merge.rs`, repris à l'identique) : leurs fusions se recouvrent.
