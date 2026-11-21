---
title: "Passer un agent à un collègue"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §4.2, §11, §13, §15.2 (C16), §16 (TS-17)","docs/TEAM-SESSION-COPY.md (ESC-581, TS-0c : l'essai de reprise d'une session copiée, la recommandation)","Réponse de l'utilisateur du 2026-11-21 (demande a-23) : « C16 : a »","crates/escouade-team/src/handoff.rs","crates/escouade-team-server/src/db/mod.rs","src-tauri/src/teamhandoff.rs","src-tauri/src/handover.rs"]
tickets: ["ESC-548","ESC-581","ESC-588","ESC-607"]
decisions: [274]
compared: null
---

# Passer un agent à un collègue

L'agent d'un ticket de l'équipe se passe à un collègue, ou à une machine de l'équipe en mode serveur, avec son ticket, sa branche, son travail en cours et sa conversation (TS-17, §11 de `docs/TEAM-SERVER.md`). Choix C16-A : chez qui le prend, une **nouvelle session** sur son harness et son compte, dont le premier message est le dossier de passation d'ESC-88 ; la reprise native d'une session copiée (C16-B) est décidée pour Claude Code seulement, avec la passation en repli, et pas encore construite (TS-17b, ESC-607 ; voir « La reprise native de la session »).

## Le geste

- Le menu de la carte d'un ticket de l'équipe **que ce poste tient, avec son agent**, offre « Passer à un collègue… ». Jamais un agent : le serveur MCP n'a aucun outil pour passer un agent.
- **La fenêtre dit ce qui passe avant le geste** :
  - « À qui » : les collègues du projet (une pastille, leur nom ; « ton autre poste » pour un autre appareil de son membre), puis « Machines de l'équipe » (« En mode serveur : elle le fait tourner jour et nuit. »). Seuls les appareils du projet qui ont ouvert son Kanban y sont ; personne d'autre : « Personne d'autre n'a ouvert ce projet de l'équipe… ».
  - « La branche ticket/dem-41 sera poussée sur origin. Rien n'est poussé avant « Passer ». »
  - « L'agent travaille : son tour sera interrompu avant de passer. », ou « L'agent est arrêté entre deux tours. »
  - La conversation : « chiffrée par la clé du projet, déposée sur le serveur le temps du passage, effacée une fois reprise ou le passage annulé. »
  - Le travail non commité (« 2 fichiers non commités », les premiers nommés) : **« Un patch chiffré »** (coché d'office : chiffré par la clé du projet, appliqué tel quel chez qui reprend, rien ne s'ajoute à la branche) ou **« Un commit « en cours » sur la branche »** (poussé avec elle, message `wip: travail en cours de l'agent, passé à Bob [DEM-41]`). Sans travail non commité : « Aucun travail non commité. ».
- **« Passer »**, dans cet ordre : le tour en cours interrompu ; le travail mis en commit « en cours » ou en patch ; la branche poussée ; le dépôt chiffré et l'offre écrits ; la prise passée par le serveur. « Passé à Bob Durand ».
- **Sur ce poste ensuite** : la carte dit « Passé à Bob Durand · en attente qu'il le prenne », avec « Annuler le passage » ; l'agent reste, arrêté, sa conversation le dit (« Passé à Bob Durand : la branche ticket/dem-41 est poussée sur origin… »). Son dossier est détaché de la branche (ses fichiers et son travail y restent), pour qu'elle se reprenne dans un autre worktree, même plus tard sur ce poste.
- **« Annuler le passage »**, tant que le collègue ne l'a pas pris : la prise revient à ce poste sous un nouveau jeton, l'offre et le dépôt sont effacés, l'agent retrouve son ticket et sa branche (« Passage annulé : l'agent et son ticket restent sur ce poste. »). Pris entre-temps : « Bob Durand l'a déjà pris : le passage est fait. ».

## Ce qui passe

- **Le ticket** : le serveur passe la prise à l'appareil choisi, sous un nouveau jeton (le jeton croît, §4.2) : l'ancien jeton de ce poste n'écrit plus rien. Une prise passée a un bail de **14 jours** tant qu'elle n'est pas prise ; la prendre donne encore un nouveau jeton. Le serveur refuse une prise passée à un appareil révoqué, sans accès au projet, ou à soi.
- **La branche** : poussée chez l'hébergeur par le geste de qui passe la main, jamais avant.
- **Le travail non commité** : commit « en cours » ou patch, au choix.
- **La conversation** : le journal d'Escouade de l'agent, dans le **dépôt**, un fichier du serveur chiffré par la clé du projet (au-delà de 20 Mo, les plus anciens messages sont laissés de côté). Il est effacé à la reprise et à l'abandon. Le serveur voit un fichier d'une taille, jamais ce qu'il contient.
- **Jamais un compte** : ni identifiant, ni jeton, ni réglage de compte ne passe.

## Chez le collègue

- **La proposition** : « Alice Martin te passe DEM-41 » sur la carte du ticket, avec « Prendre », et dans sa boîte « À toi » (« Agent passé », « Alice Martin te passe DEM-41 · La passation », même geste « Prendre », source `handoff` des vues de la boîte). Elle ne se croit qu'avec la prise : celle que le serveur dit passée à cet appareil par celui qu'elle nomme.
- **« Prendre »** : sur le harness de l'agent quand ce poste l'a pour le projet (écrit, trouvé, allumé, un compte qui sert le projet) ; sinon « Choisis ton harness » (« Claude Code n'est pas sur ce poste pour ce projet : l'agent repart sur le tien, avec la passation. ») liste ceux de ce poste ; aucun : « Aucun harness ne sert ce projet sur ce poste… ».
- Escouade récupère la branche de son hébergeur, prend le ticket (un nouveau jeton), efface l'offre et le dépôt, puis lance l'agent du ticket comme « Lancer » : son worktree **sur la branche**, son compte à lui, le protocole du ticket. Le patch y est appliqué (s'il ne s'applique pas, la conversation le dit et le patch est gardé dans un fichier nommé).
- **La conversation** : la conversation d'avant, à lire, puis le séparateur « Passé par Alice Martin · suite sur Claude Code », puis le **dossier de passation** replié (« Dossier de passation »), premier message de la nouvelle session : « Passation : suite d'un travail que Alice Martin t'a passé », ce qui a été demandé, fait, les fichiers, l'état git de son worktree, ce qui attendait.
- Chez qui l'a passé, la carte dit ensuite « Pris par Bob Durand ».

## Vers la machine de l'équipe

- Une machine en **mode serveur** (`escouade-server`, membre de l'équipe) le dit dans chaque projet de l'équipe qu'elle ouvre : la fenêtre du passage la range sous « Machines de l'équipe ».
- Le même geste lui passe l'agent ; **elle le prend d'elle-même**, sur son harness (celui de l'agent s'il y est, sinon le premier qui sert le projet), et le fait tourner jour et nuit.

## Les refus

Dits avec leur raison, rien n'est passé ni perdu :

- un **agent d'un projet personnel** : « Cet agent est d'un projet personnel : seul l'agent d'un ticket d'un projet de l'équipe se passe à un collègue. » ; un ticket « Ne pas partager », aussi ;
- **quelqu'un hors du projet** : « Cet appareil n'est pas du projet demo… », ou, refusé par le serveur, « Bob n'a pas accès à ce projet sur le serveur de l'équipe : rien n'est passé. » ;
- **une branche dont l'hébergeur ne répond pas** : « L'hébergeur de la branche ticket/dem-41 ne répond pas : rien n'est passé, l'agent et son travail restent ici. (…) » — le commit « en cours » est défait ; chez qui prend, « La branche … ne se récupère pas de son hébergeur : le passage attend, rien n'est perdu. » ;
- sans dépôt distant : « La branche … n'a pas d'hébergeur où la pousser » ; un agent hors de son worktree, un projet de plusieurs dépôts, un ticket que ce poste ne tient pas, un passage déjà en cours ; hors ligne.

## Le serveur MCP

- Aucun outil ne passe un agent.
- `get_ticket` et `list_tickets` disent, dans `team`, `handoff` : `from`, `to`, `machine`, `branch`, `toHere`, `fromHere`, `since`.

## Ce que voit le serveur

La prise (son id opaque, l'appareil qui la tient, celui qui l'a passée tant qu'elle n'est pas prise, le jeton, les heures), un fichier chiffré d'une taille, une opération chiffrée de plus au journal du projet. Jamais le ticket, la branche, le code, la conversation ni les noms.

## La reprise native de la session (C16-B pour Claude Code, décidée, pas encore construite)

Décidée par l'utilisateur le 2026-11-21, d'après l'essai d'ESC-581 (`docs/TEAM-SESSION-COPY.md`) ; construite par TS-17b (ESC-607). D'ici là, tout passage se fait en passation (ci-dessus).

- **Seulement Claude Code, des deux côtés** : l'agent passé tourne sur Claude Code, entre deux tours, et le collègue le prend avec Claude Code. Les autres harness gardent la passation.
- **Ce qui passe en plus** : la session de l'agent, le chemin de son worktree et la version de `claude`, dans le même dépôt chiffré que la conversation, effacés du serveur une fois repris ou le passage annulé.
- **Les contrôles, chez qui reprend** : sa version de `claude` égale ou plus récente ; la session réécrite par chaîne JSON, du worktree de qui passe la main vers le sien, sous ses deux séparateurs ; plus aucune occurrence de l'ancien worktree. Elle est déposée dans le dossier du compte choisi, puis l'agent est lancé par `--resume <id> --fork-session` (un nouvel identifiant : jamais deux histoires sous le même).
- **Le repli** : à tout échec (version, chemin restant, session non trouvée, erreur au premier tour), la passation d'ESC-88, comme aujourd'hui.
- **Ce que dit l'écran** : « Reprise de la session » ou « Passation » au séparateur de la conversation ; chez qui passe la main, la fenêtre dit que la session part avec l'agent, chiffrée.
- **Reste à essayer par l'utilisateur** : la reprise d'un compte ou d'une organisation à l'autre (demande « À toi » a-24).
