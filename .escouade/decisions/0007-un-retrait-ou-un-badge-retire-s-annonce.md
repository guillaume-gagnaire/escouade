---
number: 7
title: "Un retrait ou un badge retiré s'annonce toujours en public ; --quiet ne coupe que l'e-mail à l'auteur"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-302"
pages: ["store/revue-et-signature","store/annonces-discord"]
tickets: ["ESC-302","ESC-295"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/revue-et-signature","added":4,"removed":2},{"page":"store/annonces-discord","added":3,"removed":2}]
accepted_at: "2026-10-30T03:55:59+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

À la validation de STORE-12 (ESC-295), le code n'annonçait au salon public que les entrées `disable`, et seulement sans `--quiet` : un `unverify` (« Badge retiré ») n'était jamais annoncé, et un retrait `--quiet` ne l'était pas non plus, alors que la spec disait que les retraits partent en public et que `--quiet` ne fait que ne pas prévenir l'auteur.

## Décision

Chaque nouvelle entrée d'une liste de révocation part dans le salon des admins et dans le salon public fr et en, avec sa raison publique : « Plugin retiré » / « Plugin taken off » pour `disable`, « Badge retiré » / « Badge withdrawn » pour `unverify`. `--quiet` (notify: false) ne change que l'e-mail à l'auteur ; le salon des admins dit « Auteur : prévenu » ou « non prévenu (--quiet) ».

## Conséquences

Le public apprend aussi les badges retirés et les retraits discrets, ce que la liste de révocation, publique (`/api/v1/revocations.json`, copie sur escouade.dev), disait déjà. Si un jour un retrait doit rester hors du salon public, il faudra une option à part (pas `--quiet`), à décider d'abord.
