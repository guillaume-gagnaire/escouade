---
title: "Les projets de l'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §5, §2.2, §2.6, §2.7, §3.8, §16 (TS-8, TS-7)","crates/escouade-team/src/catalog.rs, crates/escouade-team/src/proposals.rs","src-tauri/src/team_projects.rs, src-tauri/src/teams_catalog.rs, src-tauri/src/teamjoin.rs","src/components/team/TeamProjects.svelte, src/components/team/TeamClone.svelte"]
tickets: ["ESC-480","ESC-493"]
decisions: [186,194]
compared: null
---

# Les projets de l'équipe

La liste des projets d'une équipe, **chiffrée par la clé d'équipe** : le serveur d'équipe n'en lit ni les noms ni les adresses. Ses administrateurs la tiennent ; chaque membre la lit, voit les projets déjà sur son poste et clone les autres d'un clic. Le code reste chez son hébergeur : le serveur d'équipe ne donne aucun accès git et ne garde aucun identifiant git.

## Un projet de la liste

- **Son nom, sa couleur et son logo** : proposés, changeables sur le poste. Le logo est une image PNG, JPEG ou WebP (256 Kio au plus), jamais un SVG.
- **Ses dépôts** : une adresse chacun, et pour un projet à plusieurs dépôts (ESC-169) le dossier de chacun dans le projet (par défaut, la fin de son adresse). L'hébergeur se lit à l'adresse (GitHub, GitLab, Bitbucket, Azure DevOps, Codeberg, sinon son hôte).
- **Sa branche par défaut** : vide, celle du dépôt.
- **Qui y a accès** : toute l'équipe, ou certains membres. Un membre qui n'en est pas voit le projet « Réservé à d'autres membres de l'équipe », sans « Cloner ».
- **Son Kanban** (TS-7) : l'identifiant du journal de son Kanban d'équipe (32 chiffres hexadécimaux), chiffré avec le reste de l'entrée. Vide : pas de Kanban connu ; c'est le cas de toute entrée écrite avant TS-7, qui se lit comme avant. Le formulaire de l'administrateur le garde tel quel quand il change le reste de l'entrée.

## Qui l'écrit

- **Un administrateur** ajoute un projet, le change (entier) et le retire ; ses clones restent sur les postes. Un membre la lit.
- Un membre ne peut pas l'écrire : son app refuse avant l'envoi, le serveur (qui connaît les rôles) refuse une entrée d'un membre dans la liste, et chaque appareil refuse à la lecture une entrée qui n'est pas d'un administrateur.
- Un projet ajouté par un administrateur parti depuis reste dans la liste : un administrateur encore là le réécrit pour qu'un membre arrivé après le lise.
- Un nouvel administrateur écrit la liste une fois que le serveur connaît son rôle (porté par l'appareil d'un administrateur à sa lecture suivante de l'équipe).
- **Le Kanban d'un projet de la liste** est écrit dans son entrée par l'administrateur qui partage ce Kanban (« Partager son Kanban avec l'équipe »). Un membre qui le partage le **propose** : sa proposition va dans un flux de l'équipe à part (scellé par la clé d'équipe, signé par sa feuille, ouvert à toute l'équipe) ; l'écran d'un administrateur la montre sous le projet (« Un membre propose son Kanban pour ce projet : <identifiant> ») avec **« Inscrire le Kanban »**, qui écrit l'entrée. Une proposition qu'aucune feuille de l'équipe n'a signée est refusée à la lecture ; une proposition n'est jamais appliquée sans ce geste.

## Ce qui vient de l'équipe n'est pas de confiance aveugle

- **Seules les adresses `https://` et ssh** (`ssh://…` ou `git@hôte:chemin`) entrent dans la liste. Refusées, à l'écriture comme à la lecture : `file://`, `http://`, `git://`, les autres transports de git (`ext::…`), une adresse ou une partie de chemin qui commence par un tiret (une option de git glissée), un chemin local (`C:\…`, `/…`, `./…`, `~`, un partage `\\serveur`), une adresse qui porte un mot de passe ou un jeton, un espace ou un caractère de contrôle. Un projet dont une adresse, un dossier, la branche, le logo ou le Kanban ne passe pas est refusé entier, et l'écran dit pourquoi (« La liste refuse ce projet : … »).
- **L'adresse est montrée entière avant le clone**, et le clone la relit dans la liste (jamais dans la fenêtre), la revérifie et la donne à git après `--`.
- **Un nom ou un logo de la liste ne remplace jamais ceux d'un projet déjà sur le poste** : l'écran dit « Ici « site », dans la liste « Site ACME » : ton nom et ton logo restent. »
- **Un serveur qui ment** : comme pour le fichier d'équipe, l'appareil relit la dernière entrée lue avant de lire plus loin ; une entrée disparue ou changée arrête la lecture, l'écran le dit, « Réessayer » relit. Rien n'est écrit sous l'alarme. Un serveur qui cache une proposition empêche seulement un administrateur de la voir : il peut toujours inscrire le Kanban lui-même.

## « Projets de l'équipe »

Un bouton de chaque équipe dans la fenêtre des équipes (la barre des projets par espace viendra avec les espaces d'Escouade à distance).

- **Sur ce poste** : les projets de la liste dont un dépôt est déjà cloné ici, avec « Rattacher à l'équipe » tant qu'il n'est pas un projet de l'équipe, puis « Projet de l'équipe » ; « Ouvrir ». Un projet d'une autre équipe le dit.
- **À cloner** : les autres, chacun avec ses adresses entières, sa branche, qui y a accès, et « Cloner ».
- Sous chaque projet, son Kanban : « Kanban de l'équipe rejoint » (ce poste le suit), « Kanban de l'équipe inscrit », ou « Pas encore de Kanban de l'équipe » ; « Ce poste suit un autre Kanban que celui de la liste » quand c'est le cas (il n'en change pas de lui-même).
- **L'administrateur** : « Ajouter un projet », « Modifier », « Retirer » (après confirmation), « Inscrire le Kanban » proposé par un membre. Le formulaire : nom, dépôts (adresse, et dossier quand il y en a plusieurs), branche par défaut, couleur, logo, accès (toute l'équipe ou certains membres).
- Le nombre d'entrées refusées à la lecture est dit.

## Un projet déjà cloné

- **Il se reconnaît à l'adresse d'un remote d'un de ses dépôts**, telle qu'elle est configurée : https et ssh, avec ou sans `.git`, avec ou sans port ni utilisateur, l'hôte en n'importe quelle casse, sont la même adresse. Sur un hôte connu (GitHub, GitLab, Bitbucket, Azure DevOps, Codeberg), la casse du chemin ne compte pas non plus ; ailleurs, elle compte.
- **« Rattacher à l'équipe »** en fait un projet de l'équipe (ce que l'équipe impose et recommande vaut pour lui, page « Le .escouade.yml d'équipe »), sans nouveau clone. Jamais de lui-même : par le geste du membre. Le rattachement (TS-7) :
  - **rejoint le Kanban de l'entrée**, quand elle en nomme un, sans identifiant à copier ; ses tickets d'avant restent sur ce poste jusqu'à « Verser à l'équipe » (page « Le Kanban d'équipe ») ;
  - **écrit la clé `team` du `.escouade.yml` du dépôt** (l'adresse du serveur, l'id de l'équipe, l'id du projet dans la liste, le nom de l'équipe : rien de secret), et le dit : « la clé « team » est écrite dans .escouade.yml. Commite-la : qui ouvrira le dépôt saura qu'il est de l'équipe. » Escouade ne commite rien. Décoché de l'équipe, le projet perd cette clé quand elle nomme cette équipe.
- Dans les réglages de l'équipe, la case « projet de l'équipe » reste pour les projets dont le dépôt n'est pas dans la liste ; ceux de la liste se rattachent dans « Projets de l'équipe », ce que l'écran dit. La case rattache de la même façon (clé `team` écrite, sans id de projet de la liste).

## « Cloner »

- « Cloner » ouvre l'assistant « Nouveau projet » sur le clone : le titre « Cloner « Acme API » de l'équipe ACME », chaque adresse entière, la branche, et « Le clone passe par tes comptes sur l'hébergeur de code (tes identifiants git, tes clés ssh) : le serveur d'équipe ne donne aucun accès et ne garde aucun identifiant ».
- Le dossier : « Dans le dossier » (par défaut, celui qui tient le plus de projets du poste) et « Nom du dossier » (le nom du projet fait nom de dossier). Le dossier et les worktrees restent sur le poste. Un dossier qui existe et n'est pas ce clone est refusé.
- **Le clone passe par git et les comptes du membre** (ses identifiants, son gestionnaire d'identifiants, ses clés ssh), à la branche de la liste quand elle en a une.
- **Sans accès**, l'assistant le dit sous le dépôt : « Demande l'accès à acme/api sur GitHub ».
- **Un projet à plusieurs dépôts** les clone tous, chacun dans son dossier, et dit lequel manque (« 1 dépôt manque : le projet attend qu'il soit cloné ») ; « Réessayer » ne clone que ce qui manque. Un clone arrêté en route ne laisse pas de dossier.
- Une fois tout cloné, l'assistant continue sur le dossier (« Cloné dans … Le projet sera un projet de l'équipe ACME. ») avec le nom, la couleur et le logo de la liste proposés ; le projet créé est rattaché à l'équipe comme ci-dessus : **son Kanban est celui de la liste, avec ses tickets, sans identifiant à copier**. Les tickets des fichiers `.escouade/tickets/` du clone, s'il en a, sont ceux du Kanban de l'équipe (même id) : jamais en double.

## Ce que voit le serveur

Qu'une entrée de la liste est écrite, par quel appareil, sa taille et son heure ; de même pour une proposition. Jamais un nom, une adresse, une branche, un hôte, un Kanban, ni qui a accès ; il ne sait même pas quelles entrées parlent du même projet.
