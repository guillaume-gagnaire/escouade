---
number: 32
id: "net-fetch-juge-les-adresses-resolues-a-l"
title: "net/fetch juge les adresses résolues à la connexion, sans proxy, et escouade-js est embarqué dans l'exécutable"
date: "2026-11-01"
state: "superseded"
origin: "ticket"
origin_ref: "ESC-320"
pages: ["plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-320"]
documents: []
supersedes: []
superseded_by: 78
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":13,"removed":11}]
accepted_at: "2026-11-01T14:50:40+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

WASM-1 (ESC-320) construit dans l'app le runtime des plugins `run.js` conçu par ESC-319 (décision 29). Deux points restaient ouverts. 1) Le prototype jugeait seulement le nom d'hôte de l'URL : un hôte déclaré (ou un sous-domaine) dont le DNS répond une adresse locale menait le `fetch` aux services de la machine ou du réseau local (revue du chef, 2026-11-01). 2) « escouade-js livré avec l'app » ne disait pas comment : une ressource du paquet, ou l'exécutable.

## Décision

- `net/fetch` juge le nom (https, un hôte de `permissions.network` ou un sous-domaine, jamais une IP écrite), puis les adresses que ce nom résout, dans le résolveur que la connexion emploie elle-même : toute adresse de bouclage, privée, de lien local, non spécifiée, de multidiffusion ou de diffusion (IPv4, IPv6, IPv4 mappée ou NAT64) est écartée, et s'il n'en reste aucune la connexion est refusée (`-32003`, la raison au journal). `net/fetch` ne passe donc pas par le proxy de Réglages › Réseau, qui résoudrait le nom là où l'adresse n'est pas jugée.
- `escouade-js` est construit par un job de la CI et de la release (`scripts/escouade-js.sh`, son SHA-256 au résumé du job) et embarqué dans l'exécutable par `build.rs` ; une app construite sans lui ne lance aucun plugin `run.js` et dit pourquoi. La Javy épinglée est téléchargée au premier besoin et vérifiée par son SHA-256.
- Le module est compilé et précompilé à l'installation, gardé dans `<données>/cache/plugins-wasm/` (le module par la clé de ce qui l'a fait, le précompilé par la version de l'app), rechargé de là, refait après une mise à jour.

## Conséquences

Un plugin `run.js` ne joint ni la machine ni le réseau local, même par un DNS complaisant. Sur un poste dont la sortie passe obligatoirement par un proxy, son `fetch` échoue (une limite connue, à revoir si un utilisateur en a besoin : il faudrait un proxy qui juge lui-même les adresses). L'exécutable grossit d'environ 15 Mo (docs/PLUGINS.md §5.3), dont 1,8 Mo d'`escouade-js`. Les jobs qui construisent l'app attendent le job `escouade-js`.
