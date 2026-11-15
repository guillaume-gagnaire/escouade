---
title: "La synchronisation des projets"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team/README.md","crates/escouade-team-crypto/README.md","crates/escouade-team-server/README.md"]
tickets: ["ESC-466"]
decisions: [168,173,178]
compared: null
---

# La synchronisation des projets

Chaque projet d'une équipe est un **journal d'opérations** : « créer le ticket X », « changer le titre de X », « déplacer X », « supprimer X », « écrire la page P, version 4 ». Chaque appareil le rejoue dans sa **copie locale**, que l'interface lit : jamais le serveur. Tout se lit hors ligne, et tout s'écrit hors ligne, sauf la prise d'un ticket (TS-6). Le Kanban d'équipe (TS-6) et la spec sur le serveur (TS-9) s'appuient dessus ; ce ticket n'a pas d'écran.

## Un projet de toute l'équipe, ou de quelques membres

- **De toute l'équipe** : ses opérations sont chiffrées sous la clé du projet de l'époque de l'équipe. Sa première opération dit qu'il est de toute l'équipe : un appareil qui le suit le sait avant d'y écrire.
- **Gardé à quelques membres** (par un administrateur) : il a **son propre groupe MLS**, dont les feuilles sont les appareils de ces membres seuls. Un membre de l'équipe hors de ce groupe ne lit rien du projet.
  - Un appareil y entre par son KeyPackage, qu'il publie dans le flux de l'équipe ; il n'y entre que si c'est un appareil de l'équipe et que son membre a accès.
  - Tout appareil du projet tient son groupe au pas de l'équipe : il ajoute les nouveaux appareils de ses membres, il retire ceux qui ont quitté l'équipe.
  - Seul un administrateur de l'équipe change qui a accès. Une invitation peut donner l'accès à des projets : le membre invité y entre une fois dans l'équipe.
  - Un appareil ajouté plus tard lit ce qui a été écrit avant lui, par les liens d'historique du groupe du projet.

## Une opération

- Elle porte son auteur (le membre, l'appareil, la clé de sa feuille), sa place dans la chaîne de son appareil (son rang et l'empreinte de l'opération d'avant du même appareil), la tête du journal des membres que son auteur connaissait, et son horodatage.
- Elle est signée par la feuille de son auteur, puis chiffrée sous la clé de son époque.
- Le serveur lui donne un numéro d'ordre par projet, la garde et la diffuse en direct.
- **Ce que voit le serveur** : l'époque, la taille, l'heure et l'appareil qui l'envoie. Jamais le projet en clair, l'objet, le champ, la valeur, la feuille de l'auteur, sa chaîne ou son horodatage.

## La fusion, champ par champ

- **Le dernier qui écrit gagne**, champ par champ, sous une **horloge hybride** (HLC) : l'heure de l'appareil, un compteur, puis l'appareil. Une écriture faite après en avoir vu une autre gagne sur elle, quelles que soient les horloges. Une horloge trop en avance ne fait pas avancer celle des autres de plus d'une heure.
- **Le conflit gardé** : deux écritures d'un champ faites depuis la même valeur sont concurrentes. Celle qui perd est gardée, pour que TS-6 la montre (« Bob a changé le titre en même temps · Reprendre le mien »).
- **Une suppression est définitive**, quoi qu'on écrive ensuite dans l'objet.
- **Une page de spec va en versions entières**. Deux versions faites depuis la même base sont un conflit, gardé ; la fusion à trois voies viendra avec TS-9.
- Quel que soit l'ordre d'arrivée des opérations, toutes les copies finissent pareilles.

## La copie locale

- SQLite, dans le dossier `teams` des données, un fichier par projet de chaque équipe. Elle est en clair sur le disque, comme le Kanban local de `.escouade/` ; les clés et l'état MLS restent dans l'état chiffré de l'équipe.
- Elle garde le journal et se reconstruit depuis lui (et depuis le dernier instantané).
- **Reprise après coupure** : un appareil reprend la lecture au dernier numéro vu.

## La file d'envoi hors ligne

- Ce que l'appareil écrit s'applique à sa copie tout de suite et attend dans une file, dans son ordre. Les opérations partent au retour du réseau, une à une, chacune chiffrée à l'époque du moment : un membre parti entre-temps ne lit pas ce qui était en attente.
- Une opération dont la réponse s'est perdue est renvoyée, et comptée une fois.
- **L'état « Hors ligne · 3 changements en attente »** est donné par le cœur (`team_sync_status`) ; TS-4 et TS-6 l'affichent.

## Le serveur qui ment

- **Rejouer** une opération déjà lue, **en réordonner** deux d'un même appareil, **en retirer** une : la chaîne de l'appareil le montre.
- **Taire** une opération que l'appareil a envoyée : l'appareil attend chacune des siennes à son numéro.
- **Forger** une opération, ou **changer sa signature** : elle ne s'ouvre pas, ou n'est signée par aucune feuille de son époque.
- **Revenir en arrière**, ou **changer une entrée déjà lue** : l'appareil relit la dernière entrée lue avant de lire plus loin.
- **Une opération qui cite une autre tête** du journal des membres : c'est la fourche (voir « Le client de l'équipe »).

Dans tous ces cas, l'appareil s'arrête et le dit, comme pour la fourche : « Le serveur a rejoué, réordonné, retiré ou forgé une opération d'un projet de l'équipe », et il n'envoie plus rien à l'équipe tant qu'une lecture n'a pas réussi.

## Les instantanés

- Au-delà de 10 000 entrées depuis le dernier, un appareil de l'équipe écrit un **instantané** chiffré et signé de l'état du projet, avec la chaîne de chaque appareil. Le serveur oublie alors les opérations d'avant ; il garde les commits du groupe du projet.
- Un nouvel appareil, ou un appareil resté absent au-delà, part du dernier instantané. Il en vérifie la signature et la tête, et vérifie que chaque chaîne va au moins aussi loin que ce qu'il a lu. Un instantané forgé est refusé.

## La rotation

Après le départ d'un membre de l'équipe, les nouvelles opérations sont à l'époque suivante : celle de l'équipe pour un projet de toute l'équipe, celle de son groupe pour un projet gardé à quelques membres. L'ancien membre ne lit plus rien de nouveau. L'historique n'est pas rechiffré (C8-A).

## Le direct

L'app tient pour chaque équipe sa connexion en direct au serveur. Une entrée nouvelle de l'équipe ou d'un projet, une connexion revenue ou des trames perdues déclenchent une lecture de l'équipe, puis de ses projets suivis, en moins d'une seconde. La relecture de l'app toutes les 15 s ne sert plus que si la connexion en direct ne passe pas.
