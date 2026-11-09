---
number: 110
id: "la-documentation-d-un-plugin-de-premiere"
title: "La documentation d'un plugin de première main : un README par langue et docs/fr, docs/en"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-370"
pages: ["plugins/livres-avec-escouade"]
tickets: ["ESC-370"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":2,"removed":0}]
accepted_at: "2026-11-07T11:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur a demandé le 2026-11-07 une documentation sur le store pour chaque plugin. Le test du registre des plugins livrés exigeait un README.md dans chaque dossier, ce qui empêchait de passer aux README par langue que le store accepte.

## Décision

Un plugin de plugins/ a README.fr.md et README.en.md (sa page, sans rien pour les contributeurs) à la place de README.md, et ses pages dans docs/fr et docs/en (installer, utiliser, permissions, dépannage, développer, nommées pour leur ordre), déclarées par store.docs. Le test du registre accepte README.md ou les deux README par langue.

## Conséquences

Les familles passent une à une (ESC-370 : isola, Discord, Slack, Contraste élevé, Règles de l'équipe). Aucune version ne monte : la documentation part avec la prochaine publication de chaque plugin.
