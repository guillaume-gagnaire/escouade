---
number: 260
id: "une-equipe-recommande-ou-impose-un-regla"
title: "Une équipe recommande ou impose un réglage simple de plugin sur ses projets"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-532"
pages: ["plugins/profils-et-reglages-d-un-projet","serveur-equipe/fichier-equipe"]
tickets: ["ESC-532"]
documents: ["docs/PLUGINS-CONTRACT-STUDY.md"]
supersedes: []
superseded_by: null
changes: [{"page":"plugins/profils-et-reglages-d-un-projet","added":28,"removed":15},{"page":"serveur-equipe/fichier-equipe","added":14,"removed":0}]
accepted_at: "2026-11-21T17:56:08+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Jusqu'ici, une clé `plugins.<id>.<clé>` du `.escouade.yml` d'équipe (TS-13) n'avait pas de sorte : gardée, dite, sans effet. L'étude du contrat des plugins (ESC-501, §7, choix n° 10 tranché le 2026-11-17) demandait qu'une équipe puisse recommander ou imposer un réglage de plugin, une valeur simple seulement, jamais un secret ni un profil.

## Décision

`plugins.<id>.<clé>` a la sorte valeur dans le fichier d'équipe : `defaults` la recommande (rang 2, sous le `.escouade.yml` du dépôt et la valeur de cette machine), `enforced` l'impose (rang 5, par-dessus), pour les projets de l'équipe seuls. Elle est jugée contre le manifeste du plugin installé là où elle est lue : seulement un réglage simple de portée `project` (string, number, boolean, select, sur plusieurs lignes compris) d'une valeur de son type ; un secret, un profil, une valeur importée, un réglage `repo: false`, un réglage de l'app, une clé inconnue ou d'un mauvais type sont refusés avec leur raison (à la relecture de l'administrateur et dans la page Plugins du projet), gardés sans effet. Un plugin non installé ou inactif : la clé est gardée, dite, sans effet. Dans la page Plugins d'un projet de l'équipe, un réglage imposé est grisé avec le cadenas et « Imposé par l'équipe … » ; ni la valeur de cette machine ni « Écrire dans .escouade.yml » ne passent devant (l'app les refuse) ; un réglage recommandé dit « Recommandé par l'équipe … ». `settings/get { project }` rend la valeur selon ces rangs ; le protocole et le journal ne changent pas. Le poste géré (rang 6) ne verrouille aucun réglage de plugin.

## Conséquences

Le cœur passe au plugin ce que l'équipe règle (`Place::project_team`, `ProjectShared.teamPlugins`), et le relu d'une version de l'administrateur juge ces clés contre les plugins de son poste. Le faux Escouade du SDK et le banc ne simulent pas encore les rangs de l'équipe.
