# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- `api.github.com` et les API de GHE.com (`*.ghe.com`), en https.
- L'instance de chacun de tes comptes, celle que tu as tapée (un GitHub Enterprise Server) : cet hôte seul, sur son port, aucun de ses sous-domaines.
- Les hôtes où GitHub range le journal d'un job d'Actions : `*.actions.githubusercontent.com` et `*.blob.core.windows.net`. GitHub y renvoie par une redirection ; le plugin la suit **sans ton jeton**, qui ne quitte jamais l'API de GitHub.
- Aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local est refusé avant de partir. Seule l'instance que tu as tapée échappe à ce dernier contrôle, puisque c'est toi qui l'as choisie.
- Une redirection vers un autre hôte que l'API ne reçoit ni ton jeton ni le corps de l'appel. La page suivante d'une liste n'est suivie que sur l'API elle-même.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit chez GitHub

Il lit, avec ton compte :

- ton login, et les droits d'un jeton classic (pour la ligne du compte) ;
- les PR des dépôts de tes projets : leur état, leurs relecteurs, leurs relectures, leurs commentaires et leurs fils, leurs checks et statuts, le journal d'un job d'Actions, leurs déploiements et ceux des environnements du dépôt ; les réglages de merge du dépôt ;
- pour GitHub Issues : tes dépôts, leurs étiquettes et leurs issues ouvertes.

Il écrit, quand Escouade le lui demande :

- une PR (son titre, son texte, sa branche, brouillon ou non), ses relecteurs et ses étiquettes ;
- son merge, ou la demande d'auto-merge ;
- la relance des jobs en échec d'un run d'Actions ;
- une relecture que tu as validée, une réponse sous un commentaire, un commentaire ;
- pour une issue : un commentaire, sa fermeture ou sa réouverture, ses étiquettes de la correspondance des statuts.

Il ne pousse aucun code : c'est `git`, lancé par Escouade, qui pousse les branches. Il ne supprime ni branche, ni dépôt, ni issue.

## Les secrets qu'il reçoit

- Le jeton de chacun de tes comptes GitHub. Escouade le garde dans le trousseau du système (ou le demande à `gh` pour un compte qui passe par la CLI) et le prête au plugin quand il le demande, pour un appel ; le plugin ne l'écrit nulle part.
- Il part dans l'en-tête `Authorization` des appels à l'API de l'instance du compte, et seulement là.
- Il peut lire les jetons de ses propres comptes GitHub, jamais celui d'un autre plugin.

## Ce qui part chez GitHub

- Les lectures ci-dessus.
- Ce qu'Escouade publie en ton nom : le titre et le texte d'une PR, une relecture et ses commentaires (ce que tu as gardé, rien avant ton clic), les commentaires de synchronisation d'une issue (la clé du ticket Escouade, sa branche, ses critères, les notes de l'agent, ce qui a été fait, le lien de la PR).

## Ce qui n'y part jamais

- Les conversations de tes agents et tes fichiers. Ton code ne part que par la branche que `git` pousse, pas par le plugin.
- Ton jeton, vers un autre hôte que l'API de son instance.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande (ni `git`, ni `gh`).
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service que GitHub et le stockage de ses journaux.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : les PR ne sont plus suivies (la carte de l'agent dit « Hébergeur arrêté : PR plus suivies » et garde la dernière lecture), les issues ne sont plus synchronisées, et rien n'est perdu. Son onglet « Journal » montre ce qu'il a fait.
