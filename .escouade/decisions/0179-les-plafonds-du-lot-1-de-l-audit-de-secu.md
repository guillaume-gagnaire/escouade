---
number: 179
id: "les-plafonds-du-lot-1-de-l-audit-de-secu"
title: "Les plafonds du lot 1 de l'audit de sécurité de la 2.2.0"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-470"
pages: ["plugins/harness-en-process","plugins/programmes-js-ts-en-wasm","harness/serveur-mcp"]
tickets: ["ESC-470"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-en-process","added":5,"removed":0},{"page":"plugins/programmes-js-ts-en-wasm","added":2,"removed":0},{"page":"harness/serveur-mcp","added":4,"removed":0}]
accepted_at: "2026-11-15T18:16:39+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit de la 2.2.0 relève des constats faibles qui se corrigent sans choix de produit (SEC-05 en partie, SEC-06, 08, 11, 13, 16, 17). Trois d'entre eux posent une limite qu'un plugin ou un harness rencontre : il faut dire laquelle et pourquoi elle ne gêne pas un usage légitime.

## Décision

Un message d'un harness (encadré par Content-Length ou sur une ligne) fait 64 Mio au plus, une ligne d'en-tête 8 Kio : au-delà, une erreur. Ce qu'un plugin WASM écrit pendant un tour est plafonné à la mémoire de son module (64 Mio par défaut, 256 au plus) : au-delà, l'instance est arrêtée et refaite. À côté du fichier privé d'une interface, un plugin n'écrit que des extensions de réglages (json, jsonc, json5, toml, yaml, yml, md, txt, lock), et tout fichier d'un lancement est jugé là où il atterrit, liens et jonctions suivis. Les serveurs locaux (MCP, hooks) ferment une connexion muette après 2 s et servent 64 connexions à la fois.

## Conséquences

Aucun plugin ni harness connu n'approche ces plafonds (leurs plus gros messages font quelques Mio, une réponse de plugin est faite dans sa mémoire). Un plugin qui écrirait sans fin, ou un harness qui annoncerait une longueur absurde, est arrêté par une erreur qui le dit au lieu d'arrêter l'app. Un client du poste lent à envoyer ses en-têtes (plus de 2 s) verrait sa connexion fermée et la rouvrirait.
