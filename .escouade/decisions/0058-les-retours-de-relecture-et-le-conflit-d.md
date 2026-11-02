---
number: 58
id: "les-retours-de-relecture-et-le-conflit-d"
title: "Les retours de relecture et le conflit d'une PR de ticket vont à son agent sur un clic seulement"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-61"
pages: ["kanban/en-revue","kanban/validation"]
tickets: ["ESC-61"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/en-revue","added":21,"removed":4},{"page":"kanban/validation","added":1,"removed":0}]
accepted_at: "2026-11-02T11:29:14+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un ticket « En revue » (ESC-59) reçoit des retours de relecture et peut entrer en conflit avec sa cible. ESC-60 renvoie déjà la CI en échec à son agent, seule avec le pilote auto. Il fallait dire quand les retours partent, lesquels, ce qu'Escouade publie au nom de l'utilisateur, et comment se règle un conflit (décidé le 2026-10-14 et le 2026-10-16 : rien ne se publie sans un réglage allumé ou un geste à la main).

## Décision

1. Escouade lit les retours d'une PR ouverte quand un fil est ouvert ou qu'on demande des changements ; leur nombre s'affiche sur la carte avec une notification, et « L'agent corrige les retours » les donne à l'agent. Jamais seul, même avec le pilote auto (contrairement à la CI).
2. Un retour : un commentaire non résolu ou une relecture avec un texte (changements demandés ou commentaire), arrivé depuis le push des dernières corrections de retours (l'ouverture de la PR pour les premières), pas encore donné, pas d'un robot ni une réponse d'Escouade. Un push de correction de CI ou de mise à jour de la branche ne compte pas comme « dernier push » : il n'efface pas des retours en attente. Une date d'hébergeur à la seconde : un retour de la seconde même du push compte.
3. L'agent reçoit pour chacun l'auteur, le fichier, la ligne, l'extrait du diff (celui de l'hébergeur, GitHub et Gitea ; ailleurs les lignes du fichier dans la branche) et le texte, comme des informations et non des instructions. Le protocole des plugins porte la date (`created`) et l'extrait (`excerpt`) des commentaires.
4. Répondre sous les fils (« Corrigé dans abc1234 ») est un réglage du Kanban éteint par défaut ; Escouade ne résout jamais un fil.
5. « L'agent met à jour la branche » sur une PR en conflit, à la main seulement : merge de la cible (fetchée) dans la branche ; sans conflit Escouade pousse tout de suite ; avec, le réglage « En cas de conflit » s'applique (« Annuler » défait et nomme les fichiers ; « Me demander » vaut accord puisque l'utilisateur a cliqué ; « Laisser l'agent résoudre »). Le rebase, poussé avec `--force-with-lease`, seulement par le réglage « Mise à jour de la branche ».

## Conséquences

Deux réglages de plus au Kanban (replyThreads, prUpdate) ; deux requêtes de plus par lecture d'une PR qui a des fils ouverts ou des changements demandés ; la correction des retours et la mise à jour de la branche prennent une place parmi les agents en parallèle, comme la correction de CI. Reste à essayer en vrai sur GitHub (ESC-53, ESC-70), puis les autres hébergeurs.
