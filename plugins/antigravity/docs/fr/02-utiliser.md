# Utiliser

## Lancer un agent

Choisis « Antigravity CLI » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets › Projet, « Harness par défaut »).

Le menu « Modèle » liste les modèles de ton compte (`agy models`) ; pour Gemini, l’effort (Bas, Moyen, Élevé) est son menu « Effort ». Un changement de modèle, d’effort ou de mode prend effet au message suivant : Escouade relance `agy` sur la même conversation.

Un message envoyé pendant un tour attend sa fin. « Stop » arrête le tour ; le message suivant reprend la conversation. Pas d’images ni de PDF joints, pas de « Dupliquer la conversation », pas de commandes `/` dans Escouade. Antigravity donne les tokens de chaque tour, pas leur coût ni son quota.

## Les demandes en fin de tour

Quand Escouade le pilote, `agy` ne demande rien pendant un tour : ce que ses règles ne permettent pas est refusé sur le moment. Une fois le tour fini, le plugin juge chaque refus : ce que les règles d’Escouade acceptent dans le mode est accepté sans carte, le reste devient une carte (« Autoriser », « Pour cet agent », « Refuser »), et ses questions aussi. Puis il fait écrire les règles permises, relance `agy`, et l’agent reprend dans le même tour avec un message qui lui dit ce qui est permis, à refaire, et ce que tu as répondu.

« Autoriser » vaut pour cette reprise, et `agy` lit peut-être sa règle comme un début de texte : ce qui commence pareil passerait aussi pendant la reprise, puis la règle est retirée. La carte le dit. « Pour cet agent » est gardé par le plugin tant qu’il tourne : seul le même appel, au caractère près, est ensuite accepté sans carte, mais chaque fois qu’il revient, sa règle « début de texte » vaut de nouveau pour la reprise. Il n’est pas offert à l’agent d’un projet privé, ni pour une écriture dans `.git`, `.husky`, `.githooks`, `.agents` ou `.gemini` ou du dossier de l’agent lui-même, ni pour un chemin relatif ou qui n’est pas déjà absolu et normal.

Ce que le plugin accepte d’office vaut comme « Autoriser » : une règle « début de texte » pour la reprise. La conversation le dit à chaque fois. C’est pourquoi aucune commande n’est acceptée d’office, une écriture ne l’est que pour un fichier strictement dans le dossier de l’agent nommé par son chemin absolu et normal, et la reprise refuse en plus les écritures dans `.git`, `.husky`, `.githooks`, `.agents` et `.gemini` du dossier de l’agent. L’essai réel sur `agy` 1.5.3 a montré qu’une règle tient la commande entière, pas son début ; le plugin garde pourtant cette prudence.

Un appel dont le texte a un `*`, un `?`, une parenthèse, un crochet, une accolade ou un retour à la ligne ne peut pas devenir une règle d’`agy` : son refus tient, et la conversation le dit.

## Les modes

Antigravity n’a pas de bac à sable. Il lit sans demander. Les règles qu’Escouade écrit pour l’agent décident du reste :

| Mode | Chez Antigravity CLI |
|---|---|
| Demander | Son mode par défaut. Ses écritures d’un fichier dans le dossier de l’agent, nommé par son chemin absolu, sont acceptées par le plugin à la fin du tour, sauf dans `.git`, `.husky`, `.githooks`, `.agents` et `.gemini` et sauf le dossier lui-même ; ses commandes, ses écritures ailleurs et les outils MCP sont demandés. |
| Édits auto | Comme Demander : `git add` et `git commit` sont demandés aussi, tant que l’essai réel d’`agy` manque (une règle `command(git add .)` laisserait peut-être passer `git add . ; node x.js`). |
| Plan | Son mode `plan` : il étudie et écrit un plan sans rien changer, qui arrive sur une carte ; approuvé, Antigravity le met en œuvre. |
| Bypass | Les règles d’Escouade lui permettent tout : commandes, fichiers, outils MCP, web. Pour un projet privé, il n’a aucune règle large : chaque refus revient au plugin, qui refuse ce qui nomme un chemin clôturé ou y mène une fois résolu, demande chaque commande et accepte le reste. |

Dans tous les modes, une lecture dans les dossiers des fichiers joints à l’agent, par son chemin absolu, est acceptée par le plugin.

Comme une écriture refusée par `agy` revient au plugin, elle prend une reprise du tour : `agy` est relancé, puis la refait.

## La vue Antigravity CLI

La « Vue Antigravity CLI » fait tourner l’agent dans l’interface d’Antigravity, dans un terminal de la fenêtre, sur la même conversation. Elle a les mêmes règles : aucune règle `allow` sauf en Bypass sans projet privé ni discussion, les refus de la clôture ; Édits auto y est le mode par défaut d’`agy`. Ce qu’aucune règle ne permet, l’interface le demande, et ses demandes s’y répondent.

## Une discussion de l’onglet Docs

Une discussion refuse toute écriture et toute commande, par ses règles, quel que soit le mode ; ces refus ne deviennent pas des cartes.

## Les projets privés

Pour un projet privé, les règles de l’agent lui refusent la lecture des chemins des autres projets, ce qui refuse aussi leur écriture, même en Bypass. Ce qu’il a voulu y faire n’arrive jamais en carte : le plugin le refuse d’office, qu’il nomme le chemin ou y arrive par `..`, un mot de commande compris (`type ..\prive\secret.txt`). En Bypass, l’agent n’a pas de `command(*)` : chaque commande revient au plugin, et va à une carte.

Sa limite : une commande qui lirait ces chemins sans les nommer, ou par un chemin que fait le shell (`$HOME`, `%CD%`), n’est pas refusée d’office ; elle va à une carte.
