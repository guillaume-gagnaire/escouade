---
title: "Entrer dans une équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team/README.md","crates/escouade-team-server/README.md","crates/escouade-team-crypto/README.md"]
tickets: ["ESC-461","ESC-633"]
decisions: [173,286]
compared: null
---

# Entrer dans une équipe

Créer une équipe sur un serveur d'équipe, y inviter, y ajouter ses autres appareils, la récupérer. Tout est chiffré de bout en bout (seul mode) : le serveur ne voit jamais un secret d'invitation ni une clé. Les espaces et les réglages de l'équipe viendront avec TS-4 : en attendant, ces écrans s'ouvrent par Réglages › Avancé › « Serveur d'équipe (aperçu) » › « Ouvrir les équipes… », par « Équipes… » du menu « + » de la barre des projets quand il montre un menu (des projets fermés), et par un lien `escouade://team/join`.

## Où vivent les équipes sur l'appareil

- Les clés de chaque appareil vont au trousseau du système, le même que celui des comptes de l'app, sous un préfixe propre au dossier de données : deux dossiers d'une même machine (deux utilisateurs, les tests) ne partagent jamais leurs clés. Sans trousseau, elles vont dans un fichier lisible du seul propriétaire, et la fenêtre le dit.
- L'état de chaque équipe est dans le dossier `teams` des données, avec `teams.json` : le nom donné à l'équipe sur cet appareil, son serveur, la clé de secours confirmée ou non, l'alerte.
- Tout passe par le proxy et les certificats de l'app (Réglages › Avancé › Réseau), la connexion en direct comme les requêtes.
- Un serveur se joint en https ; en http seulement sur cette machine (`127.0.0.1`, `localhost`).
- L'app relit chaque équipe toutes les 3 s et fait, pour les autres, ce que fait un appareil en ligne (voir « Un administrateur en ligne »).

## Créer une équipe

- L'adresse du serveur, le jeton qu'a affiché `escouade-team-server init` (il ne sert qu'une fois), un nom.
- **La clé de secours**, obligatoire : 24 mots, montrés une fois, en grille numérotée. « J'ai noté les 24 mots », puis le propriétaire retape trois mots tirés au hasard (leur numéro donné). Tant qu'il ne l'a pas fait, la carte de l'équipe le dit et propose « Nouvelle clé de secours » : les mots ne sont gardés qu'en mémoire, une app arrêtée avant les a perdus.
- **L'empreinte de l'équipe** : six mots, ceux de sa genèse, les mêmes chez chaque membre. Chaque invitation les porte.
- **Un second administrateur** est recommandé tant qu'il n'y en a qu'un : l'écran de fin et la carte de l'équipe le disent.
- L'identifiant de l'équipe est montré à la création : l'hébergeur en a besoin pour un jeton de récupération.

## La clé de séquestre (en option)

- À la création : « Ajouter une clé de séquestre de l'organisation », qui la détient, et sa moitié publique, collée ou faite sur place (« Créer la paire ici » : la moitié secrète est montrée une fois, à remettre à son détenteur).
- Tout le contenu lui est aussi chiffré : l'organisation peut toujours le lire. **Chaque membre le voit**, sur la carte de l'équipe et en entrant : « Cette équipe a une clé de séquestre : l'organisation peut lire son contenu. Détenteur : … ».

## Inviter

- Un administrateur choisit le rôle (membre ou administrateur) et la durée (24 h, **72 h par défaut**, 7 jours). Un lien sert **une seule fois**.
- Le lien : `escouade://team/join/<serveur>/<id>#s=<secret>&t=<empreinte>` (et `&m=<membre>` pour un autre appareil d'un membre). La fenêtre le donne à copier, et en QR code ; elle dit que l'invité le clique ou, si sa messagerie ne le rend pas cliquable, le colle dans « Rejoindre par un lien ».
- **Le secret ne passe jamais par une page web** : il n'y a aucune forme `https://` du lien qui le porte. HTTP n'envoie pas ce qui suit le `#`, mais une page servie par le serveur d'équipe le lirait dans le navigateur, et un serveur piraté entrerait alors à la place de l'invité. Une page statique ailleurs (escouade.dev) ferait d'un autre site un tiers de confiance : écartée aussi.
- Le jeton du serveur que l'invité utilise est dérivé du secret : le serveur n'en garde que l'empreinte, et ne laisse entrer qu'une fois.
- Les invitations de l'équipe sont listées sur sa carte : ouverte (jusqu'à quand), en attente d'un administrateur, utilisée, expirée.

## Les liens de l'ancienne forme

- Un lien `https://<serveur>/join/<id>#…` (d'avant la 2.3, qui ouvrait une page du serveur) est **refusé**, collé comme ouvert : « Ce lien d'invitation a l'ancienne forme (https://…/join/…) et ne sert plus : demande un nouveau lien à qui t'invite. Les liens d'Escouade commencent maintenant par escouade://. » Rien ne part.
- L'appareil qui a fait une invitation avant ce changement **n'admet plus personne avec elle**, MAC valide ou non : elle est listée expirée. Son lien a pu passer par la page du serveur.
- Ouvert dans un navigateur, un tel lien mène à la page `/join/<id>` du serveur, qui n'est plus qu'un texte fixe sans script (voir « Le serveur d'équipe »).

## Rejoindre

- « Rejoindre par un lien », ou un lien `escouade://team/join` ouvert (un clic dans une messagerie, un navigateur, un QR code scanné) : la fenêtre s'ouvre sur « Rejoindre une équipe », le lien rempli, et montre le serveur et les six mots de l'équipe. **Un lien venu d'ailleurs ne fait rien d'autre** : aucune requête ne part avant « Rejoindre ». **L'invité vérifie les six mots** avec qui l'invite : s'ils diffèrent, il n'entre pas. Rejoindre reste son geste.
- Le schéma `escouade://` est celui de l'app sur les trois systèmes : l'installeur l'enregistre sous Windows et Linux (`.deb`, `.rpm`), le bundle sous macOS, et l'app elle-même pour une AppImage.
- La demande part ; un administrateur en ligne le fait entrer (« Un administrateur en ligne te fait entrer : cette fenêtre le dira »). Le groupe rejoint doit avoir l'empreinte du lien.
- **Une invitation déjà utilisée** : « Cette invitation a déjà servi : demande un nouveau lien à son administrateur, qui est prévenu. » Sur la carte de l'équipe de l'administrateur, l'invitation dit que quelqu'un l'a réutilisée après son invité, et quand : le lien a peut-être fuité.
- **Une invitation expirée** est refusée par le serveur, et un administrateur en ligne après son terme ne fait plus entrer la demande.

## Un administrateur en ligne

À chaque lecture, l'appareil :
- fait entrer les demandes des invitations qu'il a faites (un Welcome MLS), sauf celles d'une invitation de l'ancienne forme ;
- s'il est administrateur du serveur, enregistre le jeton qu'un membre a demandé pour son propre appareil (voir plus bas) ;
- tient **les rôles du serveur au pas du registre** : un administrateur nommé après son inscription le devient aussi sur le serveur, un administrateur redevenu membre aussi ; le dernier administrateur du serveur reste ;
- lit les seconds usages de ses invitations.

## Ajouter un de ses appareils

- « Ajouter un appareil… », onglet **QR code** : le lien d'un appareil du même membre (valable une heure), à scanner ou à coller dans « Rejoindre par un lien » sur l'autre appareil. L'appareil du membre le fait entrer lui-même.
- Onglet **Code court**, sans caméra : 8 caractères (`K7Q4-M2XZ`), à taper sur l'autre appareil dans « Ajouter cet appareil par un code », avec l'adresse du serveur. Un code a **un seul essai** et 10 minutes : un mauvais code le rend inutilisable (« Un mauvais code a été tapé : ce code ne sert plus »), il en faut un nouveau. Le code passe par un échange de clés authentifié par mot de passe (SPAKE2) : le serveur ne peut pas l'essayer hors ligne.
- Un membre qui n'est pas administrateur du serveur : le jeton de son nouvel appareil est demandé dans l'équipe, signé par son appareil, et un administrateur en ligne l'enregistre. En attendant, la fenêtre le dit (« En attente d'un administrateur »).
- Les O se lisent 0, les I et L se lisent 1, majuscules et tirets indifférents.

## Récupérer une équipe

- Quand plus aucun administrateur n'a d'appareil : « Récupérer une équipe », avec l'adresse du serveur, un jeton que l'hébergeur donne (`escouade-team-server recovery-token --team <identifiant>`) et les 24 mots.
- Cet appareil revient dans l'équipe, administrateur (sur le serveur aussi), et l'équipe reçoit **une nouvelle clé de secours**, montrée une fois et confirmée comme à la création. L'ancienne n'ouvre plus rien.
- Des mots faux sont refusés avant d'utiliser le jeton.

## L'alerte de fourche

- Si le serveur montre deux versions de l'équipe (une fourche), revient en arrière sur ce que l'appareil avait lu, ou change une entrée lue, la carte de l'équipe dit : « Le serveur montre deux versions de l'équipe : contacte un administrateur. », et pourquoi.
- **Ce que l'app bloque** : elle ne lit plus le flux de l'équipe et n'y envoie plus rien depuis cet appareil : ni invitation, ni entrée d'un invité, ni nouvel appareil, ni nouvelle clé de secours.
- **Ce que l'utilisateur peut faire** : comparer avec un administrateur les six mots de la tête de l'équipe (s'ils diffèrent, le serveur ment à l'un des deux) ; « Réessayer » (une lecture qui passe lève l'alerte) ; « Retirer l'équipe de cet appareil » (ses clés sont effacées ; un administrateur retire l'appareil du groupe).

## Retirer une équipe de cet appareil

Sur la carte de l'équipe, après confirmation : son état et ses clés sont effacés de cet appareil. Le groupe garde sa feuille jusqu'à ce qu'un administrateur la retire (TS-4).
