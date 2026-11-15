---
title: "Le client de l'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team/README.md","crates/escouade-team-crypto/README.md","crates/escouade-team-server/README.md"]
tickets: ["ESC-458","ESC-461"]
decisions: [168,173]
compared: null
---

# Le client de l'équipe

Ce que fait Escouade, sur chaque appareil, pour être dans une équipe d'un serveur d'équipe : la crate `escouade-team`, sur MLS (RFC 9420). Elle est branchée dans l'app : le trousseau de l'app, son réseau (proxy et certificats), le dossier `teams` des données. Créer une équipe, y inviter et y entrer : voir « Entrer dans une équipe ». Les espaces et les réglages de l'équipe viendront avec TS-4, la synchronisation des projets avec TS-5.

## L'équipe est un groupe MLS

- Chaque appareil d'un membre est une feuille du groupe. Le registre de l'équipe (genèse, rôles, clé de secours, clé de séquestre) est dans le groupe, et seul un commit le change.
- Avant de le fusionner, chaque appareil vérifie chaque commit : MLS d'abord, puis les règles de l'équipe. Un administrateur ajoute et retire ; un membre n'ajoute ou ne retire que ses propres appareils ; seule la clé de secours entre de l'extérieur. Le journal des membres est ce groupe vérifié.
- Le contenu de chaque objet est chiffré sous la clé de son projet et de son époque, et signé par son auteur.

## Les clés au trousseau

- Les clés de l'appareil sont créées sur l'appareil et ne le quittent jamais : sa clé pour le serveur, sa clé de feuille MLS, la clé de son état sur le disque. Elles vont au trousseau du système, le même que celui des comptes de l'app, sous un préfixe propre au dossier de données.
- Sans trousseau (Linux sans Secret Service), elles vont dans un fichier lisible de son seul propriétaire, et l'interface le dit, comme pour les comptes. Le fichier passe au trousseau dès que celui-ci les accepte.

## L'état sur le disque

- Un dossier par équipe, sous `teams` des données, et dans ce dossier l'état d'openmls et celui de l'appareil, en SQLite. Chaque valeur y est chiffrée sous la clé du trousseau : le fichier seul ne donne rien.
- Les clés des époques passées sont effacées du disque, journal compris, dès qu'une nouvelle époque commence.
- La plus haute époque vue et sa tête (le point de contrôle) sont gardées au trousseau, hors de l'état restaurable. Un état plus ancien qu'elles (une sauvegarde remise, une image disque) est refusé.

## Le serveur

- L'appareil entre par le jeton à usage unique de `init` (il crée l'équipe), par celui d'une invitation (dérivé du secret du lien), ou par celui de récupération. Il s'authentifie ensuite par le défi signé, et sa session se renouvelle seule.
- Il lit le flux d'équipe depuis le dernier numéro lu, écrit ses commits et ses messages, et garde dans une boîte d'envoi, sur le disque, ce que le serveur n'a pas encore pris.
- La connexion en direct revient seule après une coupure ou un redémarrage du serveur. À chaque retour, l'appareil relit les flux depuis leur dernier numéro. Elle s'arrête quand l'appareil est révoqué. L'app, elle, relit chaque équipe toutes les 3 s.
- Les fichiers (documents, images) partent chiffrés et signés.
- Tout passe par le proxy et les certificats de l'app (Réglages › Avancé › Réseau).

## Les commits concurrents

- L'appareil n'applique son commit qu'une fois que le serveur l'a pris. Si un autre commit tient déjà l'époque (le premier gagne), il abandonne le sien, applique le gagnant, puis refait le sien si c'est encore à faire.
- Un appareil n'a donc jamais deux époques qui divergent. Un commit envoyé sans réponse est retrouvé dans le flux à la lecture suivante.

## Retirer, révoquer, faire tourner les clés

- **Retirer un membre** : le serveur coupe d'abord tous ses appareils (sessions et connexions en direct), puis un commit le retire du groupe et du registre. La nouvelle époque lui échappe, et l'ancien contenu n'est pas rechiffré.
- **Révoquer un appareil volé** : de même, le serveur d'abord, puis le commit. Le serveur ne laisse que ses administrateurs couper un autre appareil. Un membre qui révoque son propre autre appareil s'en remet donc au commit seul.
- **Changer les clés de l'équipe** ouvre une nouvelle époque à la demande.
- **Les rôles du serveur suivent le registre** : un appareil administrateur du serveur corrige, à chaque nouvelle époque, le rôle sur le serveur des appareils dont le rôle au registre a changé.

## L'historique

- Un nouvel appareil lit tout le contenu d'avant son arrivée, par la chaîne des liens d'historique.
- Si le lien d'une époque manque (son auteur s'est arrêté avant de l'envoyer), tout membre qui tient cette époque et la précédente l'écrit.

## Le serveur qui ment

- **Retour en arrière** : l'appareil relit la dernière entrée lue avant de lire plus loin. Si elle a disparu, il refuse ; si elle a changé, il refuse aussi.
- **Fourche** : le serveur montre deux versions de l'équipe à deux membres. Elle se voit :
  - au premier objet signé qui passe d'une branche à l'autre ;
  - à un autre commit pour une époque déjà vue ;
  - aux balises de tête que chaque appareil signe à chaque nouvelle époque, et toutes les 6 h.

  L'appareil le dit alors (« Le serveur montre deux versions de l'équipe : contacte un administrateur »), s'arrête de lire le flux et n'envoie plus rien à l'équipe (voir « Entrer dans une équipe », L'alerte de fourche).
- **Pas de fausse alerte** : une balise ne compte que signée par un membre de l'époque courante (un membre retiré ne peut pas en lever une). Un objet doit être signé par un membre avant qu'on compare sa tête.
- Un serveur qui partitionne l'équipe pour toujours ne se voit qu'en comparant hors bande les six mots de la tête.
