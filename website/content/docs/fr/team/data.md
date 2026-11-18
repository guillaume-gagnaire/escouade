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
| La spec, les décisions, les documents | les pages, les décisions, les fichiers déposés | le texte extrait, l’index, les vecteurs | là, au choix de l’équipe |
| La mémoire partagée | les souvenirs et leur provenance signée ; ceux d’un agent proposés jusqu’à ce qu’un humain les garde | l’index et les vecteurs | là, avec les docs |

La spec, les décisions, les documents et la mémoire partent sur le serveur par un geste : « Sur le serveur d’équipe » dans les réglages du projet, ou « Verser à l’équipe » sur le Kanban. L’équipe peut aussi les garder dans le dépôt, comme avant. Les tickets d’un projet ne partent que si son Kanban est partagé avec l’équipe.

## La mémoire de l’équipe

La mémoire partagée d’un projet d’équipe (voir [la mémoire partagée](shared-memory)) est lue de tous les agents de l’équipe : un souvenir y est une consigne qui dure. Escouade la garde de ce qu’un agent trompé, ou un membre malveillant, y glisserait :

- **Chaque souvenir porte sa provenance**, signée par l’appareil qui l’écrit : le membre, l’agent (son ticket, son harness, son modèle), la date, écrit à la main ou par un agent, et si cet agent avait lu des données de tiers (une issue, une PR, une page web). Un souvenir dont la provenance ne tient pas à sa signature est refusé.
- **Un souvenir écrit par un agent naît proposé** : seuls tes agents le lisent, comme une donnée. Il devient celui de l’équipe quand tu le gardes, sur la carte du ticket à valider (« Ce ticket a laissé 2 souvenirs · Garder · Écarter ») ou dans Réglages › Projets › Équipe › Mémoire partagée. Un souvenir écrit à la main est accepté d’office. « Valider les souvenirs des agents » est allumé d’office ; l’équipe peut l’éteindre, ou l’imposer.
- **Les agents lisent comme une donnée**, jamais comme une consigne, un souvenir non validé ou écrit après des données de tiers.
- **Un administrateur met en quarantaine** ce qu’a écrit l’agent d’un ticket, ou un membre ou l’un de ses appareils depuis une date, après un aperçu de ce qui part : la règle vaut sur tous les postes, aussi pour ce qui arrive plus tard. L’historique reste au journal de l’équipe.
- **Un secret est refusé** à l’écriture : un jeton, une clé privée, l’adresse d’un webhook. Une adresse que le projet ne connaît pas est montrée à la validation.

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
