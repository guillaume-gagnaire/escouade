# Rejoindre une équipe

Pour un membre : entrer dans une équipe par le lien qu’on t’a envoyé, y ajouter tes autres appareils, garder la clé de secours, cloner les projets de l’équipe et répondre aux réglages qu’elle te demande d’accepter.

> [!NOTE]
> Le serveur d’équipe est un aperçu de la 2.3. Aujourd’hui, une équipe partage ses membres, sa liste de projets et ses réglages. Le Kanban d’équipe en direct, puis la spec, les décisions, les documents et la mémoire sur le serveur, sont à venir.

## Une équipe, en deux mots

Une équipe partage, sur un serveur qu’elle héberge, ce qu’Escouade garde aujourd’hui sur ton poste. Tout y est chiffré de bout en bout sur ton poste : le serveur ne lit rien. Tes comptes de harness, tes secrets, les conversations de tes agents et ton code n’y vont jamais (voir [Où vont mes données ?](team/data)).

Chaque membre a un rôle : **Administrateur** (il invite, tient la liste des projets et signe les réglages de l’équipe) ou **Membre**.

## Ouvrir les équipes

La fenêtre des équipes s’ouvre :

- par Réglages › Avancé › « Serveur d’équipe (aperçu) » › « Ouvrir les équipes… » ;
- par « Équipes… » du bouton « + » de la barre des projets, quand il montre un menu (c’est le cas quand tu as des projets fermés) ;
- par un lien d’invitation ouvert dans ton navigateur.

Elle liste les équipes de cet appareil, et propose « Créer une équipe », « Rejoindre par un lien », « Ajouter cet appareil par un code » et « Récupérer une équipe ».

## Rejoindre par un lien

Un administrateur t’envoie un lien de la forme `https://team.exemple.fr/join/…#s=…&t=…`. Il sert une seule fois, et expire (72 h par défaut). Ce qui suit le `#` est un secret qui ne passe jamais par le serveur.

1. Ouvre le lien dans ton navigateur : la page du serveur le passe à Escouade. Ou colle-le dans « Rejoindre par un lien ».
2. Escouade montre le serveur et les six mots de l’empreinte de l’équipe. **Vérifie-les avec la personne qui t’invite** : ce sont ceux de son équipe. S’ils diffèrent, n’entre pas.
3. Donne à l’équipe un nom sur cet appareil, puis « Rejoindre ».
4. La demande part. Un administrateur en ligne te fait entrer : rien à faire, la fenêtre le dit (« Tu es dans l’équipe … »).

Si l’app dit « Cette invitation a déjà servi : demande un nouveau lien à son administrateur, qui est prévenu. », quelqu’un s’en est servi avant toi : le lien a peut-être fuité. Si le serveur ne connaît pas l’invitation, elle a expiré ou un administrateur ne l’a pas encore validée : réessaie dans un moment, ou demande un nouveau lien.

## Ajouter un de tes appareils

Sur un appareil déjà dans l’équipe, ouvre sa carte et « Ajouter un appareil… ».

- **QR code** : fais-le scanner par l’autre appareil, ou colle son lien dans « Rejoindre par un lien » sur l’autre appareil. Il est valable une heure.
- **Code court**, sans caméra : l’appareil dans l’équipe affiche un code de 8 caractères (`K7Q4-M2XZ`). Sur le nouvel appareil, « Ajouter cet appareil par un code », avec l’adresse du serveur et ce code. Un code n’a qu’un essai et 10 minutes : un mauvais code le rend inutilisable, affiches-en un nouveau.

Si tu n’es pas administrateur, ton nouvel appareil attend qu’un administrateur du serveur le valide : dès qu’il est en ligne.

Un appareil perdu ne te fait rien perdre : le contenu est à l’équipe, et un administrateur t’invite de nouveau.

## La carte de l’équipe

Chaque équipe a sa carte dans la fenêtre des équipes :

- **Empreinte de l’équipe** : six mots, les mêmes chez chaque membre. Compare-les de vive voix si tu veux en être sûr.
- **Tête de l’équipe** : les six mots de son état actuel, les mêmes chez un membre à jour.
- **La clé de séquestre**, si l’équipe en a une : « Cette équipe a une clé de séquestre : l’organisation peut lire son contenu. », avec son détenteur.
- **Les clés de cet appareil dans un fichier**, quand le trousseau du système les a refusées (toujours sous Linux).
- Pour un administrateur : « Inviter… », les invitations ouvertes, utilisées ou expirées, et le rappel de nommer un second administrateur.
- « Projets de l’équipe », « Réglages de l’équipe », « Relire » et « Retirer de cet appareil ».

## La clé de secours

Celui qui crée l’équipe en devient le propriétaire et reçoit sa **clé de secours** : 24 mots, montrés une seule fois, dont il retape trois. Si tous les administrateurs perdent leurs appareils, ces mots et un jeton que donne l’hébergeur du serveur ramènent l’équipe (« Récupérer une équipe »).

- Note-les à la création : imprime-les, ou garde-les dans le gestionnaire de mots de passe de l’organisation. Jamais dans un message.
- Tant qu’ils ne sont pas confirmés, la carte le dit. Si l’app a été fermée avant, les mots sont perdus : « Nouvelle clé de secours » en fait une autre.
- Une récupération donne une nouvelle clé de secours ; l’ancienne n’ouvre plus rien.

Un simple membre n’a pas de clé de secours à garder.

## Projets de l’équipe

« Projets de l’équipe » montre la liste des projets de l’équipe, chiffrée : le serveur n’en lit ni les noms ni les adresses.

- **Sur ce poste** : les projets de la liste dont un dépôt est déjà cloné ici, reconnus à l’adresse de leur dépôt. « Rattacher à l’équipe » en fait un projet de l’équipe, sans nouveau clone : ce que l’équipe impose et recommande vaut alors pour lui. Ton nom et ton logo restent.
- **À cloner** : les autres, chacun avec ses adresses entières, sa branche et qui y a accès. « Cloner » ouvre l’assistant « Nouveau projet » sur le clone. **Vérifie les adresses avant de cloner.** Le clone passe par tes comptes sur l’hébergeur de code (tes identifiants git, tes clés ssh) ; sans accès, l’assistant le dit (« Demande l’accès à acme/api sur GitHub »).
- Un projet réservé à d’autres membres est marqué « Réservé à d’autres membres de l’équipe ».
- Un administrateur ajoute, modifie et retire les projets de la liste. Seules les adresses `https://` et ssh y entrent.

## Les réglages de l’équipe

L’équipe recommande et impose des réglages à ses projets par son `.escouade.yml` d’équipe, signé par ses administrateurs (voir [le guide du RSSI](team/security#ce-qu-une-equipe-peut-imposer)). « Réglages de l’équipe » le montre :

- la version en vigueur, qui l’a signée et quand, et la règle du quorum ;
- **ce que l’équipe impose**, avec la raison écrite par l’administrateur, et **ce qu’elle recommande** ;
- **l’historique** de chaque version et son changement, clé par clé.

Dans les réglages d’un projet de l’équipe, ce qu’elle impose est grisé, avec un cadenas et « Imposé par l’équipe … » ; un choix interdit dit « non permis par l’équipe … ».

### Les accords qu’on te demande

Ce qui touche le réseau ou lance une commande ne s’applique jamais sans ton accord, sur ce poste :

- **Une clé sensible** (l’adresse d’un proxy, ses exclusions, un fichier d’autorités) arrive avec ce qu’elle permet, dit en clair : « Ce certificat permet de lire le trafic TLS d’Escouade. » « Accepter sur ce poste » l’applique.
- **Une commande proposée** (la préparation des worktrees, `launch`, la commande de test) arrive avec son texte et son empreinte. Relis-la : elle s’exécutera sur ce poste, avec tes droits, dans les projets de l’équipe. « J’ai relu, accepter » l’accepte.
- Un changement de la valeur ou de la commande redemande ton accord. « Retirer mon accord » le retire.

Une bande, en haut de la fenêtre, dit chaque nouvelle version (« L’équipe Acme a changé 2 réglages · Claire Martin, il y a 5 min · Voir ») et chaque accord en attente (« Acme demande ton accord pour 1 réglage »).

## Si l’app dit que le serveur ment

Si le serveur montre deux versions de l’équipe, revient en arrière ou change ce que ton appareil avait déjà lu, la carte de l’équipe dit « Le serveur montre deux versions de l’équipe : contacte un administrateur. », et pourquoi. Ton appareil cesse alors de lire l’équipe et de lui envoyer quoi que ce soit.

- Compare avec un administrateur les six mots de la tête : s’ils diffèrent, le serveur ment à l’un de vous.
- « Réessayer » relit : une lecture qui passe lève l’alerte.
- « Retirer l’équipe de cet appareil » efface son état et ses clés de ce poste.

## Ce qui n’est pas encore là

- Le Kanban d’équipe en direct, et la prise d’un ticket par un seul agent (à venir).
- La spec, les décisions, les documents et la mémoire sur le serveur (à venir).
- L’équipe comme espace à part dans la fenêtre, avec ses membres, ses appareils et leurs rôles, et l’éditeur des réglages clé par clé (à venir).
- Rattacher un dépôt à l’équipe par son `.escouade.yml`, avec l’invitation proposée à qui l’ouvre (à venir).
