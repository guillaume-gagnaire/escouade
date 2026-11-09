# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- `codeberg.org` et `gitea.com` (et leurs sous-domaines), en https.
- L'instance de chacun de tes comptes, celle que tu as tapée : cet hôte seul, sur son port, aucun de ses sous-domaines. Elle peut être sur ton réseau local : c'est toi qui l'as choisie.
- Aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local (hors l'instance que tu as tapée) est refusé avant de partir.
- Une redirection vers un autre hôte que l'instance ne reçoit ni ton jeton ni le corps de l'appel.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit sur l'instance

Il lit, avec ton compte :

- ton login (pour la ligne du compte), et la version de l'instance ;
- les PR des dépôts de tes projets : leur état, leurs relectures et leurs lignes, leurs commentaires, les statuts combinés de leur dernier commit, les runs et jobs d'Actions et leurs journaux ; les réglages de merge du dépôt, ses étiquettes, et si une branche est protégée.

Il écrit, quand Escouade le lui demande :

- une PR (son titre, son texte, sa branche), ses relecteurs et ses étiquettes ;
- son merge, ou son auto-merge sur une branche protégée ;
- la relance des jobs en échec d'un run d'Actions (Gitea 1.26 et après) ;
- une relecture que tu as validée, un commentaire, une réponse dans un fil et sa résolution (Gitea 1.26 et après).

Il ne pousse aucun code : c'est `git`, lancé par Escouade, qui pousse les branches. Il ne supprime ni branche, ni dépôt.

## Les secrets qu'il reçoit

- Le jeton de chacun de tes comptes. Escouade le garde dans le trousseau du système et le prête au plugin quand il le demande, pour un appel ; le plugin ne l'écrit nulle part.
- Il part avec les appels à l'API de l'instance du compte, et seulement là.
- Il peut lire les jetons de ses propres comptes, jamais celui d'un autre plugin.

## Ce qui part sur l'instance

- Les lectures ci-dessus.
- Ce qu'Escouade publie en ton nom : le titre et le texte d'une PR, une relecture et ses commentaires (ce que tu as gardé, rien avant ton clic).

## Ce qui n'y part jamais

- Les conversations de tes agents et tes fichiers. Ton code ne part que par la branche que `git` pousse, pas par le plugin.
- Ton jeton, vers un autre hôte que l'API de son instance.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande.
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service que tes instances Gitea ou Forgejo.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : les PR ne sont plus suivies (la carte de l'agent dit « Hébergeur arrêté : PR plus suivies » et garde la dernière lecture), et rien n'est perdu. Son onglet « Journal » montre ce qu'il a fait.
