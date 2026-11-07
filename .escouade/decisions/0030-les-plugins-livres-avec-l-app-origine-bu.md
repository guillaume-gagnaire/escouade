---
number: 30
id: "les-plugins-livres-avec-l-app-origine-bu"
title: "Les plugins livrés avec l'app : origine builtin, registre embarqué, l'identifiant de l'app l'emporte"
date: "2026-10-31"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-210"
pages: ["plugins/livres-avec-escouade"]
tickets: ["ESC-210"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":15,"removed":0}]
accepted_at: "2026-11-01T10:45:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-4 (ESC-210) : tout l'externe passe en plugins en process, y compris ceux de première main. Il faut dire ce qu'est un plugin livré avec l'app, comment il se distingue de ceux du store (ESC-314) et comment se départagent un plugin livré et le même identifiant venu d'ailleurs.

## Décision

Un plugin livré avec l'app a l'origine `builtin`, vient d'un registre embarqué (`plugins/builtin/<id>/`), a la version de l'app, ne se met pas à jour ni ne se révoque à part, se coupe pour l'app ou un projet mais ne se désinstalle pas. Son programme est `escouade-plugins <nom>`, signé avec l'app, que le cœur vérifie (chemin dans le dossier de l'app, signature) avant de le lancer ; seule l'origine `builtin` peut lancer les programmes `{escouade}/…`. Si le même identifiant vient du store, d'un dossier ou d'un dépôt, celui de l'app l'emporte : l'installation est refusée et une copie plus ancienne cède la place, ses choix gardés. Les plugins `plugins/<id>/` du store restent des plugins du store.

## Conséquences

Chaque plugin livré a son dossier lisible seul (manifeste, README) vérifié par un test. Le premier est `dev.escouade.minimal`. Les sorties du cœur (INT-5 et suivants) s'ajoutent au registre. Les vérifications Authenticode et notarisation de `escouade-plugins` ne se voient qu'au Release.</consequences>
<parameter name="origin">ticket
