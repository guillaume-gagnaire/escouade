# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- `gitlab.com` (et ses sous-domaines), en https.
- L'instance de chacun de tes comptes, celle que tu as tapée : cet hôte seul, sur son port, aucun de ses sous-domaines. Elle peut être sur ton réseau local : c'est toi qui l'as choisie.
- Aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local (hors l'instance que tu as tapée) est refusé avant de partir.
- Une redirection vers un autre hôte que l'instance ne reçoit ni ton jeton ni le corps de l'appel. La page suivante d'une liste n'est suivie que sur l'API de l'instance.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit chez GitLab

Il lit, avec ton compte :

- ton login (pour la ligne du compte et la puce « Assignées à moi ») ;
- les MR des projets de tes dépôts : leur état, leurs approbations, leurs relecteurs, leurs discussions, leur pipeline et ses jobs, le journal d'un job, leurs déploiements ; les réglages de merge du projet ; le chemin du fork d'où vient une MR ;
- pour GitLab Issues : les projets dont tu es membre, leurs étiquettes, les étiquettes des listes de leurs tableaux, et leurs issues ouvertes.

Il écrit, quand Escouade le lui demande :

- une MR (son titre, sa description, sa branche, brouillon ou non), ses relecteurs et ses étiquettes ;
- son merge, ou la demande d'auto-merge ;
- la relance des jobs en échec d'un pipeline ;
- une relecture que tu as validée, son approbation, une réponse dans une discussion, sa résolution, un commentaire ;
- pour une issue : une note, sa fermeture ou sa réouverture, ses étiquettes de la correspondance des statuts.

Il ne pousse aucun code : c'est `git`, lancé par Escouade, qui pousse les branches. Il ne supprime ni branche, ni projet, ni issue.

## Les secrets qu'il reçoit

- Le jeton de chacun de tes comptes GitLab. Escouade le garde dans le trousseau du système (ou le demande à `glab` pour un compte qui passe par la CLI) et le prête au plugin quand il le demande, pour un appel ; le plugin ne l'écrit nulle part.
- Il part avec les appels à l'API de l'instance du compte, et seulement là.
- Il peut lire les jetons de ses propres comptes GitLab, jamais celui d'un autre plugin.

## Ce qui part chez GitLab

- Les lectures ci-dessus.
- Ce qu'Escouade publie en ton nom : le titre et la description d'une MR, une relecture et ses commentaires (ce que tu as gardé, rien avant ton clic), les notes de synchronisation d'une issue (la clé du ticket Escouade, sa branche, ses critères, les notes de l'agent, ce qui a été fait, le lien de la MR).

## Ce qui n'y part jamais

- Les conversations de tes agents et tes fichiers. Ton code ne part que par la branche que `git` pousse, pas par le plugin.
- Ton jeton, vers un autre hôte que l'API de son instance.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande (ni `git`, ni `glab`).
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service que tes instances GitLab.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : les MR ne sont plus suivies (la carte de l'agent dit « Hébergeur arrêté : MR plus suivies » et garde la dernière lecture), les issues ne sont plus synchronisées, et rien n'est perdu. Son onglet « Journal » montre ce qu'il a fait.
