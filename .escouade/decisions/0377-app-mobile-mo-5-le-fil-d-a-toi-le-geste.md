---
number: 377
id: "app-mobile-mo-5-le-fil-d-a-toi-le-geste"
title: "App mobile (MO-5) : le fil d'« À toi », le geste armé à la fin de la lecture, ce que le téléphone garde"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-571"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-571"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":24,"removed":14}]
accepted_at: "2026-11-28T16:35:48+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

MO-5 (ESC-571) fait du téléphone l'endroit où l'on répond vite : le fil de B, les demandes dépliées sur place, la boîte entière, la conversation et l'hors ligne (docs/MOBILE.md §§ 3.3 à 3.6 et § 5, choix M3, M4, M9, M13). Il fallait fixer comment le fil se range et reste stable, comment « Autoriser » sait que la commande a été lue, ce que le téléphone lit de chaque espace et ce qu'il en garde, et ce que le natif ajoute pour la dictée et les fichiers.

## Décision

1. Le fil reprend les groupes et l'ordre de la boîte du bureau sur tous les espaces montrés (les règles pures sorties de inbox.svelte.ts dans src/lib/inbox/rules.ts, lues par les deux) ; une ligne garde sa place d'une relecture à l'autre (seule une ligne nouvelle s'insère) et son point tant qu'on ne l'a pas dépliée dans son état ; une demande de la boîte dit elle-même si elle a été vue (inbox_seen). 2. Le téléphone lit la boîte de chaque espace (inbox_list, inbox_views) quand son coup d'œil change, et au moins toutes les 15 s ; une réponse à un agent passe par sa vue de la boîte quand il en a une (réglée ailleurs, elle est refusée), sinon par answer_permission ou answer_question ; aucune commande nouvelle n'est servie au téléphone. 3. « Autoriser » ne s'arme qu'une fois la dernière ligne de la commande entière vue (une marque après elle, observée dans la vue) ; une commande portée coupée est relue dans la conversation, et si elle ne peut pas l'être, le geste reste éteint ; au-delà de douze lignes à la largeur du téléphone, la commande se lit en plein écran. Les gestes qui font agir passent par confirmByLock (autoriser, approuver un plan, oui à un feu vert, valider un ticket, prendre un agent, accepter une tâche qui lance des agents). 4. Le téléphone ne garde (stockage privé de l'app) que le dernier coup d'œil de chaque espace sans ce qu'un agent demande (ni commande, ni motif, ni question), les brouillons et les clés des lignes vues ; retirer un espace l'oublie ; les pièces jointes ne vivent qu'en mémoire. 5. La photo jointe à un message part dans le message même (réduite à 2048 px, JPEG), comme une image du bureau ; les fichiers joints par chemin (attach_*) restent au poste. 6. Le plugin gagne dictate, stopDictation (la reconnaissance du système, sur l'appareil quand il le permet) et openFile (un fichier écrit dans le cache de l'app, ouvert par le choix du système) ; dans un navigateur, son côté web tient lieu du système pour les essais.

## Conséquences

Le bureau n'a rien perdu : sa boîte lit les mêmes règles, réexportées. Le téléphone fait deux lectures de plus par espace quand quelque chose change (et toutes les 15 s), et une lecture de la conversation quand on déplie une demande d'agent. « Déjà réglée » avec qui et quand (§ 9) reste à faire côté cœur : le téléphone dit « Réglée ailleurs ». La dictée sous iOS et l'ouverture d'un fichier sous iOS ne se construisent que sur un Mac (non vérifiées en local) ; l'essai réel sur un téléphone reste celui de l'utilisateur, déposé par MO-4.
