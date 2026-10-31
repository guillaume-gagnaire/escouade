---
number: 20
title: "Le choix automatique du compte juge chaque fenêtre selon sa longueur : la 5 h est une pause courte, la semaine se juge au rythme"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "PR #20"
pages: ["harness/comptes","fenetre/barre-de-statut","kanban/pilote-auto"]
tickets: []
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":32,"removed":16},{"page":"fenetre/barre-de-statut","added":4,"removed":2},{"page":"kanban/pilote-auto","added":1,"removed":0}]
accepted_at: "2026-10-31T13:08:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le mode « Automatique » doit toujours choisir le compte le plus pertinent. Jusque-là, toutes les fenêtres du quota d'un compte comptaient pareil : un compte était « au-delà du seuil » dès que n'importe laquelle (5 h, semaine, 30 jours, crédits du mois) atteignait le seuil en pourcentage, et le choix prenait le premier compte actif de la liste qui n'était pas au-delà. On restait sur un compte presque fini en semaine alors qu'un autre était frais, et une semaine qui allait se remettre à zéro avec beaucoup de marge était quittée alors qu'on tapait dans un autre compte. La 5 h (quelques heures) et la semaine (tout le reste de la semaine) n'ont pas le même sens. La projection à la remise à zéro n'existait que dans la fenêtre (projectedPct).

## Décision

L'ordre des comptes reste la priorité (demandé, puis « Compte préféré » du projet, puis le premier compte « bon » dans l'ordre) ; ce qui change est ce qu'est un compte bon, fenêtre par fenêtre, par la longueur de la fenêtre et non par le nom du harness. Une fenêtre courte (24 h ou moins : la 5 h) est une pause courte : au seuil, le compte est écarté jusqu'à la remise à zéro de cette fenêtre seulement, puis il redevient un candidat comme les autres. Une fenêtre longue (plus de 24 h : semaine, 30 jours ; crédits du mois de Copilot CLI, pris comme une fenêtre d'un mois) est jugée au rythme, avec la projection de la barre d'état : à 100 % écartée jusqu'à sa remise à zéro ; projection à 100 % ou moins, bonne même au-dessus du seuil en % ; au-delà de 100 %, le compte passe derrière les autres même sous le seuil ; sans projection (première journée, remise à zéro inconnue), le seuil en % décide ; à 95 % ou plus (NEAR_LIMIT), quel que soit son rythme, le compte passe derrière les autres (ajouté avec l'utilisateur à la revue de la PR #20 : les nouveaux agents ne partent pas au bord de la limite tant qu'un autre compte peut les prendre). Aucun compte bon : d'abord le premier dont seul le rythme est mauvais, sinon le premier actif (le pilote auto fait alors sa pause). « Changer de compte à » reste un seul réglage, sans second seuil : il déclenche le départ d'un agent déjà lancé, mais le choix des nouveaux agents et des tickets suit toujours la règle, « À la limite » compris. Le cœur Rust a un seul prédicat (pace::judge), utilisé par le compte en cours, les nouveaux agents et tickets, la reprise sur un autre compte, la passation et le départ d'un agent lancé ; la projection y est portée avec les mêmes chiffres que la fenêtre (tests/fixtures/quota-pace.json, vérifié des deux côtés).

## Conséquences

Par défaut, un agent déjà lancé quitte son compte à la fin de son tour quand une fenêtre courte atteint le seuil, ou quand une fenêtre longue l'atteint et que son rythme n'est pas bon (projection au-delà de 100 % ou pas de projection) : un agent sur une semaine à 88 % qui se remet à zéro dans 3 h ne bouge pas. Il va sur le compte que donne la règle de choix, autre que le sien, que l'agent ne quitterait pas aussitôt (pas de va-et-vient) ; un compte dont seul le rythme est mauvais, sous le seuil, convient à défaut d'un compte bon. « Pause au-delà du quota », la pause du pilote auto, « Reprendre maintenant » et les agents qui attendent leur remise à zéro ne changent pas : le plafond est celui de l'utilisateur, jugé en pourcentage. Avec un plafond sous 100 %, un compte peut donc être bon pour le choix et pourtant retenu par le pilote auto (il n'est alors pas choisi, et la pause ne tombe que si tous les comptes sont retenus) ; ce n'est pas corrigé. Les messages disent la vraie raison quand c'est le rythme qui écarte un compte sous le seuil (toast, note de conversation, notification, « aucun compte utilisable »), et le panneau des quotas marque « rythme trop rapide pour sa fenêtre » à côté de « au-delà du seuil… ». Le jugement repose sur une projection, pas sur une garantie : un travail qui s'arrête la nuit ou le week-end peut faire passer un bon compte derrière les autres le temps d'une journée.
