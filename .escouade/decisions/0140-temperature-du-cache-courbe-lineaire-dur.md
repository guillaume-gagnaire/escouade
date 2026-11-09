---
number: 140
id: "temperature-du-cache-courbe-lineaire-dur"
title: "Température du cache : courbe linéaire, durée de vie par fournisseur, rien d'inventé"
date: "2026-11-09"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-398"
pages: ["conversation/temperature-du-cache","fenetre/barre-de-statut","conversation/conversation","fenetre/barre-laterale"]
tickets: ["ESC-398"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"conversation/temperature-du-cache","added":25,"removed":0},{"page":"fenetre/barre-de-statut","added":2,"removed":0},{"page":"conversation/conversation","added":2,"removed":0},{"page":"fenetre/barre-laterale","added":2,"removed":0}]
accepted_at: "2026-11-09T17:25:04+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

docs/FEATURES.md §3 (2026-10-28) laisse au constructeur la courbe de la température, ses couleurs, et la vérification harness par harness de ce qui est lu. Les harness autres que Claude Code ne passent au cœur que les tokens de leur tour, lus et écrits en cache en un seul chiffre (TurnUsage.cache), et aucun ne dit la durée de vie de son cache. Changer le protocole des plugins pour séparer lus et écrits demanderait une nouvelle version publiée de escouade-plugin-protocol sur crates.io et une montée de version de chaque plugin.

## Décision

1) La température baisse en ligne droite de 100 °C (dernière requête qui a lu ou écrit le cache) à 0 °C (expiration), arrondie à l'entier supérieur ; tons : rouge-orangé ≥ 67, ambre ≥ 34, bleu en dessous, gris pour « — °C » (jetons --heat-hot/-mid/-cool, AA dans les deux thèmes). 2) Durée de vie : Claude Code d'après l'usage de chaque requête (1 h quand elle écrit en ephemeral_1h, 5 min sinon) ; les autres d'après le fournisseur du modèle : 5 min chez Anthropic et chez OpenAI (la plus courte des « 5 à 10 min » annoncées), inconnue ailleurs (« — °C »). 3) Lus et écrits séparés seulement quand c'est sûr (Claude Code, et OpenAI où rien ne s'écrit) ; sinon une seule ligne « Lus ou mis en cache » et pas d'économie affichée. 4) Cassé (0 °C) quand le modèle, le compte ou la consigne ajoutée par Escouade changent, à une compaction ou à une nouvelle conversation. 5) Aucun plugin ni le protocole ne changent pour ce ticket : tout est lu par le cœur depuis ce qu'il reçoit déjà. 6) Toujours affichée tant que « Coût » n'a pas d'interrupteur (aucun réglage actuel ne cache le coût).

## Conséquences

Pas de version de plugin à publier. Copilot CLI et OpenCode sur un modèle d'Anthropic montrent la température mais pas l'économie, faute de séparer lus et écrits ; un ticket à venir peut ajouter cacheRead/cacheWrite à TurnUsage (protocole publié, plugins montés). Codex avec une rétention étendue (24 h) serait sous-estimé à 5 min : le chiffre refroidit plus tôt que le cache, jamais l'inverse.
