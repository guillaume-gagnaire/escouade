---
number: 19
title: "Un harness absent : l'agent est gardé, ne démarre pas, aucun n'est créé dessus"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-209"
pages: ["harness/harness"]
tickets: ["ESC-209"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/harness","added":6,"removed":0}]
accepted_at: "2026-10-31T10:03:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-3 (ESC-209) ouvre l'id des harness pour que les plugins de harness puissent en ajouter (docs/PLUGINS.md §4.7). Les données peuvent alors nommer un harness que cette version n'a pas : une version plus récente, un plugin retiré ou désactivé. Avant, l'id était une énumération fermée et un id inconnu ne se lisait pas.

## Décision

L'id d'un harness est gardé et relu tel quel, sérialisé à l'identique (rien à migrer). Un agent dont le harness manque reste avec sa conversation, marqué « <id> (harness absent) », sans aucune de ses fonctions ; il ne démarre pas (raison « Harness absent : … ») et aucun agent n'est créé sur ce harness. Côté code, hors de engine/ et agent/, le cœur ne demande plus quel harness tourne mais ce que fait le moteur (méthodes d'Engine, EngineCaps) ; seul Claude Code reste nommé comme harness par défaut et pour sa mise à jour automatique (choix de l'utilisateur, « tout derrière le trait », 2026-10-30).

## Conséquences

Les plugins de harness (INT-9, INT-10) s'enregistrent dans le registre (registry::register) sans toucher au cœur. Désinstaller ou désactiver l'un d'eux ne perd aucun agent. La passation d'un agent d'un harness absent vers un autre n'est pas encore essayée.
