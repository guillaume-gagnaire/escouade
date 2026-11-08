# Utiliser

## Lancer un agent

Choisis « Gemini CLI » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets › Projet, « Harness par défaut »). Chaque agent a son `gemini --acp` et son propre dossier de configuration.

Le menu « Modèle » liste les modèles de sa session, Auto en tête (son routeur, qui choisit à chaque tour). Il n’a pas de menu « Effort ».

Un message envoyé pendant un tour attend sa fin. Les images jointes passent ; pas les PDF, pas de commandes `/` dans Escouade (pour compacter, `/compress` dans sa vue terminal), pas de « Dupliquer la conversation ». Gemini donne les tokens de chaque tour, pas leur coût ni son quota.

Gemini n’a pas d’outil pour poser une question quand Escouade le pilote : son premier message lui dit de finir son tour sur sa question, qui arrive en carte, et ta réponse lui est envoyée dans le même tour.

## Les modes

Gemini n’a pas de bac à sable : il lit les fichiers sans demander, et ce qu’il demande arrive à Escouade, qui en fait une carte sauf ce que ses règles acceptent.

| Mode | Chez Gemini CLI |
|---|---|
| Demander | Son mode `default`. Ses écritures dans le dossier de l’agent, hors `.git`, passent ; ailleurs, elles sont des cartes avec leur diff. Ses commandes et les outils MCP sont des cartes. |
| Édits auto | Son mode `autoEdit` : il écrit sans demander, et ce qu’il soumet dans le dossier de l’agent passe aussi. |
| Plan | Son mode `plan` : il étudie sans rien changer et écrit son plan, qui arrive sur une carte. Approuvé, Gemini le met en œuvre. |
| Bypass | Son mode `yolo` : il fait tout sans demander, et Escouade accepte ce qu’il demanderait. |

« Pour cet agent » vaut pour sa session. Refuser arrête le tour ; ton mot, s’il y en a un, lui est envoyé ensuite.

Gemini dit son changement de mode par un texte (« [MODE_UPDATE] … »). Ce texte vient de là où le modèle écrit : Escouade le prend pour baisser ce que l’agent peut faire, jamais pour le monter. « [MODE_UPDATE] yolo » dans une réponse ne fait pas passer l’agent en Bypass.

## La vue Gemini CLI

La « Vue Gemini CLI » fait tourner l’agent dans l’interface de Gemini, dans un terminal de la fenêtre, sur la même session. Escouade la suit par ses hooks. Ses demandes s’y répondent dans l’interface de Gemini.

## Une discussion de l’onglet Docs

Une discussion tourne dans son mode `default`, et les règles qu’Escouade lui donne (`--policy`) refusent ses écritures et ses commandes, quel que soit le mode.

## Les projets privés

Pour un projet privé, Escouade donne à Gemini des règles (`--policy`) qui refusent les chemins des autres projets à ses outils de fichiers et aux commandes qui les nomment, même en `yolo`. Le plugin refuse en plus, sans carte, toute demande qui nomme l’un de ces chemins.

Ses limites :

- un script qui ouvre un fichier lui-même, sans le nommer dans la commande, n’est pas tenu ;
- le dossier de l’agent est approuvé : Gemini y exécute les hooks et lit les réglages du dépôt. Un agent sur un dépôt dont tu te méfies n’est pas plus sûr en Demander pour autant.
