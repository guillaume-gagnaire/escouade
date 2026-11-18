# Utiliser

## Lancer un agent

Choisis « Prime Agent » dans le menu « Harness » de « Nouvel agent », en mode **Bypass**. Chaque agent a son `prime-agent --mode acp --no-session`, lancé dans le dossier de l’agent. Son modèle est celui que Prime Agent prend par défaut ; sa session propose les autres par son option de configuration `model`.

Un message envoyé pendant un tour attend sa fin. « Stop » arrête le tour (`session/cancel`) ; le message suivant part normalement.

## Son outil : une cellule Python

Prime Agent n’a qu’un outil, un noyau IPython qui reste en vie le temps de la session. Ses fichiers, ses commandes (`await bash(...)`), ses serveurs MCP (`await mcp.call_tool(...)`) passent tous par du code. Chaque cellule apparaît dans la conversation comme un outil **Python** : son code sur sa ligne, sa sortie une fois ouverte. Une écriture de fichier n’a donc pas de diff à elle dans la conversation : regarde le diff de l’agent.

Prime Agent n’a pas d’outil pour poser une question en ACP : son premier message lui dit de finir son tour sur sa question, qui arrive en carte, et ta réponse lui est envoyée.

## Les modes

Un seul : **Bypass**. Prime Agent ne demande jamais rien et n’a ni Plan, ni Demander, ni Édits auto, ni lecture seule. Voir l’onglet Permissions.

## Le serveur MCP d’Escouade

Quand « Les agents peuvent piloter Escouade » est allumé, Escouade donne son serveur MCP à la session de chaque agent (`session/new`, en HTTP, le jeton dans un en-tête). Prime Agent l’accepte et son modèle l’appelle depuis Python ; l’essai l’a montré. Escouade n’écrit rien dans `~/.prime/agent/settings.json` : tes serveurs MCP à toi y restent tels quels.

## Ce qu’il ne fait pas encore dans Escouade

- **Reprendre une session.** Prime Agent 0.10.3 ne charge pas de session par ACP (`loadSession: false`). Après un redémarrage d’Escouade ou de l’agent, la conversation reste dans Escouade, mais Prime Agent repart d’une session neuve, sans le contexte d’avant. Escouade 2.2.3 le dit dans la conversation et redonne à cette session les consignes de l’agent (le protocole de son ticket, celui des questions) avec un résumé de ce qu’il a fait ; la 2.2.2 ne le fait pas.
- **Ses sous-agents.** `rlm.spawn` demande une session sauvegardée ; celles de ses agents dans Escouade ne le sont pas (`--no-session`, pour ne rien laisser dans son démon). Son modèle le voit refusé et fait autrement.
- **Le niveau de réflexion**, avant Escouade 2.2.3. Prime Agent le règle par son option `thought_level`, pour les modèles qui raisonnent ; depuis la 2.2.3, l’effort de l’agent (Faible, Moyen, Élevé) la pose, quand son modèle l’offre (`acp.efforts`). Un modèle qui ne raisonne pas n’a pas l’option : rien n’est posé.
- **Les jetons et le coût.** Prime Agent n’en dit rien par ACP.
- **Une discussion de l’onglet Docs**, et un agent à côté d’un projet privé : il ne démarre pas.
