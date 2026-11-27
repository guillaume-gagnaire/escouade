---
number: null
id: "la-lecture-d-un-depot-qui-echoue-n-est-j"
title: "La lecture d'un dépôt qui échoue n'est jamais « aucun commit »"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-687"
pages: ["projets/nouveau-projet"]
tickets: ["ESC-687"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/nouveau-projet","added":4,"removed":3}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Sur un poste chargé, l'assistant « Nouveau projet » disait « Le dépôt n'a aucun commit » d'un dépôt qui en avait un, et proposait « Faire un premier commit ». Mesuré (ESC-687) : git ne répondait jamais faux, mais chaque git mettait de 0,5 à 21 s à démarrer (lancement des process Windows sous charge). L'assistant prenait un dossier pas encore vérifié pour un dossier sans commit, et `has_commit` rendait « non » pour toute erreur de git (une erreur de `rev-parse --show-toplevel` faisait même `git init`).

## Décision

has_commit dit trois choses : oui ; non seulement quand git répond que HEAD ne désigne aucun commit (ou que le dossier n'est dans aucun dépôt, aucun `.git` au-dessus) ; une erreur sinon. La racine du dépôt est lue de même (repo_root). L'assistant dit l'erreur, « Lecture du dépôt impossible : … », avec « Réessayer », et « Lecture du dépôt… » tant que le dossier n'est pas vérifié. Aucun commit, aucun `git init` sur une erreur ; la branche des docs, son partage et les worktrees de plusieurs dépôts renvoient l'erreur au lieu de « aucun commit ». Les lectures git de l'assistant partent ensemble, et un git de plus de 5 s est noté dans app.log.

## Conséquences

Un dépôt que git ne lit pas bloque l'assistant avec son erreur, au lieu d'y créer un commit. Sur un poste chargé, l'assistant répond en à peu près la durée d'un seul git au lieu de leur somme. La lenteur des lancements elle-même dépend du poste (charge, antivirus) ; app.log permet de la voir.
