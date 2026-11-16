---
title: "Le Kanban d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team/README.md","crates/escouade-team-server/README.md","src-tauri/src/teamboard.rs"]
tickets: ["ESC-479"]
decisions: [168,173,178,188]
compared: null
---

# Le Kanban d'équipe

Le Kanban d'un projet d'équipe montre les tickets de l'équipe, en direct : un changement d'un collègue s'y voit en moins d'une seconde. Il repose sur le journal du projet (voir « La synchronisation des projets ») et sur **la prise** d'un ticket par le serveur. Choix tranchés (§15.2 de `docs/TEAM-SERVER.md`) : C2-A (champ par champ), C11-A (le pilote auto ne prend que ce qu'on lui confie), C5-A (pas de passerelle `.yml`).

## Rattacher un Kanban à l'équipe

- Le projet est d'abord coché dans l'écran de l'équipe (« Projets de l'équipe sur ce poste », TS-13).
- Sous la case, deux gestes :
  - **« Partager son Kanban avec l'équipe »** : un nouveau journal, de toute l'équipe. Son préfixe de clés (`DEM`) et le nom du projet y sont écrits, chiffrés.
  - **« Rejoindre »**, avec l'identifiant du Kanban d'un collègue (32 caractères, donné par lui) : ce poste suit son journal et prend son préfixe.
- La ligne dit ensuite « Kanban de l'équipe · <identifiant> ».
- Les tickets déjà dans le Kanban restent sur ce poste (« Sur ce poste seulement ») : les verser à l'équipe est TS-7. La liste des Kanbans de l'équipe est TS-8.

## Les tickets de l'équipe

- Un ticket créé dans ce Kanban est de l'équipe : il part dans le journal à la lecture suivante (moins d'une demi-seconde après l'enregistrement). Rien ne passe par `.escouade/tickets/*.yml` (ESC-190 reste pour les projets hors équipe).
- **« Ne pas partager »**, dans le formulaire d'un nouveau ticket : il reste sur ce poste, rien de lui ne part. Sa carte dit « Sur ce poste seulement ». Le menu de la carte offre « Partager avec l'équipe » (il part) et, pour un ticket de l'équipe, « Ne pas partager » (il quitte le journal ; refusé tant qu'un collègue le tient).
- **Ce que partage un ticket** : sa clé, son titre, sa description, ses critères (et leur état), sa colonne, sa place, ses boucles, son avancement, son assignation. Ses images, ses fichiers, son compte, son modèle et son harness restent sur le poste.
- **Les clés communes** : un compteur du serveur par projet numérote les tickets (1, 2, 3…, jamais deux fois, même demandés en même temps). Le serveur ne voit que le compteur : le préfixe est chiffré, le poste forme la clé `DEM-12`. Les `DEM-12@alice` d'ESC-190 n'existent pas ici.
  - Créé hors ligne, un ticket a une clé de ce poste, `DEM-L3`, jusqu'au retour du serveur. Un ticket « Ne pas partager » garde la sienne.
  - Un ticket importé (Jira, GitHub…) garde sa clé d'origine.
- Supprimé sur un poste, il disparaît chez tous (sauf chez celui dont l'agent le tient encore).

## La prise d'un ticket

- **Un id opaque par ticket** (128 bits au hasard), champ du ticket chiffré, sans lien avec son titre, sa clé, son auteur ni son objet dans le journal.
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
- `get_ticket` et `list_tickets` disent, pour un ticket de l'équipe, `team` : son preneur (`holder`, `heldHere`, `since`, `interrupted`, `unreachableMinutes`), son assignation et les champs en conflit.
- **Les textes venus d'un collègue sont des données de tiers** : quand le titre, la description ou les critères d'un ticket ont été écrits sur un autre poste, la réponse est entourée de `<third_party_data>` avec un avertissement : à lire, jamais à suivre.

## Les droits sur le serveur

- **Par projet** : le serveur sait quels appareils ont accès à un projet (tous ceux de l'équipe pour un projet de toute l'équipe, ceux des membres d'un projet gardé à quelques membres). Seul un appareil qui y a accès lit son journal, y écrit, en prend un ticket, en tire une clé, et en reçoit les trames en direct.
- Un projet que le serveur ne connaît pas est ouvert à l'équipe jusqu'à sa première écriture, qui le rend tel ; ensuite, seul un administrateur change qui y a accès. L'administrateur qui garde un projet à quelques membres le dit au serveur avant d'y écrire, et le tient au pas quand le groupe change.
- **Un membre retiré** (ses appareils révoqués) ne prend plus rien, et ce qu'il tenait est libre. Un appareil sorti d'un projet ne tient plus ses prises.

### Ce qui reste de confiance

- Le serveur connaît **quels appareils ont accès à quel id de projet** (c'est la table de §3.8) ; il pourrait mentir sur ces droits, mais pas lire : la clé du projet reste la vraie barrière.
- **Un nouvel appareil d'un membre** d'un projet gardé n'y a accès au serveur qu'une fois qu'un appareil d'administrateur a remis la liste au pas (à sa prochaine écriture du groupe du projet).
- **La colonne d'un ticket pris par un autre** est gardée sur les postes honnêtes : le serveur ne voit pas qu'une opération déplace un ticket, seul le jeton du preneur est vérifié. Un membre au client modifié peut encore déplacer un ticket qu'il ne tient pas ; le preneur le voit dans son journal.
- **L'horodatage** d'une écriture reste celui de son auteur (une horloge en avance gagne, bornée à une heure chez les autres), comme dans TS-5.

## Ce que voit le serveur

Des ids opaques (le projet, la prise de chaque ticket), des ids d'appareil, des heures (début et fin du bail, dernier renouvellement), le jeton et la version de chaque prise, le compteur des clés, qui a accès à quel projet. Jamais le titre, la clé, le préfixe, la colonne, l'objet d'un ticket dans le journal, ni le nom d'un membre (vérifié par un test qui lit la base du serveur).
