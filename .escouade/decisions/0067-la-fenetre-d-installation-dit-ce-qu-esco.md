---
number: 67
id: "la-fenetre-d-installation-dit-ce-qu-esco"
title: "La fenêtre d'installation dit ce qu'Escouade applique du réseau d'un plugin déclaratif"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-184"
pages: ["store/store-dans-l-app","plugins/vue-d-ensemble"]
tickets: ["ESC-184"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/store-dans-l-app","added":2,"removed":0},{"page":"plugins/vue-d-ensemble","added":26,"removed":0}]
accepted_at: "2026-11-04T18:36:11+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'essai réel du store (ESC-184) a montré, sur Discord, « Escouade ne filtre pas encore son réseau » : c'est vrai d'un programme lancé par une commande, faux d'un plugin déclaratif, dont les webhooks sont envoyés par Escouade seulement vers les hôtes de `permissions.network`, comme la documentation le dit. La même fenêtre disait aussi « 2 webhooks : » sans rien après, et « store.escouade.dev · store.escouade.dev ».

## Décision

Trois textes pour un hôte déclaré : appliqué pour un `run.js`, appliqué pour un plugin déclaratif (« ses webhooks ne partent que vers cet hôte »), seulement déclaré pour un `run.command`. Un webhook, qui n'a pas de nom, est nommé par les faits qui le déclenchent. L'origine n'est pas dite deux fois.

## Conséquences

Aucun changement de ce qu'Escouade laisse faire à un plugin : seuls les textes de la fenêtre changent, en français et en anglais. Un plugin déclaratif peut encore viser une adresse locale de l'hôte déclaré : le texte ne promet donc pas ce que dit celui d'un `run.js`.
