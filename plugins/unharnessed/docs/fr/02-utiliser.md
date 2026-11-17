# Utiliser

## Lancer un agent

Choisis « Unharnessed » dans le menu « Harness » de « Nouvel agent ». Chaque agent a son `unharnessed --mode acp` et sa session, dans le dossier de l’agent.

## Son seul outil : une cellule Python

Le modèle n’a qu’un outil, `ipython` : un REPL Python qui reste vivant toute la session, où il lit, cherche, modifie les fichiers et lance ses commandes (`bash(...)`). Chaque cellule apparaît dans la conversation comme un outil **Python** : sa ligne montre le début de son code, et la sortie de la cellule s’affiche quand tu l’ouvres. Le code entier d’une longue cellule n’est pas montré aujourd’hui, ni ses écritures en diff : Unharnessed ne dit pas à Escouade quels fichiers une cellule touche.

## Le modèle et l’effort

Un agent tourne sur le modèle par défaut d’Unharnessed (son réglage `defaultModel`, ou le dernier choisi). Le menu « Modèle » d’Escouade n’a rien à choisir : les modèles d’Unharnessed sont ceux des fournisseurs de chacun. Pour en changer, change le défaut d’Unharnessed (`unharnessed model list` les liste). L’effort n’est pas réglé par Escouade.

## Pendant un tour

- Un message envoyé pendant un tour attend sa fin.
- **Stop** arrête le tour : la cellule en cours est interrompue.
- Unharnessed n’a pas d’outil pour poser une question en ACP : son premier message lui dit de finir son tour sur sa question, qui arrive en carte, et ta réponse lui est envoyée.
- Unharnessed lance parfois **un tour à lui**, sans message : quand une commande lancée par `bash()` finit après sa cellule. Avec Escouade 2.2.3, c’est un tour de la conversation : l’agent passe au travail, puis revient au repos à la fin que son flux marque, et ce qu’il a dit reste dans ce tour. Avant, ses mots apparaissaient sous le dernier tour et comptaient dans la réponse du suivant.
- Ce que tient son contexte et la fenêtre de son modèle sont lus dans ses messages (Escouade 2.2.3). Ni le coût ni les jetons d’entrée et de sortie d’un tour : il ne les dit pas (son coût vaut 0, `billed: false`, avec un modèle local), Escouade n’en estime rien.

## Après un redémarrage

Un processus d’Unharnessed ne tient qu’une session, et n’en reprend une autre qu’au lancement (`--resume`), sous l’id qu’il lui a donné en l’enregistrant. Avec Escouade 2.2.3, son manifeste le dit (`acp.resume`) : quand Escouade relance un agent (après un redémarrage d’Escouade, ou de l’agent), il lance Unharnessed avec `--resume <id>`, puis charge la session, son historique et les variables de son noyau. Si la session est perdue (son fichier effacé), l’agent continue sur une session neuve, la conversation le dit, et ses consignes lui sont redonnées avec son prochain message. Avec Escouade 2.2.2, la session n’est pas reprise : une session neuve, dite.

## Piloter Escouade

Quand « Les agents peuvent piloter Escouade » est allumé (Réglages › Agents), chaque session reçoit le serveur MCP d’Escouade par ACP, en HTTP, avec le jeton de l’agent dans un en-tête. Chez Unharnessed, les serveurs MCP s’appellent depuis le noyau : le modèle écrit `await mcp.call_tool("escouade", "get_board", {})` (son skill `mcp`). Il n’y a rien à déclarer par `unharnessed mcp add` : n’y ajoute pas le serveur d’Escouade, son jeton change d’un agent à l’autre.

## Ce qu’il ne fait pas dans Escouade

- **Une discussion de l’onglet Docs** : Unharnessed n’a pas de mode qui ne change rien, une discussion ne démarre pas.
- **Les projets privés** : Unharnessed lit et écrit les fichiers lui-même, la clôture ne tiendrait pas. Dès qu’un de tes projets est privé, aucun agent Unharnessed ne démarre, dans aucun projet.
- **Plan, Demander, Édits auto, Auto** : il n’a que Bypass.
