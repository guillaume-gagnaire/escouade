---
number: 313
id: "le-panneau-de-droite-se-redimensionne-pa"
title: "Le panneau de droite se redimensionne par son bord gauche, sa largeur gardée par forme"
date: "2026-11-23"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-659"
pages: ["git-fichiers/git-fichiers"]
tickets: ["ESC-659"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/git-fichiers","added":5,"removed":0}]
accepted_at: "2026-11-24T04:18:06+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le panneau de droite (Non commités, Historique, Workflow, PR et CI) avait une largeur fixe (340 px, ou 44 % de la zone de travail côte à côte), alors que la barre des projets et la barre latérale se tirent par leur bord et gardent leur largeur. Constat de l'utilisateur le 2026-11-23.

## Décision

La poignée Splitter connaît le bord gauche d'une colonne (edge: 'left', miroir du bord droit : tirée à gauche ou ← l'élargit). Le panneau l'a sur son bord gauche, ouvert et côte à côte, de 340 px à ce qui laisse 356 px à la conversation (1 200 px au plus). Deux largeurs gardées dans les réglages, panelWidth et splitPanelWidth (0 = par défaut : 340 px, 44 %), car les deux formes n'ont pas la même largeur par défaut.

## Conséquences

Les autres colonnes ne changent pas. Une fenêtre étroite borne le panneau à l'affichage sans perdre la largeur choisie. Les requêtes de conteneur du panneau (420 px) suivent sa largeur. Correctif de la ligne 2.2 (2.2.10), à reverser dans release/2.3.
