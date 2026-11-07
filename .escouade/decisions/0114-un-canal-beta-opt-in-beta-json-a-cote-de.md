---
number: 114
id: "un-canal-beta-opt-in-beta-json-a-cote-de"
title: "Un canal bêta opt-in : beta.json à côté de latest.json, les bêtas ignorées par défaut"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-374"
pages: ["introduction/livraison"]
tickets: ["ESC-374"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/livraison","added":85,"removed":0}]
accepted_at: "2026-11-07T19:48:21+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'utilisateur veut pouvoir déployer une majeure en bêta, la faire tester aux volontaires, sans gêner tout le monde (demande du 2026-11-07, patch 2.1). Les installations jusqu'à la 2.1.3 n'ont aucun filtre de version et lisent le latest.json de la dernière release : y mettre une bêta la leur installerait.

## Décision

Une bêta se nomme X.Y.Z-beta.N (semver). Réglage « Recevoir les versions bêta », décoché par défaut, enregistré aussitôt ; le cocher lance une vérification. Décoché, toute version suffixée est ignorée (comparateur de l'updater : plus récente et sans préversion) et l'app ne lit que latest.json ; une app sur une bêta reste dessus jusqu'à la stable qui la suit. Coché, l'app lit beta.json puis latest.json s'il manque. latest.json ne contient jamais une bêta ; beta.json, toujours dans la dernière release stable, contient la plus récente des versions, stable ou bêta, tenu par le workflow de release (scripts/beta-channel.mjs) et contrôlé après publication. Une préversion sans section de changelog prend « Non publié » / « Unreleased ». L'app dit « (bêta) » après la version d'une bêta.

## Conséquences

Les anciennes versions ne voient jamais de bêta. Chaque release stable porte un beta.json ; une bêta remplace celui de la release « latest ». Une bêta n'est pas annoncée sur Discord. Le .dmg, la notarisation, l'AppImage et le .deb d'une bêta ne sont vérifiés qu'au premier vrai tag de bêta.
