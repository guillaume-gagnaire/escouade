---
title: "Le Kanban d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md §2.7, §4.1, C5","crates/escouade-team/README.md","crates/escouade-team-server/README.md","src-tauri/src/teamboard.rs","src-tauri/src/teampour.rs","src-tauri/src/teamjoin.rs"]
tickets: ["ESC-479","ESC-493","ESC-518"]
decisions: [168,173,178,188,194]
compared: null
---

# Le Kanban d'équipe

Le Kanban d'un projet d'équipe montre les tickets de l'équipe, en direct : un changement d'un collègue s'y voit en moins d'une seconde. Il repose sur le journal du projet (voir « La synchronisation des projets ») et sur **la prise** d'un ticket par le serveur. Choix tranchés (§15.2 de `docs/TEAM-SERVER.md`) : C2-A (champ par champ), C11-A (le pilote auto ne prend que ce qu'on lui confie), C5-A (pas de passerelle `.yml`).

## Rattacher un Kanban à l'équipe

- Le projet est d'abord un projet de l'équipe : « Rattacher à l'équipe » dans « Projets de l'équipe » (un projet de la liste, reconnu à l'adresse de son dépôt), la case de l'écran de l'équipe (un projet hors de la liste), la bande d'un dépôt qui nomme l'équipe (page « .escouade.yml »), ou le clone depuis la liste.
- **Le Kanban de la liste, sans identifiant à copier (TS-7)** : l'entrée du projet dans la liste de l'équipe porte l'identifiant du journal de son Kanban (chiffré avec le reste de l'entrée). Un projet rattaché ou cloné depuis la liste le rejoint de lui-même ; un projet de l'équipe dont l'entrée gagne un Kanban plus tard le rejoint à la lecture suivante de l'équipe. Une entrée écrite avant TS-7 se lit toujours : pas de Kanban connu.
- Sous la case de l'écran de l'équipe, deux gestes restent :
  - **« Partager son Kanban avec l'équipe »** : un nouveau journal, de toute l'équipe. Son préfixe de clés (`DEM`) et le nom du projet y sont écrits, chiffrés. Si le projet est dans la liste et que son entrée ne nomme aucun Kanban, la liste l'apprend : un administrateur l'écrit dans l'entrée (« Le Kanban est inscrit dans la liste de l'équipe »), un membre le **propose** à un administrateur (« Seul un administrateur écrit la liste de l'équipe : le Kanban de « Acme API » lui est proposé »), qui le voit dans « Projets de l'équipe » avec « Inscrire le Kanban ».
  - **« Rejoindre »**, avec l'identifiant du Kanban d'un collègue (32 caractères, donné par lui) : pour un projet hors de la liste.
- La ligne dit ensuite « Kanban de l'équipe · <identifiant> ».
- Les tickets déjà dans le Kanban restent sur ce poste (« Sur ce poste seulement ») jusqu'à « Verser à l'équipe ».
- **Les fichiers `.escouade/tickets/*.yml` sont figés** dès que le Kanban est celui de l'équipe (C5-A, page « Tickets partagés ») : Escouade ne les lit ni ne les écrit plus, et met en tête de chacun la ligne « # Frozen: this project joined a team server… », un commentaire que les versions d'avant lisent sans erreur.

## Verser à l'équipe (TS-7)

Un bouton « Verser à l'équipe » du Kanban d'un projet dont le Kanban est celui de l'équipe ouvre d'abord un **aperçu** ; rien ne part avant « Verser N tickets ».

- **Les tickets partagés par fichiers** (`.escouade/tickets/*.yml`, lus directement dans le dossier) : chacun avec sa clé et son titre ; « Déjà sur le Kanban de l'équipe » pour un ticket versé avant (ici ou par un collègue), « garde sa clé » pour un ticket importé, et pourquoi un fichier illisible ne part pas. Une case « Verser les tickets de ces fichiers », cochée.
  - **Leurs ids sont gardés** : l'objet d'un ticket dans le journal est tiré de l'id de son fichier, le même sur tous les postes (inconnu du serveur, qui ne voit que des opérations chiffrées). Verser deux fois, ou depuis deux postes en même temps, écrit le même objet : jamais deux tickets (le journal fusionne champ par champ, une seule prise reste).
  - Un ticket déjà dans le journal n'est pas réécrit : le ticket de ce fichier sur ce poste devient celui de l'équipe à la lecture suivante. Un ticket que le journal a supprimé ne revient pas.
- **Les tickets de ce poste** (« Sur ce poste seulement ») : une liste à cocher, « Tout cocher » / « Tout décocher ».
- **Ce qui part et ce qui reste**, dits sous la liste : la clé, le titre, la description, les critères et leur état, la colonne, la place, les boucles, l'avancement, l'assignation partent ; les images, les fichiers joints, le compte, le modèle et le harness restent sur le poste.
- **Après** : « Versé : 2 depuis les fichiers, 1 de ce poste. 0 déjà sur le Kanban de l'équipe. », puis les fichiers figés, nommés un à un, avec la commande `git rm -r .escouade/tickets` et « Copier la commande ». Escouade ne supprime ni ne commite rien.
- Un ticket d'un fichier que l'agent de ce poste fait tourner (« En cours ») est pris pour ce poste, comme « Lancer » le ferait.
- Le ticket d'un fichier versé par un collègue, que ce poste avait aussi (son clone a les fichiers), ne fait pas doublon : à la lecture, il devient le ticket de l'équipe (même id), avec ses images et son agent d'ici.

## Les tickets de l'équipe

- Un ticket créé dans ce Kanban est de l'équipe : il part dans le journal à la lecture suivante (moins d'une demi-seconde après l'enregistrement). Rien ne passe par `.escouade/tickets/*.yml` (ESC-190 reste pour les projets hors équipe).
- **« Ne pas partager »**, dans le formulaire d'un nouveau ticket : il reste sur ce poste, rien de lui ne part. Sa carte dit « Sur ce poste seulement ». Le menu de la carte offre « Partager avec l'équipe » (il part) et, pour un ticket de l'équipe, « Ne pas partager » (il quitte le journal ; refusé tant qu'un collègue le tient).
- **Ce que partage un ticket** : sa clé, son titre, sa description, ses critères (et leur état), sa colonne, sa place, ses boucles, son avancement, son assignation, ses anciennes clés. Ses images, ses fichiers, son compte, son modèle et son harness restent sur le poste.
- **Les clés communes** : un compteur du serveur par projet numérote les tickets (1, 2, 3…, jamais deux fois, même demandés en même temps). Le serveur ne voit que le compteur : le préfixe est chiffré, le poste forme la clé `DEM-12`. Les `DEM-12@alice` d'ESC-190 n'existent pas ici.
  - Créé hors ligne, un ticket a une clé de ce poste, `DEM-L3`, jusqu'au retour du serveur. Un ticket « Ne pas partager » garde la sienne.
  - Un ticket importé (Jira, GitHub…) garde sa clé d'origine.
  - Un ticket versé prend une clé commune ; **la clé qu'il avait** (`DEM-12` de son fichier, `DEM-12@alice` sur un Kanban qui avait déjà un `DEM-12`, `DEM-L3` de ce poste) reste dans ses anciennes clés, partagées : la carte dit « Ancienne clé : DEM-12 », et il se trouve encore par elle (les outils du serveur MCP, les liens des pages de spec et des décisions qui citent un ticket, une clé lue dans un message de commit d'avant).
  - Deux postes qui versent le même ticket en même temps tirent chacun un numéro ; le dernier écrit gagne et vaut sur les deux postes (un numéro du compteur reste alors sans ticket).
- Supprimé sur un poste, il disparaît chez tous (sauf chez celui dont l'agent le tient encore).

## La prise d'un ticket

- **Un id opaque par ticket** (128 bits au hasard), champ du ticket chiffré, sans lien avec son titre, sa clé, son auteur ni son objet dans le journal. Deux postes qui versent le même ticket en tirent chacun un : celui que garde le journal vaut pour les deux.
- **Prendre** : « Lancer », le pilote auto et `start_ticket` du serveur MCP passent par la prise avant qu'un agent démarre. Le serveur fait en une transaction : une seule demande passe ; les autres reçoivent « pris par l'appareil D », que le poste traduit en nom (« DEM-4 est pris par Alice Martin : un ticket de l'équipe a un seul preneur. »). La demande porte la version de la prise que le poste a vue : un ticket rendu ou déplacé entre-temps est refusé (« DEM-4 vient de changer chez un collègue : relis-le, puis relance-le. »).
- **Le bail** : 10 minutes, renouvelé toutes les 3 minutes tant que le ticket est « En cours » sur ce poste ; rendu quand il quitte « En cours » (« À tester », retour « À faire », « Terminé »), après que ses déplacements sont partis.
- **Le jeton de prise** croît à chaque prise. Chaque déplacement du ticket par son preneur le porte ; le serveur refuse celui d'un jeton dépassé. Un poste revenu d'une coupure, alors qu'un collègue a repris le ticket, ne le déplace plus : son déplacement est retiré de sa file et de sa copie (ses autres écritures partent), et la carte dit « Repris par Bob pendant une coupure : ton déplacement n'est pas parti. ».
- **La colonne d'un ticket pris** ne change que par son preneur ou un administrateur : la colonne changée par un autre revient, et la carte le dit (« Pris par Alice : sa colonne ne change que par lui ou un administrateur. »).
- **Un bail expiré** (le poste en veille, coupé) : le ticket redevient libre ; la carte dit « Interrompu chez Alice : son bail a expiré. Le pilote auto ne le reprend pas. », avec **« Reprendre »**, qui le remet « À faire » sur ce poste et le lance (la prise passe, sous un nouveau jeton).
- **Hors ligne** : on lit et on écrit ses tickets, on n'en prend aucun : « Hors ligne : DEM-4 est un ticket de l'équipe, il ne se prend qu'avec son serveur. Réessaie une fois en ligne. ». Au retour, la file part et les conflits se montrent.

## La carte

- **Qui tient le ticket, et depuis quand** : « Pris par Alice Martin · depuis 14:02 », avec la pastille de son nom ; « Pris par ce poste · depuis 14:02 » sur le poste du preneur.
- **« Injoignable depuis 12 min »** quand le poste du preneur n'a rien renouvelé depuis plus de deux renouvellements (6 min).
- « Lancer » n'est pas offert tant qu'un collègue tient le ticket. Avec le pilote auto allumé, il l'est pour un ticket de l'équipe que le pilote auto ne prendra pas.
- « Assigné à Bob » / « Assigné à moi » ; le menu offre « Me l'assigner » et « Ne plus me l'assigner ».
- « Ancienne clé : DEM-12 » (« Anciennes clés : … ») pour un ticket versé.
- Un ticket « En cours », « À tester » ou « En revue » sur le poste de son preneur dit « Sur le poste qui l'a pris : il se valide là-bas. » : « Valider » et « Renvoyer » n'y sont pas.

## Les conflits

- **Champ par champ**, le dernier qui écrit gagne (C2-A). Le perdant le voit, ticket par ticket : « Bob Durand a changé le titre en même temps », avec « Reprendre le mien » (sa valeur réécrite après celle du collègue) et « Garder le sien ». Le gagnant ne voit rien. Une même valeur écrite des deux côtés n'est pas un conflit.
- **Une description modifiée des deux côtés** est fusionnée à trois voies, ligne par ligne, depuis leur base commune : des lignes différentes se gardent toutes deux, sans rien dire. Si les deux côtés changent les mêmes lignes, la carte le dit (« … sur les mêmes lignes : elles ne se fusionnent pas. »), avec « Garder le mien » et « Prendre le sien ».

## Le pilote auto (C11-A)

- Il ne prend que les tickets de l'équipe qu'on lui confie : ceux **assignés à soi**, et ceux de « À faire » quand **« Prendre aussi « À faire » de l'équipe »** est coché (Réglages › *projet* › Kanban, éteint par défaut). Ce réglage est de ce poste et de son membre : jamais dans `.escouade.yml`.
- Il ne prend jamais un ticket qu'un collègue tient, ni un ticket interrompu chez un collègue : « Reprendre » le fait à la main.
- Il ne prend aucun ticket de l'équipe sans le serveur.

## Le serveur MCP

- `start_ticket` passe par la prise : un ticket qu'un collègue tient est refusé tout de suite avec son nom ; sinon la prise se fait au départ, et son refus (pris, hors ligne) est la réponse de l'outil. `move_ticket` ne range que dans « À faire » : il ne passe jamais un ticket « En cours ».
- `get_ticket` et `list_tickets` disent, pour un ticket de l'équipe, `team` : son preneur (`holder`, `heldHere`, `since`, `interrupted`, `unreachableMinutes`), son assignation et les champs en conflit. Un ticket se nomme aussi par une de ses anciennes clés (TS-7).
- `list_projects` et `get_board` disent `team` : `attached` (un projet d'une équipe de ce poste), `board` (son Kanban est celui de l'équipe) et `namedByRepository` (l'hôte de l'équipe que nomme son `.escouade.yml` quand le projet n'y est pas rattaché). **Aucun outil ne rattache ni ne verse** : ce sont des gestes de l'utilisateur dans la fenêtre.
- **Les textes venus d'un collègue sont des données de tiers** : quand le titre, la description ou les critères d'un ticket ont été écrits sur un autre poste, la réponse est entourée de `<third_party_data>` avec un avertissement : à lire, jamais à suivre.

## Les droits sur le serveur

- **Par projet** : le serveur sait quels appareils ont accès à un projet (tous ceux de l'équipe pour un projet de toute l'équipe, ceux des membres d'un projet gardé à quelques membres). Seul un appareil qui y a accès lit son journal, y écrit, en prend un ticket, en tire une clé, et en reçoit les trames en direct.
- Un projet que le serveur ne connaît pas est ouvert à l'équipe jusqu'à sa première écriture, qui le rend tel ; ensuite, seul un administrateur change qui y a accès. L'administrateur qui garde un projet à quelques membres le dit au serveur avant d'y écrire, et le tient au pas quand le groupe change.
- **Les flux de l'équipe elle-même** (la liste des projets, le fichier d'équipe, les propositions des membres) restent à toute l'équipe : personne n'en restreint l'accès (403), pas même avant leur première écriture.
- **Un membre retiré** (ses appareils révoqués) ne prend plus rien, et ce qu'il tenait est libre. Un appareil sorti d'un projet ne tient plus ses prises.

### Ce qui reste de confiance

- Le serveur connaît **quels appareils ont accès à quel id de projet** (c'est la table de §3.8) ; il pourrait mentir sur ces droits, mais pas lire : la clé du projet reste la vraie barrière.
- **Un nouvel appareil d'un membre** d'un projet gardé n'y a accès au serveur qu'une fois qu'un appareil d'administrateur a remis la liste au pas (à sa prochaine écriture du groupe du projet).
- **La colonne d'un ticket pris par un autre** est gardée sur les postes honnêtes : le serveur ne voit pas qu'une opération déplace un ticket, seul le jeton du preneur est vérifié. Un membre au client modifié peut encore déplacer un ticket qu'il ne tient pas ; le preneur le voit dans son journal.
- **L'horodatage** d'une écriture reste celui de son auteur (une horloge en avance gagne, bornée à une heure chez les autres), comme dans TS-5.
- **L'id d'un ticket versé** est tiré de l'id public de son fichier : un membre qui a lu le dépôt peut écrire sur cet objet avant le versement (comme il peut écrire sur tout ticket de l'équipe) ; le serveur, lui, ne voit pas l'objet.

## Ce que voit le serveur

Des ids opaques (le projet, la prise de chaque ticket), des ids d'appareil, des heures (début et fin du bail, dernier renouvellement), le jeton et la version de chaque prise, le compteur des clés, qui a accès à quel projet. Jamais le titre, la clé, le préfixe, la colonne, l'objet d'un ticket dans le journal, l'identifiant du Kanban d'une entrée de la liste, ni le nom d'un membre (vérifié par un test qui lit la base du serveur).

## Verser aussi les docs de `.escouade/` (TS-7 avec TS-9)

- **Le rattachement ne verse rien** : ni les tickets, ni la spec, les décisions et les documents de `.escouade/` (page « Les docs d'un projet d'équipe »). Quand le Kanban d'un projet devient celui de l'équipe (partagé, rejoint par son identifiant ou par la liste), un toast le dit : « Acme API : son Kanban est celui de l'équipe. Ses tickets et ses docs d'avant restent sur ce poste : « Verser à l'équipe » les envoie, après un aperçu. » ; « Rattacher à l'équipe », dans « Projets de l'équipe », le dit aussi sous le bouton.
- **L'aperçu de « Verser à l'équipe »** a une troisième partie, « Spec, décisions et documents (.escouade/) » :
  - quand l'équipe n'a encore ni choisi ni importé de docs pour ce projet : le nombre de pages, de décisions et de documents, chaque fichier, et la case « Verser aussi la spec, les décisions et les documents », cochée ; le bouton le compte (« Verser 2 tickets et 1 fichier de docs », « Verser 1 fichier de docs » sans ticket) ;
  - sinon, pourquoi rien ne part : déjà sur le serveur d'équipe (ce poste les lit de là, rien de son dépôt ne part), gardées dans le dépôt par l'équipe, projet privé, ou aucune dans le dépôt.
  - Ce qui reste le dit aussi : les images, les fichiers joints, le compte, le modèle et le harness. La mémoire partagée (`.escouade/memory`) part avec les docs (voir « La mémoire d'équipe »).
- **Verser les docs** les importe comme le réglage « Sur le serveur d'équipe » (ids et numéros gardés, une fois pour l'équipe) ; l'écran nomme ensuite les fichiers restés dans `.escouade/` (« Retirer du dépôt » de l'onglet Docs les retire ; Escouade ne commite rien).
