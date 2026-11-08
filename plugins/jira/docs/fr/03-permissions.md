# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- Les sites `*.atlassian.net`, en https.
- Le site de ton compte Jira, celui que tu as tapé, s'il n'est pas sous `atlassian.net` : cet hôte seul, sur son port, aucun de ses sous-domaines.
- Aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local est refusé avant de partir. Seul le site que tu as tapé échappe à ce dernier contrôle, puisque c'est toi qui l'as choisi.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse de Jira est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit chez Jira

Il lit, avec ton compte, par l'API REST v3 de Jira Cloud :

- ton e-mail, ton nom et ton identifiant (pour la ligne « Connecté · … » et la puce « Assignés à moi ») ;
- les projets de ton site (clé et nom), et les statuts des types de tickets du projet lié ;
- les tickets du projet lié que tu cherches : clé, titre, description, statut, type, priorité, personne assignée et labels ; le nombre approché de tickets d'une recherche ;
- le statut d'un ticket et les transitions de son workflow, avant de le faire avancer.

Il écrit seulement :

- une transition de workflow, quand son ticket change de colonne et que la correspondance des statuts donne un statut ;
- un commentaire sur un ticket, quand Escouade le lui demande.

Il ne crée, ne modifie, n'assigne et ne supprime aucun ticket ni projet.

## Les secrets qu'il reçoit

- Le jeton d'API de ton compte Atlassian, que tu as donné dans Réglages › Services connectés. Escouade le garde dans le trousseau du système et le prête au plugin quand il le demande, pour un appel ; le plugin ne l'écrit nulle part.
- Avec ton e-mail, il forme l'en-tête `Authorization` (Basic) de chaque appel vers ton site, et seulement là.
- Le site et l'e-mail ne sont pas des secrets : Escouade les donne au plugin avec le compte.
- Il ne peut lire aucun autre secret : ni celui d'un autre plugin, ni un autre compte d'Escouade.

## Ce qui part chez Jira

- Les lectures ci-dessus, et les recherches que tu tapes dans la fenêtre d'import (en JQL).
- Le texte des commentaires qu'Escouade fait poster : la clé du ticket Escouade (`DEM-5`), sa branche, ses critères et leur état, les notes de l'agent, ce qui a été fait, l'issue de la validation et le lien de la pull request, selon la colonne. Ils sont écrits dans la « Langue des textes rédigés par les agents » (Réglages › Général).

## Ce qui n'y part jamais

- Ton code, tes fichiers, tes diffs, les conversations de tes agents.
- Les tickets qui ne viennent pas de Jira.
- Ton jeton, vers un autre hôte que ton site.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande.
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service que Jira.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : la synchronisation de ses tickets s'arrête et le dit, les tickets importés restent. Son onglet « Journal » montre ce qu'il a fait.
