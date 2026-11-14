---
number: 162
id: "ce-qu-un-harness-tiers-dit-sans-le-lance"
title: "Ce qu'un harness tiers dit sans le lancer est montré et jugé (SEC-30)"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-443"
pages: ["plugins/harness-acp-decrits"]
tickets: ["ESC-443","ESC-418"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":6,"removed":0}]
accepted_at: "2026-11-14T10:36:44+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit sur la 2.2.0-beta.15 (ESC-418, SEC-30, test W-32) a montré que deux champs du manifeste d'un harness décrit par un tiers échappaient aux contrôles : `install` et `updateCommand`, que l'app affichait comme ses propres commandes (accueil avec bouton Copier, Réglages › Comptes et harness, message « trop ancien »), et `acp.removeEnv`, qui pouvait retirer du lancement le proxy et le fichier d'autorités posés par le cœur. L'utilisateur a choisi de corriger avant la 2.2 finale.

## Décision

`acp.removeEnv` d'un tiers est jugé par les familles de `env` : refusé au manifeste (code `variable`), et l'hôte, sur chaque lancement d'un tiers, garde ce qu'il pose lui-même (proxy, autorités, variables du compte, adresse des hooks) et toute variable d'une famille refusée. `CODEX_CA_CERTIFICATE` rejoint ces familles. La fenêtre d'installation et de mise à jour montre les commandes d'installation et de mise à jour (`harnessInstall:`, `harnessUpdate:`, « écrites par le plugin ») et les variables retirées (`harnessUnset:`), non révocables. Pour un tiers, partout où Escouade affiche ces commandes, elle dit qu'elles viennent du plugin, nommé par son identifiant ; la première main garde l'affichage d'avant.

## Conséquences

Un plugin tiers qui retirait le proxy ou les autorités est refusé à l'installation et au store. Les plugins de première main voient apparaître leurs commandes d'installation dans leur fenêtre d'installation, sans redemande d'accord (les permissions installées sont recalculées par le même code). Une mise à jour qui change une de ces commandes redemande l'accord.
