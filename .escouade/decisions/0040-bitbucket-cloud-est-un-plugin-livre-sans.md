---
number: 40
id: "bitbucket-cloud-est-un-plugin-livre-sans"
title: "Bitbucket Cloud est un plugin livré, sans auto-merge, écrit d'après la documentation"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-57"
pages: ["git-fichiers/hebergeurs-de-code","plugins/livres-avec-escouade"]
tickets: ["ESC-57"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/hebergeurs-de-code","added":10,"removed":1},{"page":"plugins/livres-avec-escouade","added":1,"removed":0}]
accepted_at: "2026-11-01T21:35:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-57 : le client Bitbucket Cloud de l'interface des hébergeurs. Depuis la décision sur GitHub en plugin (ESC-217), aucun client d'hébergeur nouveau n'entre dans le cœur. L'utilisateur a décidé le 2026-10-16 que les essais réels se font sur GitHub seulement. L'API de Bitbucket Cloud n'a pas d'auto-merge et ne donne pas ce qui reste de la limite horaire.

## Décision

Le client est le plugin de première main dev.escouade.bitbucket (crate sur escouade-plugin-kit, programme escouade-plugins bitbucket), compte par e-mail et jeton d'API Atlassian. Il couvre PR et brouillons, merge, statuts de commit, pipelines et journaux d'étapes, commentaires, fils et relectures, déploiements. La capacité auto_merge est déclarée absente et l'appel dit « non supporté » : Escouade attend la CI et merge elle-même. Le client compte ses propres appels sur l'heure (1000 par défaut), croit X-RateLimit-Limit, ralentit près de la limite et attend un 429 une fois. Il est écrit d'après la documentation de l'API 2.0 et testé sur un faux serveur aux réponses écrites à la forme de la documentation (aucun compte pour enregistrer) ; les essais réels restent à faire, listés dans docs/TODO-2.2.md.

## Conséquences

Aucun code de client Bitbucket dans le cœur (il ne garde que la déclaration du fournisseur). Le kit change d'une ligne (errors::said lit le message d'erreur de Bitbucket). Sans relance de jobs (aucun appel connu) ni étiquettes. Les fixtures devront être remplacées par de vrais enregistrements le jour d'un essai réel.</consequences>
<parameter name="origin">ticket
