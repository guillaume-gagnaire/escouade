---
number: 157
id: "azure-devops-l-adresse-d-une-pr-est-fait"
title: "Azure DevOps : l'adresse d'une PR est faite quand Azure ne donne pas webUrl"
date: "2026-11-13"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-437"
pages: ["git-fichiers/pull-requests"]
tickets: ["ESC-437"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":3,"removed":0}]
accepted_at: "2026-11-13T12:16:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Signalé sur la 2.2 bêta : le plugin Azure DevOps ne donnait pas de lien vers les PR. Il prenait l'adresse de la PR sous repository.webUrl, que les listes de PR d'Azure (et souvent la lecture d'une PR) ne donnent pas : l'adresse restait vide et l'app n'ouvrait rien.

## Décision

Le plugin garde webUrl quand Azure la donne ; sinon il fait l'adresse depuis l'adresse du compte (cloud ou collection d'un Server), le projet et le dépôt que dit la PR (sinon ceux du dépôt du projet), chacun encodé : {org}/{projet}/_git/{dépôt}/pullrequest/{n}. Elle vient toujours de l'adresse du compte, pas de celle de l'API.

## Conséquences

Chaque PR Azure DevOps a son lien (plugin 2.2.4, à publier). Le faux Azure des e2e ne donne plus webUrl, comme le vrai, et l'e2e vérifie l'adresse faite. Les autres liens du plugin (builds, éléments de travail) étaient déjà faits ou avaient un repli.
