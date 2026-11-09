# Utiliser

## Lancer un agent

Choisis « OpenCode » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets › Projet, « Harness par défaut »).

Un seul `opencode serve` sert tous les agents OpenCode du compte : chaque agent y est une session, dans son dossier. Escouade lance le serveur avec le premier agent et l’arrête avec le dernier.

Le menu « Modèle » liste les modèles des fournisseurs connectés, avec une recherche, leur fournisseur et des favoris (★) en tête. L’effort est la variante du modèle quand il en a. Escouade donne toujours le modèle, pour qu’OpenCode n’en choisisse pas un de lui-même.

Un message envoyé pendant un tour attend sa fin : OpenCode perdrait la demande du premier. Les images jointes passent ; pas les PDF. Seule la commande `/compact` est proposée.

## Les modes

OpenCode n’a pas de bac à sable : ce sont les règles de permission que le plugin donne à la session qui suivent le mode, et une commande acceptée peut tout faire.

| Mode | Chez OpenCode |
|---|---|
| Demander | Les fichiers du dossier de l’agent changent sans demander ; les commandes (`git add` et `git commit` compris), le web et les chemins hors du dossier demandent. |
| Édits auto | Pareil, et `git add` et `git commit` passent sans demander, dans leurs formes simples seulement. |
| Plan | L’agent `plan` d’OpenCode étudie sans rien modifier, puis te propose son plan à approuver. Tout ce qu’il demande est une carte. |
| Bypass | OpenCode ne demande rien. |

Dans tous les modes sauf Bypass, une modification de ce que git ou OpenCode exécutent ou prennent pour règles reste une carte : `.git`, `.opencode`, `opencode.json`, `.husky`, `.githooks`, à toute profondeur.

Une carte d’OpenCode a « Autoriser », « Toujours autoriser » (la carte dit le motif qu’OpenCode gardera pour la session, comme `git add *`) et « Refuser ».

## La vue OpenCode

La « Vue OpenCode » branche l’interface d’OpenCode (`opencode attach`) sur la session de l’agent, dans un terminal de la fenêtre, même en plein tour. Elle ne remplace pas le process de l’agent : elle s’y ajoute, et Escouade garde le fil. Une permission répondue dans l’interface se ferme dans Escouade, et l’inverse.

## Une discussion de l’onglet Docs

Une discussion ne change rien : ses règles refusent toute modification et toute commande, quel que soit le mode. Les outils du serveur MCP d’Escouade restent.

## Les projets privés

Pour un projet privé, les règles de la session refusent les chemins des autres projets aux outils qui prennent un chemin (`read`, `edit`, `list`, et la sortie du dossier de l’agent) et aux commandes qui les nomment, quel que soit le mode. « Toujours autoriser » n’est plus proposé : ce qu’OpenCode garde passerait devant ces refus. Un `git commit` qui lit un fichier (`-F`…) est refusé, même en Bypass.

Ses limites :

- c’est un garde-fou, pas une barrière : une commande qui atteint un chemin sans le nommer passe ;
- une commande que tu as acceptée tourne avec tes droits, et peut lire le mot de passe du serveur (voir l’onglet Permissions).
