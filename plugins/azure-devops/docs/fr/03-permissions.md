# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- `dev.azure.com` et ses sous-domaines (`*.dev.azure.com` : ceux du profil et des identités d'Azure), `*.visualstudio.com`, en https.
- L'instance de chacun de tes comptes, celle que tu as tapée (un Azure DevOps Server) : cet hôte seul, sur son port, aucun de ses sous-domaines. Elle peut être sur ton réseau local : c'est toi qui l'as choisie.
- Aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local (hors l'instance que tu as tapée) est refusé avant de partir.
- Une redirection vers un autre hôte ne reçoit ni ton jeton ni le corps de l'appel.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit chez Azure DevOps

Il lit, avec ton compte :

- ton identité dans l'organisation (pour la ligne du compte, pour voter et pour activer l'auto-complete) ;
- les PR des dépôts de tes projets : leur état, leurs relecteurs et leurs votes, leurs conflits, leurs fils de commentaires, les évaluations des stratégies de leur branche, leurs statuts, leurs builds et les journaux de leurs tâches ; les stratégies de merge de la branche cible ;
- pour Azure Boards : les projets de l'organisation, leurs types d'éléments de travail et leurs états, et les éléments ouverts du projet lié (titre, description, critères d'acceptation, étapes de reproduction d'un bug, type, état, personne assignée, auteur, tags).

Il écrit, quand Escouade le lui demande :

- une PR (son titre, sa description, sa branche, brouillon ou non), ses relecteurs, ses étiquettes et l'élément de travail qu'elle ferme ;
- son merge, ou son auto-complete ;
- la relance d'un build ;
- une relecture que tu as validée et ton vote, une réponse dans un fil, sa résolution, un commentaire ;
- pour Azure Boards : l'état d'un élément de travail (`System.State`), un commentaire.

Il ne pousse aucun code : c'est `git`, lancé par Escouade, qui pousse les branches. Il ne supprime ni branche, ni dépôt, ni élément de travail.

## Les secrets qu'il reçoit

- Le jeton de chacun de tes comptes. Escouade le garde dans le trousseau du système (ou le demande à `az` pour un compte qui passe par la CLI) et le prête au plugin quand il le demande, pour un appel ; le plugin ne l'écrit nulle part.
- Il part dans l'en-tête `Authorization` des appels vers l'organisation du compte, et seulement là : en Basic pour un jeton personnel, en Bearer pour un jeton Entra ID de `az`.
- Il peut lire les jetons de ses propres comptes, jamais celui d'un autre plugin.

## Ce qui part chez Azure DevOps

- Les lectures ci-dessus.
- Ce qu'Escouade publie en ton nom : le titre et la description d'une PR, une relecture et ses commentaires (ce que tu as gardé, rien avant ton clic), les commentaires de synchronisation d'un élément de travail (la clé du ticket Escouade, sa branche, ses critères, les notes de l'agent, ce qui a été fait, le lien de la PR).

## Ce qui n'y part jamais

- Les conversations de tes agents et tes fichiers. Ton code ne part que par la branche que `git` pousse, pas par le plugin.
- Ton jeton, vers un autre hôte que celui de son organisation.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande (ni `git`, ni `az`).
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service qu'Azure DevOps.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : les PR ne sont plus suivies (la carte de l'agent dit « Hébergeur arrêté : PR plus suivies » et garde la dernière lecture), les éléments de travail ne sont plus synchronisés, et rien n'est perdu. Son onglet « Journal » montre ce qu'il a fait.
