---
number: 55
id: "la-ci-en-echec-d-un-ticket-en-revue-repa"
title: "La CI en échec d'un ticket en revue repart chez son agent"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-60"
pages: ["kanban/en-revue"]
tickets: ["ESC-60"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/en-revue","added":19,"removed":4}]
accepted_at: "2026-11-02T06:57:01+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Avec ESC-59, un ticket validé par « Valider + PR » attend « En revue » jusqu'au merge, son agent archivé et son worktree supprimé. Quand la CI de sa PR échoue, il fallait reprendre la main soi-même. Le 2026-10-14, il a été décidé qu'avec le pilote auto la correction part seule, dans la limite des boucles du ticket, et le 2026-10-16 que le worktree est recréé depuis la branche quand l'agent doit corriger.

## Décision

Une CI en échec sur la PR d'un ticket « En revue » repart chez l'agent du ticket : restauré, son worktree recréé depuis la branche (reprise du dépôt distant si elle a été supprimée), il reçoit les jobs en échec et la fin de leurs journaux. Le ticket reste « En revue » pendant la correction, avec « L'agent corrige la CI… », et y prend une place parmi les tickets en parallèle. À la fin du tour, Escouade pousse la branche, relit la PR et archive l'agent de nouveau. Sans le pilote auto, seul le bouton « L'agent corrige » le fait ; avec lui, ça part seul quand une place est libre, avant les tickets à faire, tant que les corrections n'ont pas atteint les boucles du ticket, et jamais deux fois pour le même échec (il attend que la CI tourne de nouveau). Un check sans journal (CI tierce) est dit sur la carte.

## Conséquences

Chaque correction compte une boucle du ticket. Escouade pousse elle-même la branche de la PR après une correction demandée (bouton ou pilote auto), comme l'agent en a reçu la consigne ; rien d'autre n'est publié sur l'hébergeur. Ce qui empêche d'aller au bout (rien de commité, changements non commités, push refusé, tour interrompu, app fermée) est dit sur la carte et laisse l'agent restauré. Un ticket dont l'agent a été supprimé ne peut pas faire corriger sa CI.
