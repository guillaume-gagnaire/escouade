---
number: 361
id: "le-kanban-du-telephone-un-projet-a-la-fo"
title: "Le Kanban du téléphone : un projet à la fois, une lecture légère, les gestes glissés ou dépliés"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-573"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-573"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":13,"removed":0}]
accepted_at: "2026-11-28T04:37:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

MO-7 (ESC-573) fait l'onglet Kanban de l'app mobile (docs/MOBILE.md § 3.7, M8, décision 236) : un projet avec son espace, une colonne choisie en segments, ses tickets, et les gestes valider (sous verrou), renvoyer avec un mot, lancer, « L'agent résout ». Le téléphone n'avait aucune commande pour lire le tableau d'un projet : space_glance ne nomme que les projets des agents et des demandes, et subscribe rend l'état entier du cœur (réglages, conversations, Git), trop lourd et trop large pour un téléphone. La maquette B ne montre que le glissé, qui n'est pas accessible à tous.

## Décision

1. Une commande servie légère, board_glance (portée « téléphone », src-tauri/src/glance.rs) : tous les projets du cœur, le projet montré (celui demandé, sinon le premier), les nombres de ses colonnes et ses tickets dans l'ordre du bureau, chacun réduit à sa clé, son titre (200 caractères), sa colonne, le nom de son agent, sa boucle, ses critères remplis ou non, son blocage, son conflit et son étape ; jamais sa description, le texte de ses critères ni sa conversation ; les 30 derniers « Terminé » ; sans les tickets masqués chez soi. Le téléphone la lit toutes les 3 s pour le projet montré, toutes les 15 s pour la liste des projets de chaque espace, tant que l'onglet est ouvert.
2. Le projet montré et son espace se choisissent dans une feuille (les espaces, puis leurs projets) ; le choix, deux identifiants, est gardé sur le téléphone. L'onglet s'ouvre sur « À tester », sinon « En cours », sinon « À faire » ; « En revue » n'est un segment que si le projet en a un.
3. Les gestes : « Valider » et « Renvoyer » un ticket à tester, « Lancer » un ticket à faire, « L'agent résout » un conflit ; aucun pendant une validation. Ils passent par les commandes déjà servies au téléphone (ticket_approve, ticket_reject, ticket_start, ticket_resolve_conflict). « Valider », « Lancer » et « L'agent résout » demandent le verrou du système (confirmByLock) ; « Renvoyer » s'en passe, comme refuser, et ne part qu'avec un mot.
4. Deux chemins vers les mêmes gestes : la ligne glissée vers la gauche les découvre à droite (la maquette) ; la ligne touchée, ou atteinte au clavier ou par un lecteur d'écran, les déplie en boutons sous elle (la variante accessible).

## Conséquences

Une commande de plus dans la portée « téléphone » (PHONE_COMMANDS, docs/REMOTE.md), revue comme les autres ; le téléphone ne reçoit jamais l'état entier d'un cœur. Lire un ticket en entier (sa description, ses critères en texte, sa conversation) reste au poste ; l'écrire aussi (MO-11 apportera le ticket dicté). La stratégie servers.aggregateView ne s'applique pas à ce tableau : il montre un projet d'un espace, comme la fenêtre connectée à cet espace, pas une vue qui mêle les espaces.
