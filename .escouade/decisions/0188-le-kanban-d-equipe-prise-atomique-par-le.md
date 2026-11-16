---
number: 188
id: "le-kanban-d-equipe-prise-atomique-par-le"
title: "Le Kanban d'équipe : prise atomique par le serveur, droits par projet, clés communes et conflits montrés au perdant (TS-6)"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-479"
pages: ["serveur-equipe/kanban-equipe","serveur-equipe/synchronisation","kanban/pilote-auto","kanban/kanban","harness/serveur-mcp"]
tickets: ["ESC-479"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/kanban-equipe","added":53,"removed":0},{"page":"serveur-equipe/synchronisation","added":2,"removed":0},{"page":"kanban/pilote-auto","added":4,"removed":0},{"page":"kanban/kanban","added":2,"removed":0},{"page":"harness/serveur-mcp","added":4,"removed":0}]
accepted_at: "2026-11-16T12:58:00+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-5 (ESC-466) a donné à chaque projet d'équipe un journal d'opérations chiffrées et une copie locale, sans écran ni prise de ticket, et le serveur servait un projet à tout appareil de l'équipe. TS-6 fait de ce journal le Kanban d'équipe : il faut empêcher deux postes de lancer le même ticket, numéroter les tickets sans que le serveur lise leur préfixe, montrer les écritures concurrentes, dire au pilote auto ce qu'il peut prendre (C11-A) et faire respecter les droits par projet côté serveur. docs/TEAM-SERVER.md §4, §2.5, §2.6, §3.8.

## Décision

1. La prise d'un ticket se fait sur le serveur, en une transaction, sur un id opaque de 128 bits (champ chiffré du ticket, distinct de son objet dans le journal) : une seule demande passe, les autres reçoivent l'appareil qui le tient, traduit en nom par les noms que chaque appareil écrit (chiffrés) dans le projet. Bail de 10 min renouvelé toutes les 3 min tant que le ticket est « En cours » sur le poste, rendu quand il en sort ; version attendue ; jeton croissant porté par chaque déplacement du preneur et refusé s'il est dépassé (le déplacement refusé est retiré de la file et de la copie, la chaîne de l'appareil reste sans trou). Un bail expiré rend le ticket libre, « interrompu chez Alice » ; le pilote auto ne le reprend jamais, « Reprendre » le fait à la main. Hors ligne, aucune prise.
2. Le serveur tient les droits par projet : tous les appareils de l'équipe pour un projet de toute l'équipe (ouvert par sa première écriture), ceux des membres d'un projet gardé (dits par un administrateur avant toute écriture, puis à chaque changement du groupe). Seul un appareil qui y a accès lit, écrit, prend, tire une clé et reçoit les trames ; seul un administrateur change l'accès d'un projet existant ; un appareil révoqué ou sorti du projet ne tient plus ses prises.
3. Les clés d'un projet d'équipe viennent d'un compteur du serveur par projet ; le préfixe est chiffré dans le journal et le poste forme la clé. Clé de poste `DEM-L3` hors ligne et pour « Ne pas partager » ; un ticket importé garde la sienne.
4. Conflits champ par champ (C2-A) montrés au seul perdant (« Bob a changé le titre en même temps · Reprendre le mien / Garder le sien ») ; une même valeur des deux côtés n'en est pas un ; une description fusionnée à trois voies ligne par ligne, sinon « Garder le mien / Prendre le sien ». La colonne d'un ticket pris ne change que par son preneur ou un administrateur (sur les postes honnêtes).
5. Pilote auto (C11-A) : les tickets de l'équipe assignés au membre du poste, plus « À faire » si « Prendre aussi « À faire » de l'équipe » est coché, réglage du poste jamais partagé.
6. MCP : start_ticket passe par la prise et refuse avec le nom du preneur ; get_ticket et list_tickets disent le preneur ; les textes écrits sur le poste d'un collègue sont marqués <third_party_data>.
7. Un Kanban se rattache par l'écran de l'équipe : « Partager son Kanban avec l'équipe » ou « Rejoindre » avec l'identifiant d'un collègue (en attendant la liste de TS-8) ; les tickets déjà là restent sur le poste (TS-7 les versera).

## Conséquences

Le serveur voit en plus, pour chaque prise, l'appareil, ses heures, son jeton et sa version, le compteur des clés, et qui a accès à quel projet (table de §3.8) ; jamais le titre, la clé, la colonne ni l'objet d'un ticket (vérifié par un test). Reste de confiance, écrit dans la spec : le serveur pourrait mentir sur les droits (pas lire) ; un nouvel appareil d'un membre d'un projet gardé n'a l'accès serveur qu'après le passage d'un administrateur ; un client modifié peut déplacer un ticket qu'il ne tient pas (le serveur ne voit pas les déplacements, seul le jeton du preneur est vérifié) ; l'horodatage reste celui de l'auteur. Le Kanban relit les prises sur une trame « claim » du direct ou toutes les 5 s, pour rester sous la limite de débit du serveur. À venir : TS-7 (import), TS-8 (liste des Kanbans), TS-17 (passer l'agent au « Reprendre »), TS-4 (l'état « Hors ligne · N changements »).
