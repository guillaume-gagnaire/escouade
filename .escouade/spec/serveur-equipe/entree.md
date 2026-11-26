---
title: "Entrer dans une équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-SERVER.md","crates/escouade-team/README.md","crates/escouade-team-server/README.md","crates/escouade-team-crypto/README.md","Revue de sécurité de la 2.3 (ESC-625), SR2-08 : la vérification hors bande à l'admission","Revue de sécurité de la 2.3 (ESC-625), SR2-05 : le relais d'appairage"]
tickets: ["ESC-461","ESC-632","ESC-633","ESC-634","ESC-639","ESC-606"]
decisions: [173,286,288,329,332,342]
compared: null
---

# Entrer dans une équipe

Créer une équipe sur un serveur d'équipe, y inviter, y ajouter ses autres appareils, la récupérer. Tout est chiffré de bout en bout (seul mode) : le serveur ne voit jamais un secret d'invitation ni une clé. Les espaces et les réglages de l'équipe viendront avec TS-4 : en attendant, ces écrans s'ouvrent par Réglages › Avancé › « Serveur d'équipe (aperçu) » › « Ouvrir les équipes… », par « Équipes… » du menu « + » de la barre des projets quand il montre un menu (des projets fermés), et par un lien `escouade://team/join`.

## Où vivent les équipes sur l'appareil

- Les clés de chaque appareil vont au trousseau du système, le même que celui des comptes de l'app, sous un préfixe propre au dossier de données : deux dossiers d'une même machine (deux utilisateurs, les tests) ne partagent jamais leurs clés. Sans trousseau, elles vont dans un fichier lisible du seul propriétaire, et la fenêtre le dit.
- L'état de chaque équipe est dans le dossier `teams` des données, avec `teams.json` : le nom donné à l'équipe sur cet appareil, son serveur, la clé de secours confirmée ou non, l'alerte.
- Tout passe par le proxy et les certificats de l'app (Réglages › Avancé › Réseau), la connexion en direct comme les requêtes.
- Un serveur se joint en https ; en http seulement sur cette machine (`127.0.0.1`, `localhost`).
- L'app relit chaque équipe toutes les 3 s et fait, pour les autres, ce que fait un appareil en ligne (voir « Un appareil en ligne »).

## Créer une équipe

- L'adresse du serveur, le jeton qu'a affiché `escouade-team-server init` (il ne sert qu'une fois), un nom.
- **La clé de secours**, obligatoire : 24 mots, montrés une fois, en grille numérotée, et sous eux **les six mots de l'équipe** (son empreinte), à noter sur la même feuille de secours : la récupération demande les deux. « J'ai noté les 24 mots et les six mots de l'équipe », puis le propriétaire retape trois des 24 mots tirés au hasard (leur numéro donné). Tant qu'il ne l'a pas fait, la carte de l'équipe le dit et propose « Nouvelle clé de secours » : les mots ne sont gardés qu'en mémoire, une app arrêtée avant les a perdus. « Nouvelle clé de secours » montre de même une feuille neuve.
- **L'empreinte de l'équipe** : six mots, ceux de sa genèse, les mêmes chez chaque membre. Chaque invitation les porte, et la feuille de secours aussi.
- **Un second administrateur** est recommandé tant qu'il n'y en a qu'un : l'écran de fin et la carte de l'équipe le disent.
- L'identifiant de l'équipe est montré à la création : l'hébergeur en a besoin pour un jeton de récupération.

## La clé de séquestre (en option)

- À la création : « Ajouter une clé de séquestre de l'organisation », qui la détient, et sa moitié pour l'équipe (`escouade-escrow-team:2:…`), collée ou faite sur place (« Créer la paire ici » : la moitié secrète est montrée une fois, à remettre à son détenteur). La moitié pour l'équipe porte la clé publique et l'ancre qui signe les enveloppes de séquestre : elle ne se colle que dans Escouade. Une moitié publique d'avant (`escouade-escrow-public:1:`) est refusée : il faut refaire la paire.
- Tout le contenu lui est aussi chiffré : l'organisation peut toujours le lire. **Chaque membre le voit**, sur la carte de l'équipe et en entrant : « Cette équipe a une clé de séquestre : l'organisation peut lire son contenu. Détenteur : … ».
- L'organisation ne lit que ce qu'un administrateur de l'équipe a scellé : chaque enveloppe de séquestre porte l'empreinte de l'équipe et le registre de son époque, et elle est signée par la feuille d'un administrateur et par l'ancre du séquestre (que redonne sa moitié secrète).

## Inviter

- Un administrateur choisit le rôle (membre ou administrateur) et la durée (24 h, **72 h par défaut**, 7 jours). Un lien sert **une seule fois**.
- Le lien : `escouade://team/join/<serveur>/<id>#s=<secret>&t=<empreinte>` (et `&m=<membre>` pour un autre appareil d'un membre). La fenêtre le donne à copier, et en QR code ; elle dit que l'invité le clique ou, si sa messagerie ne le rend pas cliquable, le colle dans « Rejoindre par un lien ».
- **Le secret ne passe jamais par une page web** : il n'y a aucune forme `https://` du lien qui le porte. HTTP n'envoie pas ce qui suit le `#`, mais une page servie par le serveur d'équipe le lirait dans le navigateur, et un serveur piraté entrerait alors à la place de l'invité. Une page statique ailleurs (escouade.dev) ferait d'un autre site un tiers de confiance : écartée aussi.
- Le jeton du serveur que l'invité utilise est dérivé du secret : le serveur n'en garde que l'empreinte, et ne laisse entrer qu'une fois.
- **Le lien seul ne fait entrer personne** : chaque demande est confirmée par les six mots de son code (voir « Le code de six mots »). Un lien qui fuit (un courriel transféré, une messagerie, une capture d'écran) ne donne donc à qui l'a qu'une demande que la personne qui invite refuse.
- Après « Créer le lien », l'écran d'invitation reste ouvert sur les demandes qui arrivent, avec leurs six mots.
- Les invitations de l'équipe sont listées sur sa carte : ouverte (jusqu'à quand), en attente d'un administrateur, utilisée, expirée, annulée (« Annulée : les mots différaient »).

## Le code de six mots

À l'admission de tout appareil (un nouveau membre, ou un autre appareil d'un membre, par le QR code comme par le code court) :

- **Sur l'écran de l'invité** : dès sa demande partie, « Code de cet appareil » et six mots, sur l'écran « Rejoindre » et sur la carte de l'équipe en attente : « Dis ces six mots à la personne qui t'invite : elle les compare à son écran avant de te faire entrer. S'ils diffèrent, elle refuse la demande. » Ils restent tant que l'appareil attend, même après un redémarrage, et disparaissent une fois entré.
- **Sur l'écran de qui invite** : « Demandes d'entrée », sur la carte de l'équipe, sur l'écran d'invitation et sur celui d'« Ajouter un appareil », chaque demande avec ce qu'elle fait entrer (« Nouveau membre · Membre », « Un de tes appareils ») et « Code de la demande », six mots. « Avant de faire entrer qui que ce soit, compare ces six mots avec ceux qu'affiche l'écran de l'invité, de vive voix ou en personne, jamais par le message qui portait le lien. »
- **« Les mots sont les mêmes »** fait entrer l'invité (le Welcome part), avec les six mots montrés : si la demande a changé entre-temps, ses mots ne sont plus ceux-là et rien n'entre (« Ces six mots ne sont plus ceux de la demande : rien n'est admis… »).
- **« Les mots diffèrent »** (ou l'invité n'en voit aucun : on lui a dit « Cette invitation a déjà servi ») : la demande est refusée, **l'invitation annulée** (plus personne n'entre avec, quels que soient les mots), l'appareil qui a demandé est coupé du serveur quand celui qui refuse en est administrateur, et **les administrateurs sont prévenus** : « Invitations annulées » sur la carte de l'équipe de chacun, avec quand et par qui (« Par cet appareil », « Par un administrateur », « Par un membre, pour un de ses appareils »). Un membre ne voit que les siennes.
- **Ce que garantissent les six mots** : ils sont tirés de l'identifiant de l'invitation et du KeyPackage MLS de l'invité tel qu'il est parti, signé par la clé de sa feuille, que l'appareil de l'invité fait et ne quitte pas. L'appareil de qui invite vérifie la signature et la preuve du secret de l'invitation, puis calcule les mots lui-même. Le serveur ne peut ni les calculer à la place d'un appareil ni mettre un autre KeyPackage à la place de celui de l'invité sans que les deux écrans montrent des mots différents ; qui a le lien en fait une demande à lui, dont les mots ne sont pas ceux de l'écran de l'invité.
- **Aucune admission sans confirmation** : l'appareil en ligne liste les demandes et n'en fait entrer aucune de lui-même. La confirmation est un geste sur l'ordinateur : ni le MCP, ni l'API distante (le téléphone, un client à distance : la commande y est refusée), ni l'Overseer d'équipe (qui ne répond pas aux invitations) n'admettent.
- La récupération (« Récupérer une équipe ») fait entrer le nouvel appareil de cette machine sans personne pour comparer : les deux écrans sont le même, l'app lit elle-même les six mots du nouvel appareil.

## Les liens de l'ancienne forme

- Un lien `https://<serveur>/join/<id>#…` (d'avant la 2.3, qui ouvrait une page du serveur) est **refusé**, collé comme ouvert : « Ce lien d'invitation a l'ancienne forme (https://…/join/…) et ne sert plus : demande un nouveau lien à qui t'invite. Les liens d'Escouade commencent maintenant par escouade://. » Rien ne part.
- L'appareil qui a fait une invitation avant ce changement **n'admet plus personne avec elle**, MAC valide ou non : elle est listée expirée. Son lien a pu passer par la page du serveur.
- Ouvert dans un navigateur, un tel lien mène à la page `/join/<id>` du serveur, qui n'est plus qu'un texte fixe sans script (voir « Le serveur d'équipe »).

## Rejoindre

- « Rejoindre par un lien », ou un lien `escouade://team/join` ouvert (un clic dans une messagerie, un navigateur, un QR code scanné) : la fenêtre s'ouvre sur « Rejoindre une équipe », le lien rempli, et montre le serveur et les six mots de l'équipe. **Un lien venu d'ailleurs ne fait rien d'autre** : aucune requête ne part avant « Rejoindre ». **L'invité vérifie les six mots de l'équipe** avec qui l'invite : s'ils diffèrent, il n'entre pas. Rejoindre reste son geste.
- Le schéma `escouade://` est celui de l'app sur les trois systèmes : l'installeur l'enregistre sous Windows et Linux (`.deb`, `.rpm`), le bundle sous macOS, et l'app elle-même pour une AppImage.
- La demande part ; l'écran montre le code de six mots de cet appareil (voir « Le code de six mots ») : « La personne qui t'invite compare tes six mots aux siens, puis te fait entrer : cette fenêtre le dira. » Le groupe rejoint doit avoir l'empreinte du lien.
- **Une invitation déjà utilisée** : « Cette invitation a déjà servi : demande un nouveau lien à son administrateur, qui est prévenu. » Sur la carte de l'équipe de l'administrateur, l'invitation dit que quelqu'un l'a réutilisée après son invité, et quand : le lien a peut-être fuité.
- **Une invitation expirée** est refusée par le serveur, et sa demande n'est plus montrée à qui l'a faite après son terme.

## Un appareil en ligne

À chaque lecture, l'appareil :
- **liste les demandes** des invitations qu'il a faites, vérifiées (le secret de l'invitation, la signature du KeyPackage), chacune avec ses six mots, pour la personne qui invite ; il n'en fait entrer aucune de lui-même. Une demande d'une invitation utilisée, expirée, annulée ou de l'ancienne forme, ou qui ne se prouve pas, est écartée ;
- s'il est administrateur du serveur, enregistre le jeton qu'un membre a demandé pour son propre appareil (voir plus bas) ;
- tient **les rôles du serveur au pas du registre** : un administrateur nommé après son inscription le devient aussi sur le serveur, un administrateur redevenu membre aussi ; le dernier administrateur du serveur reste ;
- donne **l'ancre de la clé de secours** (et celle du séquestre) aux appareils administrateurs qui ne l'ont pas encore, et écrit les enveloppes de secours et de séquestre de l'époque quand le commit d'un membre n'en a pas fait (voir « Le client de l'équipe ») ;
- lit les seconds usages de ses invitations, et les invitations que d'autres appareils de l'équipe ont annulées.

## Ajouter un de ses appareils

- « Ajouter un appareil… », onglet **QR code** : le lien d'un appareil du même membre (valable une heure), à scanner ou à coller dans « Rejoindre par un lien » sur l'autre appareil. L'appareil du membre le fait entrer lui-même, une fois les six mots des deux écrans comparés (« Demandes d'entrée » sous le QR code).
- Onglet **Code court**, sans caméra : 10 caractères en deux moitiés de cinq (`K7Q4M-2XZ8P`), à taper sur l'autre appareil dans « Ajouter cet appareil par un code », avec l'adresse du serveur. Un code a **un seul essai** et 10 minutes : un mauvais code le rend inutilisable (« Un mauvais code a été tapé : ce code ne sert plus »), il en faut un nouveau. Le code passe par un échange de clés authentifié par mot de passe (SPAKE2) : le serveur ne peut pas l'essayer hors ligne. Le bon code tapé, l'autre appareil montre ses six mots, et l'appareil du membre la demande avec les siens : « Compare les six mots qu'il affiche avec ceux de sa demande avant de le faire entrer. »
- **Les cinq premiers caractères** désignent le créneau du relais du serveur, un parmi 33 millions : un inconnu qui joint le serveur ne trouve pas un code ouvert pour en brûler l'essai (voir « Le serveur d'équipe »). Les cinq autres sont le mot de passe du SPAKE2, que le serveur ne voit jamais.
- Un membre qui n'est pas administrateur du serveur : le jeton de son nouvel appareil est demandé dans l'équipe, signé par son appareil, et un administrateur en ligne l'enregistre. En attendant, la fenêtre le dit (« En attente d'un administrateur »).
- Les O se lisent 0, les I et L se lisent 1, majuscules et tirets indifférents.

## Récupérer une équipe

- Quand plus aucun administrateur n'a d'appareil : « Récupérer une équipe », avec l'adresse du serveur, un jeton que l'hébergeur donne (`escouade-team-server recovery-token --team <identifiant>`), les 24 mots et **les six mots de l'équipe**, tous lus sur la feuille de secours (jamais ceux qu'un autre dicte, jamais ceux du serveur). Le bouton attend les 24 et les 6.
- **La récupération ne rejoint que l'équipe elle-même.** Elle ne suit qu'une enveloppe signée par un administrateur de l'équipe (par sa feuille, et par l'ancre que redonnent les 24 mots), pour l'équipe de ces six mots ; avant d'entrer, elle vérifie encore sur l'état du groupe de l'enveloppe qu'il est celui de ces six mots et de cette clé de secours, et que la feuille qui a signé y est administrateur. Une enveloppe écrite par un membre, par le serveur ou pour une autre équipe est écartée ; la récupération essaie les autres, de la plus récente à la plus ancienne, avant d'envoyer quoi que ce soit. Si aucune ne passe : « Le serveur ne garde aucune enveloppe de secours signée par un administrateur de l'équipe pour ces mots… ». Des six mots faux : « Ces six mots ne sont pas ceux de l'équipe : recopie-les depuis la feuille de secours, sous les 24 mots. »
- **Une feuille sans les six mots** : « Ma feuille n'a pas ces six mots » dit qu'elle date d'avant eux, que sa clé ne ramène plus l'équipe, et qu'il faut en faire une nouvelle (« Nouvelle clé de secours ») sur un appareil administrateur s'il en reste un ; rien ne part. Sur un appareil administrateur, la carte d'une équipe dont la clé de secours date d'avant le dit aussi.
- **Après le départ ou la rétrogradation d'un administrateur**, il gardait l'ancre de la clé en vigueur : la carte de l'équipe le dit à ses administrateurs (« Un administrateur a quitté son rôle depuis la clé de secours en vigueur : fais une nouvelle clé de secours et imprime la nouvelle feuille. »).
- Cet appareil revient dans l'équipe, administrateur (sur le serveur aussi), et l'équipe reçoit **une nouvelle clé de secours**, montrée une fois avec les six mots et confirmée comme à la création. L'ancienne n'ouvre plus rien.
- Des mots faux sont refusés avant d'utiliser le jeton.

## L'alerte de fourche

- Si le serveur montre deux versions de l'équipe (une fourche), revient en arrière sur ce que l'appareil avait lu, ou change une entrée lue, la carte de l'équipe dit : « Le serveur montre deux versions de l'équipe : contacte un administrateur. », et pourquoi.
- **Ce que l'app bloque** : elle ne lit plus le flux de l'équipe et n'y envoie plus rien depuis cet appareil : ni invitation, ni entrée d'un invité, ni nouvel appareil, ni nouvelle clé de secours.
- **Ce que l'utilisateur peut faire** : comparer avec un administrateur les six mots de la tête de l'équipe (s'ils diffèrent, le serveur ment à l'un des deux) ; « Réessayer » (une lecture qui passe lève l'alerte) ; « Retirer l'équipe de cet appareil » (ses clés sont effacées ; un administrateur retire l'appareil du groupe).

## Retirer une équipe de cet appareil

Sur la carte de l'équipe, après confirmation : son état et ses clés sont effacés de cet appareil. Le groupe garde sa feuille jusqu'à ce qu'un administrateur la retire (TS-4).

## Inscrire une machine de l'équipe (OE-1)

Une machine en mode serveur qui tourne jour et nuit (l'Overseer d'équipe, l'audit) entre **sous le rôle de service** : rattachée à l'équipe, pas à une personne, ni membre ni administrateur (voir « L'Overseer d'équipe », « Le rôle de service »).

- **Un administrateur seul** l'inscrit : carte de l'équipe › « Machines de l'équipe » › « Inscrire une machine de l'équipe ». L'écran montre le code court (une heure, un essai) et la commande à taper sur la machine : `escouade-server team pair <serveur> <CODE>` (`--name <nom>` : le nom de l'équipe sur la machine). Ou, dans une fenêtre connectée à la machine, « Ajouter cet appareil par un code ».
- **Sur la machine**, la commande laisse le code au serveur qui tourne sur son dossier (un fichier lisible du seul compte du service, effacé avant d'être joué), qui joue l'unique essai du SPAKE2, reçoit l'invitation sous le rôle de service et demande à entrer ; la commande affiche les six mots de la demande. Sans serveur lancé sur le dossier, elle le dit et ne fait rien.
- **L'administrateur compare les six mots** avec ceux de sa demande, puis « Les mots sont les mêmes », comme pour toute entrée. La machine a son propre id au registre.
- **Le code va de l'administrateur à la machine**, comme pour « Ajouter un appareil » : le relais du serveur ne laisse ouvrir un créneau qu'à un appareil de l'équipe (SR2-05), et la machine n'en est pas encore un.
- **Un membre** n'inscrit pas de machine, et une machine n'en inscrit pas d'autre (« Seul un administrateur de l'équipe peut le faire. »). Une machine ajoute ses propres appareils, comme un membre.
- **Retirer une machine** : « Retirer » sur sa ligne, après confirmation ; ses appareils sont coupés au serveur, elle sort du groupe et les clés de l'équipe tournent.
