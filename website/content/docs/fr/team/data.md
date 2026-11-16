# Où vont mes données ?

Ce qui part sur un serveur d’équipe, chiffré, ce qui n’y part jamais, ce qui reste sur ton poste, et pourquoi, sans serveur d’équipe, rien ne change.

## Sans serveur d’équipe

Rien ne change. Escouade garde tout sur ton disque, dans `~/.escouade/` (voir [les données locales](data)), et tes projets restent dans leurs dossiers. Les tickets partagés passent par le git de chacun (voir [le Kanban partagé](shared-board)). Le serveur d’équipe n’existe que si ton équipe l’installe et que tu la rejoins ; tant que tu n’es dans aucune équipe, l’app ne joint aucun serveur d’équipe.

Les statistiques d’usage, si tu les as acceptées, sont une autre affaire, sans lien avec le serveur d’équipe : voir la page [Confidentialité](/privacy/).

## Sur un serveur d’équipe

Le serveur d’équipe est installé par ton équipe, sur une machine qu’elle choisit (voir [Installer le serveur d’équipe](team/install)). Tout ce qui y part est chiffré de bout en bout sur ton poste : le serveur garde des octets qu’il ne sait pas lire, et ne voit que des métadonnées (qui écrit, quand, combien), détaillées dans [le guide du RSSI](team/security#ce-que-voit-le-serveur).

| Quoi | Sur le serveur, chiffré | Reste sur ton poste | Aujourd’hui |
|---|---|---|---|
| L’équipe | ses membres et leurs rôles, les appareils de chacun, les invitations, les enveloppes de la clé de secours et de la clé de séquestre | les clés de tes appareils, au trousseau ; le nom que tu donnes à l’équipe | là |
| Les réglages de l’équipe | son `.escouade.yml` d’équipe, signé, et son historique | tes accords aux clés sensibles et aux commandes proposées, les stratégies de ton poste | là |
| La liste des projets | pour chaque projet : son nom, sa couleur, son logo, ses dépôts (leurs adresses), sa branche par défaut, qui y a accès | le dossier du projet, ses worktrees, ses réglages sur ce poste | là |
| Les tickets d’un Kanban partagé avec l’équipe | clé, titre, description, critères et leur état, colonne, place, boucles, avancement, assignation, le nom de chaque appareil | images et fichiers joints, compte, modèle, harness, agent, branche, coût ; les tickets « Ne pas partager » | là |
| La prise d’un ticket | quel appareil le tient, jusqu’à quand, son jeton, le compteur des clés : en clair, sans dire quel ticket c’est | | là |
| La spec, les décisions, les documents | les pages, les décisions, les fichiers déposés | le texte extrait, l’index, les vecteurs | à venir |
| La mémoire partagée | les souvenirs et leur provenance | l’index et les vecteurs | à venir |

Tant que les lignes « à venir » ne sont pas là, la spec, les décisions, les documents et la mémoire de tes projets restent où ils sont aujourd’hui : sur ton poste et dans ton dépôt. Les tickets d’un projet ne partent que si son Kanban est partagé avec l’équipe.

## Ce qui n’y part jamais

- **Les comptes des harness** : ta connexion à Claude Code, Codex et les autres, tes quotas. Le serveur d’équipe n’en voit aucun ; chaque membre utilise les siens.
- **Les secrets et les jetons** : ceux des services connectés, des hébergeurs de code, des clés d’API. Ils restent dans le trousseau de ton poste.
- **Les conversations des agents** : leurs journaux restent dans `~/.escouade/conversations/`.
- **Le code** : il reste chez ton hébergeur de code (GitHub, GitLab…). Cloner un projet de l’équipe passe par tes propres comptes git ; le serveur d’équipe ne donne aucun accès git et ne garde aucun identifiant.
- **Tes projets personnels** : un projet qui n’est pas un projet de l’équipe n’y envoie rien.

## Ce qui reste sur ton poste

Dans `~/.escouade/teams/` :

- `teams.json` : les équipes de cet appareil, le nom que tu leur donnes, leur serveur, l’état de la clé de secours, une alerte ;
- un dossier par équipe, avec son état (`state.sqlite3`), chiffré sous une clé du trousseau ;
- dans ce dossier, la copie de chaque projet de l’équipe que suit ce poste (`projects/<projet>.sqlite3`), lisible hors ligne. Elle n’est pas chiffrée, comme le Kanban local d’aujourd’hui : le chiffrement de ton disque la protège ;
- `logos/` : les logos des projets de l’équipe.

Les clés de tes appareils sont dans le trousseau du système (le gestionnaire d’identifiants de Windows, le trousseau de macOS). Sous Linux, où Escouade n’a pas de trousseau, elles vont dans un fichier lisible de toi seul, et la carte de l’équipe le dit.

## Qui peut lire quoi

- **Les membres de l’équipe** lisent ce qui est à l’équipe, et les projets auxquels ils ont accès.
- **Le serveur et son hébergeur** ne lisent rien du contenu ; ils voient l’activité.
- **L’organisation**, si l’équipe a une clé de séquestre, peut tout lire. Chaque membre le voit sur la carte de l’équipe : « Cette équipe a une clé de séquestre : l’organisation peut lire son contenu. »
- **Un membre parti** ne lit plus rien de ce qui s’écrit après son départ, mais garde ce qu’il a déjà lu.

## Partir d’une équipe

Sur la carte de l’équipe, « Retirer de cet appareil » efface de ce poste son état et ses clés, après confirmation. Ce que tu as écrit dans l’équipe y reste : c’est à l’équipe. Un administrateur retire ensuite ton appareil du groupe.
