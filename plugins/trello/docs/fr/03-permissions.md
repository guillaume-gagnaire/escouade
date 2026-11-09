# Permissions

Ce plugin est un module WASM : il tourne dans Escouade, isolé, sans programme à lui sur ta machine. Ce qu'il peut faire, c'est ce que son manifeste demande et ce que l'hôte d'Escouade lui permet, rien de plus.

## Ce qu'il joint

- `api.trello.com`, en https, et aucun autre hôte. C'est l'hôte d'Escouade qui l'applique : un appel vers un autre hôte, vers une adresse IP écrite telle quelle, ou vers une adresse de ta machine ou de ton réseau local est refusé avant de partir.
- Il passe par le proxy de Réglages › Avancé › Réseau quand tu en as un.
- Chaque réponse de Trello est limitée à 8 Mio et à 15 secondes.

## Ce qu'il lit et ce qu'il écrit chez Trello

Il lit, avec ton compte :

- ton identifiant et ton nom (pour la ligne « Connecté · @… » et la puce « Mes cartes ») ;
- tes tableaux ouverts, leurs listes ouvertes, leurs cartes ouvertes (nom, description, liste, numéro, lien, étiquettes, membres) et leurs checklists.

Il écrit seulement :

- la liste d'une carte, quand son ticket change de colonne et que la correspondance des statuts en donne une ;
- un commentaire sur une carte, quand Escouade le lui demande.

Il ne crée, ne renomme, n'archive et ne supprime aucune carte, liste ou tableau.

## Les secrets qu'il reçoit

- La clé d'API et le jeton de ton compte Trello, que tu as donnés dans Réglages › Services connectés. Escouade les garde dans le trousseau du système et les prête au plugin quand il les demande, pour un appel ; le plugin ne les écrit nulle part.
- Trello les veut dans l'adresse de chaque appel : ils partent donc vers `api.trello.com`, et seulement là. Le journal du plugin note la méthode et l'hôte d'un appel, jamais son adresse complète.
- Il ne peut lire aucun autre secret : ni celui d'un autre plugin, ni un autre compte d'Escouade.

## Ce qui part chez Trello

- Les appels de lecture ci-dessus.
- Le texte des commentaires qu'Escouade fait poster : la clé du ticket Escouade (`DEM-5`), sa branche, ses critères et leur état, les notes de l'agent, ce qui a été fait, l'issue de la validation et le lien de la pull request, selon la colonne. Ils sont écrits dans la « Langue des textes rédigés par les agents » (Réglages › Général).

## Ce qui n'y part jamais

- Ton code, tes fichiers, tes diffs, les conversations de tes agents.
- Les tickets qui ne viennent pas de Trello.
- Ton jeton, vers un autre hôte que `api.trello.com`.

## Ce que le plugin ne peut pas faire

- Lancer un programme ou une commande.
- Lire ou écrire un fichier de ta machine.
- Recevoir les événements d'Escouade (agents, tickets, projets) : il ne connaît que les appels qu'Escouade lui fait.
- Joindre un autre service que Trello.

## Le couper

Sa page, dans la vue Plugins, a un interrupteur pour toute l'app et un onglet « Projets ». Coupé, il ne reçoit plus rien : la synchronisation de ses tickets s'arrête et le dit, les tickets importés restent. Son onglet « Journal » montre ce qu'il a fait.
