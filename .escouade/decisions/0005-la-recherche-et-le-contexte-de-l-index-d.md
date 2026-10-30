---
number: 5
title: "La recherche et le contexte de l'index du code suivent tokensave (IDX-1)"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-296"
pages: ["docs/index-du-code"]
tickets: ["ESC-296"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/index-du-code","added":26,"removed":9}]
accepted_at: "2026-10-30T03:06:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les sondes d'ESC-294 ont montré l'index moins juste que grep (rappel@10 39 % contre 93 %, 1 cas pratique sur 17). L'utilisateur a demandé le 2026-10-29 de retirer les vecteurs du code et d'implémenter exactement comme tokensave.

## Décision

L'index plein texte (Porter, colonnes pesées 10/5/1/2/3, corps des fonctions), la lecture de la question et le pipeline du contexte sont portés de tokensave au commit 580936b, mêmes constantes. index_context prend keywords et rend le diagnostic Retrieval ; index_search prend le classement de TokenSave::search. Écarts : delta des worktrees, clôture des projets privés, noms index_*, messages fr/en, liens par nom jusqu'à IDX-2, branches/boucles comptées par les mots. Licence MIT dans THIRD-PARTY-NOTICES.md et CREDITS.md.

## Conséquences

La forme de l'index change (schéma 2) : chaque projet indexé est relu une fois, sans rien demander. index_context rend par défaut 20 symboles (100 au plus) dont trois points d'entrée. Les chiffres avant/après sont dans docs/CODEINDEX-PROBES.md.
