---
number: 31
id: "les-faits-workflow-et-le-lecteur-de-work"
title: "Les faits workflow.* et le lecteur de workflow des plugins"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-200"
pages: ["kanban/workflows"]
tickets: ["ESC-200"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/workflows","added":14,"removed":0}]
accepted_at: "2026-11-01T11:32:32+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le choix 158 d'ESC-191 veut que le workflow parle aux plugins : ses faits sur le bus, et un point d'extension pour un workflow qu'Escouade ne lit pas seule.

## Décision

Cinq faits (workflow.step_entered, step_left avec durée, coût et issue, looped_back avec sa raison, left, changed) sont émis d'après les passages du suivi des agents. Un point `contributes.workflowReaders` (id, name, files en globs du projet) fait appeler `workflow/read` avec les seuls fichiers désignés ; le plugin rend un graphe simple (étapes, transitions), refusé en entier s'il ne tient pas, et suivi sous l'id `plugin:<plugin>:<id>`.

## Conséquences

Le bus passe à 39 faits (version 1, champs ajoutés seulement) ; le schéma du protocole, le SDK et les pages du site les listent. Le lecteur n'est pas encore appelé par l'app : le registre des plugins lira les manifestes avec INT-6.
