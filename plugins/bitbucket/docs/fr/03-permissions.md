# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- `api.bitbucket.org`, en https, et aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local est refusé avant de partir.
- Une redirection vers un autre hôte ne reçoit ni ton jeton ni le corps de l'appel. La page suivante d'une liste n'est suivie que sur l'API.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit chez Bitbucket

Il lit, avec ton compte :

- ton nom et ton identifiant (pour la ligne du compte) ;
- les PR des dépôts de tes projets : leur état, leurs participants et leurs approbations, leurs commentaires et leurs fils, les statuts de leur dernier commit, les étapes et les journaux de leurs pipelines, leurs déploiements et ceux des environnements du dépôt.

Il écrit, quand Escouade le lui demande :

- une PR (son titre, son texte, sa branche, brouillon ou non) et ses relecteurs ;
- son merge ;
- une relecture que tu as validée (ses commentaires, puis l'approbation ou la demande de changements), une réponse sous un commentaire, la résolution d'un fil, un commentaire.

Il ne pousse aucun code : c'est `git`, lancé par Escouade, qui pousse les branches. Il ne supprime ni branche, ni dépôt.

## Les secrets qu'il reçoit

- Le jeton d'API de ton compte Atlassian, que tu as donné avec son e-mail. Escouade le garde dans le trousseau du système et le prête au plugin quand il le demande, pour un appel ; le plugin ne l'écrit nulle part.
- Avec ton e-mail, il forme l'en-tête `Authorization` des appels vers `api.bitbucket.org`, et seulement là.
- Il peut lire le jeton de ses propres comptes, jamais celui d'un autre plugin.

## Ce qui part chez Bitbucket

- Les lectures ci-dessus.
- Ce qu'Escouade publie en ton nom : le titre et le texte d'une PR, une relecture et ses commentaires (ce que tu as gardé, rien avant ton clic).

## Ce qui n'y part jamais

- Les conversations de tes agents et tes fichiers. Ton code ne part que par la branche que `git` pousse, pas par le plugin.
- Ton jeton, vers un autre hôte que `api.bitbucket.org`.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande.
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service que Bitbucket Cloud.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : les PR ne sont plus suivies (la carte de l'agent dit « Hébergeur arrêté : PR plus suivies » et garde la dernière lecture), et rien n'est perdu. Son onglet « Journal » montre ce qu'il a fait.
