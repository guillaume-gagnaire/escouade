# Utiliser

## Lancer un agent

Choisis « Antigravity CLI » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets, « Harness par défaut »).

Le menu « Modèle » liste les modèles de ton compte (`agy models`) ; pour Gemini, l’effort (Bas, Moyen, Élevé) est son menu « Effort ». Un changement de modèle, d’effort ou de mode prend effet au message suivant : Escouade relance `agy` sur la même conversation.

Un message envoyé pendant un tour attend sa fin. « Stop » arrête le tour ; le message suivant reprend la conversation. Pas d’images ni de PDF joints, pas de « Dupliquer la conversation », pas de commandes `/` dans Escouade. Antigravity donne les tokens de chaque tour, pas leur coût ni son quota.

## Les demandes en fin de tour

Quand Escouade le pilote, `agy` ne demande rien pendant un tour : ce que ses règles ne permettent pas est refusé sur le moment. Une fois le tour fini, Escouade te montre une carte par refus (« Autoriser », « Pour cet agent », « Refuser ») et ses questions. Puis il écrit les règles que tu as permises, et l’agent reprend dans le même tour avec un message qui lui dit ce qui est permis, à refaire, et ce que tu as répondu.

« Autoriser » vaut pour cette reprise ; « Pour cet agent » garde la règle dans le dossier de l’agent, pour lui seul et ses reprises.

## Les modes

Antigravity n’a pas de bac à sable. Il lit sans demander. Les règles qu’Escouade écrit pour l’agent décident du reste :

| Mode | Chez Antigravity CLI |
|---|---|
| Demander | Son mode par défaut. Ses écritures dans le dossier de l’agent passent ; ses commandes, ses écritures ailleurs et les outils MCP sont demandés à la fin du tour. |
| Édits auto | Son mode `accept-edits` ; `git add` et `git commit` passent aussi. |
| Plan | Son mode `plan` : il étudie et écrit un plan sans rien changer, qui arrive sur une carte ; approuvé, Antigravity le met en œuvre. |
| Bypass | Les règles d’Escouade lui permettent tout : commandes, fichiers, outils MCP, web. |

Dans tous les modes, les dossiers des fichiers joints à l’agent lui sont permis en lecture.

## La vue Antigravity CLI

La « Vue Antigravity CLI » fait tourner l’agent dans l’interface d’Antigravity, dans un terminal de la fenêtre, sur la même conversation. Ses demandes s’y répondent dans l’interface.

## Une discussion de l’onglet Docs

Une discussion refuse toute écriture et toute commande, par ses règles, quel que soit le mode ; ces refus ne deviennent pas des cartes.

## Les projets privés

Pour un projet privé, les règles de l’agent lui refusent la lecture des chemins des autres projets, ce qui refuse aussi leur écriture, même en Bypass. Ce qu’il a voulu y faire n’arrive jamais en carte : le plugin le refuse d’office.

Sa limite : une commande qui lirait ces chemins n’est pas tenue.
