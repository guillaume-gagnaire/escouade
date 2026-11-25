---
number: null
id: "voix-et-modeles-epingles-a-une-revision"
title: "Voix et modèles épinglés à une révision"
date: "2026-11-25"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-664"
pages: ["voix/dictee","voix/conversation-vocale","docs/recherche-par-le-sens"]
tickets: ["ESC-664"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"voix/dictee","added":5,"removed":2},{"page":"voix/conversation-vocale","added":4,"removed":0},{"page":"docs/recherche-par-le-sens","added":1,"removed":1}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le 2026-11-25, un utilisateur en 2.2.10 n'a pas pu télécharger la voix de la conversation : « 6369 bytes and more, 4875 expected ». Les voix Piper, les modèles de whisper.cpp et le modèle des vecteurs venaient de `resolve/main` de leurs dépôts Hugging Face. Ils étaient vérifiés contre les tailles et les SHA-256 du catalogue livré avec l'app. Or `rhasspy/piper-voices` réentraîne ses voix sur place depuis le 2026-11-09 (siwis, alba, cori…), si bien que toute voix changée en amont échouait à se télécharger. Le message d'échec, en anglais technique, ne disait pas que le problème venait d'Escouade. Le même jour, un autre utilisateur croyait la dictée cassée : il n'avait pas compris qu'il faut recliquer sur le micro pour finir.

## Décision

Chaque fichier téléchargé de Hugging Face vient d'un commit fixé de son dépôt, jamais de `main`. Les voix ont un champ `revision` dans `voices.json`, un commit par dépôt, que tient `REVISIONS` dans `scripts/piper-voices.mjs` : pour `rhasspy/piper-voices`, `e7229f7d` (2026-11-02), le dernier où les 36 voix correspondent. whisper.cpp a `MODELS_REVISION` (`5359861c`) et multilingual-e5-base `MODEL_REVISION` (`1ec92430`). Changer une révision est une mise à jour du catalogue, faite exprès et revérifiée par `node scripts/piper-voices-check.mjs` et par le test ignoré `the_pinned_files_are_the_catalog_s`. Quand un fichier n'a pas la taille attendue (`FileChanged`) ou pas la SHA-256 attendue, le message le dit en clair, en français et en anglais : le fichier publié a changé, et c'est à Escouade de se mettre à jour. Pendant la dictée, l'indication devient « Reclique sur le micro ou <raccourci> pour écrire ce que tu as dit · Échap pour annuler ».

## Conséquences

Une voix ou un modèle réentraîné en amont ne casse plus un téléchargement. Les améliorations des voix n'arrivent qu'à une mise à jour du catalogue, faite exprès. Un dépôt qui supprimerait l'historique casserait la révision fixée : le message dirait alors de mettre Escouade à jour. Un dépôt de voix sans révision dans `REVISIONS` est refusé par le script de la liste.
