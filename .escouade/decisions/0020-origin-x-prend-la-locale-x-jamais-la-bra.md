---
number: 20
title: "origin/x prend la locale x, jamais la branche d'un agent qui la suit"
date: "2026-10-31"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-315"
pages: ["git-fichiers/git-fichiers"]
tickets: ["ESC-315"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/git-fichiers","added":3,"removed":1}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Avec « Branche de départ » origin/master, local_of prenait la première locale (ordre alphabétique) dont l'amont était origin/master : la branche escouade/… d'un autre agent, créée depuis une base distante et donc suivant origin/master par défaut.

## Décision

Pour origin/x : la locale x si elle la suit ; sinon une locale qui la suit et n'est ni escouade/* ni tenue par un worktree lié ; sinon celle que switch --track créerait. Les branches d'agents et de tickets créées depuis une base distante le sont avec --no-track. Les branches d'agents existantes gardent leur amont (aucune branche n'est modifiée) : elles ne sont simplement plus choisies.

## Conséquences

Un agent part du master local avancé en fast-forward, sans avertissement. Le préfixe d'agent personnalisé n'est reconnu que s'il est tenu par un worktree lié.
