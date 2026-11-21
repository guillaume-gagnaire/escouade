---
number: 245
id: "l-etat-des-docs-passe-de-la-barre-d-etat"
title: "L'état des docs passe de la barre d'état à un badge de l'onglet Docs, et « Synchroniser » remplace « Partager »"
date: "2026-11-18"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-534"
pages: ["docs/docs-dans-le-depot","docs/discussions"]
tickets: ["ESC-534"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"docs/docs-dans-le-depot","added":17,"removed":11},{"page":"docs/discussions","added":1,"removed":1}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis ESC-258, la barre d'état disait « Docs sur la branche X · N changements à partager » et offrait « Partager » (et le conflit), sur l'onglet Docs et ailleurs tant qu'il y avait quelque chose à partager. L'utilisateur a demandé le 2026-11-18, pour la ligne 2.2 : « ne mets plus le status dans la barre de status, mais via un badge sur l'onglet Docs, et un bouton de synchro dans l'onglet docs ». « Partager » ne se montrait que quand il y avait quelque chose à envoyer : rien ne permettait de récupérer seul ce que les autres avaient poussé.

## Décision

Plus rien des docs dans la barre d'état. L'onglet « Docs » porte un badge, comme ceux de Kanban et PR : le nombre de changements des docs à partager, ou un « ! » rouge pour un conflit (l'agent rédacteur en train de le résoudre compris), rien quand tout est partagé ; info-bulle et nom accessible le disent. L'en-tête de chaque espace des docs montre, à gauche de ses boutons, la branche des docs, ce qui reste à partager et « Synchroniser » (« Sync » en anglais), toujours là : avec des docs à partager, c'est le partage d'avant (confirmation si du code partirait) ; sans rien à partager, il récupère seulement (fetch puis fusion, en avance rapide si possible, rien de poussé, pas de confirmation du code) et dit « Docs à jour. » ou « N changements récupérés. ». Pendant qu'il tourne, il le dit et ne se relance pas. Un conflit prend sa place, avec « L'agent résout » et « Annuler ». Le cœur gagne `docs_pull` et le compte `received` dans la réponse du partage.

## Conséquences

La barre d'état est plus courte ; l'état des docs se lit sur l'onglet, à la même place que les autres compteurs. Un bouton toujours présent permet de suivre le travail de l'équipe sans rien avoir à partager. La récupération fusionne la branche distante entière comme le partage le faisait : sur la branche courante, le code des autres vient avec. `share_docs` garde son sens (sa réponse dit en plus `received`). Pour la 2.3 (docs d'un projet d'équipe sur le serveur), le badge et le bouton devront suivre `teamDocsOn`.
