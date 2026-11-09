# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- `api.linear.app`, en https, et aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local est refusé avant de partir.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse de Linear est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit chez Linear

Il lit, avec ton compte, par l'API GraphQL de Linear :

- ton identifiant, ton nom et ton e-mail (pour la ligne « Connecté · … ») ;
- tes équipes (nom et clé) et leurs états ;
- les issues de l'équipe liée qui ne sont ni terminées ni annulées : identifiant, titre, description, lien, priorité, personne assignée, état et étiquettes.

Il écrit seulement :

- l'état d'une issue, quand son ticket change de colonne et que la correspondance des statuts en donne un ;
- un commentaire sur une issue, quand Escouade le lui demande.

Il ne crée, ne renomme, n'archive et ne supprime aucune issue ni équipe.

## Les secrets qu'il reçoit

- La clé d'API personnelle de ton compte Linear, que tu as donnée dans Réglages › Services connectés. Escouade la garde dans le trousseau du système et la prête au plugin quand il la demande, pour un appel ; le plugin ne l'écrit nulle part.
- Elle part dans l'en-tête `Authorization` de chaque appel vers `api.linear.app`, et seulement là.
- Il ne peut lire aucun autre secret : ni celui d'un autre plugin, ni un autre compte d'Escouade.

## Ce qui part chez Linear

- Les lectures ci-dessus.
- Le texte des commentaires qu'Escouade fait poster : la clé du ticket Escouade (`DEM-5`), sa branche, ses critères et leur état, les notes de l'agent, ce qui a été fait, l'issue de la validation et le lien de la pull request, selon la colonne. Ils sont écrits dans la « Langue des textes rédigés par les agents » (Réglages › Général).

## Ce qui n'y part jamais

- Ton code, tes fichiers, tes diffs, les conversations de tes agents.
- Les tickets qui ne viennent pas de Linear.
- Ta clé, vers un autre hôte que `api.linear.app`.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande.
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service que Linear.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : la synchronisation de ses tickets s'arrête et le dit, les tickets importés restent. Son onglet « Journal » montre ce qu'il a fait.
