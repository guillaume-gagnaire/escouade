---
number: 180
id: "l-etat-tient-a-une-coupure-ecriture-forc"
title: "L'état tient à une coupure : écriture forcée, copies tournantes par âge, reprise de la dernière bonne copie au démarrage, dite dans une annonce"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-469"
pages: ["introduction/donnees-et-coupures"]
tickets: ["ESC-469"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/donnees-et-coupures","added":32,"removed":0}]
accepted_at: "2026-11-15T18:52:12+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le 2026-11-15, une coupure du poste a laissé state.json (8,7 Mo) rempli de zéros : write_atomic renommait un temporaire sans forcer ses données sur le disque (NTFS journalise le renommage avant les données). Escouade 2.2.0 est repartie d'un état vide sans rien dire et l'a écrit par-dessus ; aucune copie n'existait. 468 tickets, 490 agents et les projets perdus. L'utilisateur a demandé que cela ne puisse plus jamais arriver.

## Décision

1. write_atomic (et write_private) force les données (sync_all) puis le dossier (Unix : sync_all du dossier ; Windows : MoveFileExW avec MOVEFILE_WRITE_THROUGH, repli sur le renommage ordinaire si Windows le refuse). Les caches qui se refont passent par write_cache, l'écriture légère d'avant. Le coût mesuré (9 Mo : 8,1 ms forcé contre 3,5 ms léger, NVMe) ne demande pas de borner la cadence de 400 ms de l'état.
2. state.json, settings.json, plugins.json et integrations.json ont des copies gzip dans backups/<nom>/ : au démarrage puis toutes les 10 minutes si le fichier a changé, d'un contenu relu et lisible seulement. On garde les 6 dernières, la première de chaque heure sur 48 h, la première de chaque jour sur 30 jours, 256 Mo au plus par fichier (la plus récente toujours). La première d'une période, pas la dernière, pour qu'un état vide copié en boucle ne chasse que les 6 dernières.
3. Au démarrage, avant toute lecture : un de ces fichiers illisible (zéros, coupé, faux) est gardé sous un nom daté (<nom>.unreadable-<date>.json, jamais écrasé) et remplacé par sa copie lisible la plus récente (les illisibles sautées) ; sans copie, l'app part sans lui. Dans les deux cas, recovered.json le note et une annonce en haut de la fenêtre le dit (fichier, dommage, copie et heure, ce qui a pu se perdre, où sont le fichier et les copies) jusqu'à ce que l'utilisateur la ferme, redémarrages compris. Un fichier absent reste une installation neuve, sans annonce.
4. Le journal d'une conversation est forcé sur le disque à la fin de chaque tour ; une fin en zéros ou coupée se lit jusqu'à la dernière ligne entière et l'écriture suivante repart sur une ligne propre.
Pas d'écran de restauration ici (ticket de la 2.3).

## Conséquences

Une coupure coûte au plus quelques centaines de millisecondes d'état et la fin du tour en cours d'une conversation, jamais l'état entier. Le dossier de données gagne backups/ (environ 85 Mo au plus pour un état de 9 Mo), des fichiers *.unreadable-*.json après une reprise, et recovered.json tant que l'annonce est ouverte. state.broken.json disparaît pour ces fichiers. Quelques millisecondes de plus par sauvegarde de l'état et par fin de tour.
