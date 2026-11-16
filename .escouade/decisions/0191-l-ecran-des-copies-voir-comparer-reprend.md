---
number: 191
id: "l-ecran-des-copies-voir-comparer-reprend"
title: "L'écran des copies : voir, comparer, reprendre une copie depuis les réglages ; des copies pour les historiques des projets fermés et ce qui ne se refait pas"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-487"
pages: ["introduction/donnees-et-coupures","harness/serveur-mcp"]
tickets: ["ESC-487","ESC-469"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/donnees-et-coupures","added":41,"removed":8},{"page":"harness/serveur-mcp","added":3,"removed":0}]
accepted_at: "2026-11-16T19:08:49+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 180 (ESC-469) a donné des copies à state.json, settings.json, plugins.json et integrations.json et leur reprise au démarrage quand un fichier est illisible, sans écran pour choisir une copie à la main ; les historiques des projets fermés, la file de synchronisation, les autres comptes de tickets et le suivi des workflows n'avaient aucune copie. Le 2026-11-15, après la coupure, le Kanban (461 tickets) a été reconstruit à la main depuis les journaux, faute de pouvoir voir, comparer et reprendre une copie, et faute de copies des historiques des projets fermés.

## Décision

1. Ont des copies, avec la même rotation (6 dernières, 1 par heure sur 48 h, 1 par jour sur 30 j) : en plus des quatre fichiers de la décision 180, ticket-accounts.json, sync-queue.json, chaque closed-projects/<id>.json et chaque projects/<id>/workflow-tracks.json, 32 Mo au plus chacun. Toutes les copies ensemble tiennent en 512 Mo (les plus anciennes partent d'abord, jamais la plus récente d'un fichier). Les copies d'un fichier disparu restent 30 jours. N'ont pas de copies : les conversations (forcées à chaque tour, trop grosses), la mémoire partagée (dans le projet, partagée par git), stats.db, les caches.
2. Réglages › Avancé › « Copies des données » : chaque fichier, ce qu'il contient aujourd'hui, ses copies avec leur date, leur taille et ce qu'elles contiennent en comptes ; « Comparer » dit par nom ce qu'une copie ramènerait et ce qui serait perdu, sans rien écrire ; « Faire une copie maintenant », « Ouvrir le dossier des copies », le total et la borne, les fichiers mis de côté avec leur chemin.
3. « Reprendre » : confirmé, la copie relue et lisible, le fichier d'aujourd'hui mis de côté sous <nom>.before-restore-<date>.json (jamais effacé). Un historique de projet fermé est repris aussitôt ; tout autre fichier, tenu en mémoire, est posé au prochain démarrage (restore-pending.json, « Redémarrer maintenant », « Annuler la reprise »), et la reprise comme le redémarrage sont refusés tant qu'un agent travaille. L'annonce du démarrage dit la copie reprise et où est le fichier d'avant.
4. Un historique de projet fermé disparu du disque, dont une copie reste et que ni l'index ni les projets ouverts ne citent, est proposé à « Retrouver » puis « Rouvrir ».
5. Rien de secret : l'écran et le MCP ne reçoivent que des comptes et des noms, jamais la valeur d'un champ ; app.log ne nomme aucun chemin du disque. Le MCP a list_backups en lecture seule et aucun outil pour reprendre une copie : c'est un geste de l'utilisateur. Pour l'API distante, les commandes de l'écran sont servies, sauf restart_for_restore, du poste seulement (comme quit_app).

## Conséquences

Une perte comme celle du 2026-11-15 se répare en quelques clics, sans reconstruire le Kanban à la main ; un projet fermé perdu se retrouve. Le dossier de données gagne des copies pour les historiques des projets fermés et quelques petits fichiers (borné à 512 Mo en tout), des fichiers <nom>.before-restore-<date>.json après une reprise (à supprimer à la main), restore-pending.json tant qu'une reprise attend, restored.json tant que l'annonce est ouverte. Reprendre l'état, les réglages, les plugins ou les comptes demande un redémarrage et attend la fin du travail des agents.
