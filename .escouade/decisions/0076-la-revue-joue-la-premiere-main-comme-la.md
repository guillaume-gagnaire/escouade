---
number: 76
id: "la-revue-joue-la-premiere-main-comme-la"
title: "La revue joue la première main comme la version en développement"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-343"
pages: ["store/revue-et-signature"]
tickets: ["ESC-343"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/revue-et-signature","added":6,"removed":0}]
accepted_at: "2026-11-05T01:54:43+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les plugins de la première main sont en `escouade.min` 2.2.0 et doivent être relus et signés avant que la 2.2.0 existe (le chef publie d'abord, la release ensuite). Le banc de la revue locale les refusait pour leur version, donc aucun ne pouvait être signé.

## Décision

`escouade-plugin validate`, `test` et `dev` prennent `--escouade <version>`, qui joue le plugin comme cette version : contrôle de la plage et `initialize`. `dev` contrôle désormais la plage. La revue locale passe `--escouade` pour la première main seulement, avec `NEXT` de `website/app/data/site.ts` (déjà commité, et un test échoue dès que cette version sort). Un tiers est toujours joué comme l'outil du poste. Le rapport dit la version jouée. La revue récupère de la CI de main l'`escouade-js` que nomme `version.json`, empreinte vérifiée ; le `cargo build` reste à faire par le relecteur.

## Conséquences

La recompilation et `sign` ne changent pas. `plugins/seed.json` n'est pas utilisé : il est prévu pour l'amorce (A8). Le banc doit venir d'un `escouade-plugin` qui connaît `--escouade` (la 2.1.2 publiée ne le connaît pas). Le JSON du banc gagne le champ `escouade`.
