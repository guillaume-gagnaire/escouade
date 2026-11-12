---
number: null
id: "les-economies-du-cache-dans-la-barre-d-e"
title: "Les économies du cache dans la barre d'état, derrière un réglage éteint par défaut"
date: "2026-11-12"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-421"
pages: ["fenetre/barre-de-statut","conversation/temperature-du-cache"]
tickets: ["ESC-421"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/barre-de-statut","added":4,"removed":2},{"page":"conversation/temperature-du-cache","added":2,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-398 a mis « Cache aujourd'hui : ≈ X $ économisés » dans la barre d'état dès que le cache avait économisé quelque chose. Le 2026-11-12, l'utilisateur ne veut plus le voir par défaut : « le mettre derrière un réglage, et par défaut, ne pas l'afficher ».

## Décision

Un réglage booléen de l'app, « Économies du cache dans la barre d'état » (`Settings.cacheSavingsInBar`), dans Réglages › Comptes et harness › Avancé, à côté de « Projection des quotas ». Il est éteint par défaut, y compris pour un fichier de réglages qui ne l'a pas (une installation existante). La barre d'état n'écrit l'économie du jour que s'il est allumé. La température du cache sur les cartes et dans l'en-tête, et « Économisé sur ce tour », ne changent pas.

## Conséquences

L'élément disparaît chez tout le monde à la mise à jour ; qui le veut le rallume en un geste, et la recherche des réglages le trouve. Un champ de plus dans Settings (Rust et TS), lu à false quand il manque.
