# Utiliser

## Lancer un agent

Choisis « Antigravity CLI » dans le menu « Harness » de « Nouvel agent », ou fais-en le harness par défaut d’un projet (Réglages › Projets, « Harness par défaut »).

Le menu « Modèle » liste les modèles de ton compte (`agy models`) ; pour Gemini, l’effort (Bas, Moyen, Élevé) est son menu « Effort ». Un changement de modèle, d’effort ou de mode prend effet au message suivant : Escouade relance `agy` sur la même conversation.

Un message envoyé pendant un tour attend sa fin. « Stop » arrête le tour ; le message suivant reprend la conversation. Pas d’images ni de PDF joints, pas de « Dupliquer la conversation », pas de commandes `/` dans Escouade. Antigravity donne les tokens de chaque tour, pas leur coût ni son quota.

## Les demandes en fin de tour

Quand Escouade le pilote, `agy` ne demande rien pendant un tour : ce que ses règles ne permettent pas est refusé sur le moment. Une fois le tour fini, le plugin juge chaque refus : ce que les règles d’Escouade acceptent dans le mode est accepté sans carte, le reste devient une carte (« Autoriser », « Pour cet agent », « Refuser »), et ses questions aussi. Puis il fait écrire les règles permises, relance `agy`, et l’agent reprend dans le même tour avec un message qui lui dit ce qui est permis, à refaire, et ce que tu as répondu.

« Autoriser » vaut pour cette reprise, et `agy` lit sa règle comme un début de texte : ce qui commence pareil passe aussi pendant la reprise, puis la règle est retirée. La carte le dit. « Pour cet agent » est gardé par le plugin tant qu’il tourne : seul le même appel, au caractère près, est ensuite accepté sans carte. Il n’est pas offert à l’agent d’un projet privé, ni pour une écriture dans `.git`, `.husky`, `.githooks`, `.agents` ou `.gemini`.

Un appel dont le texte a un `*`, un `?`, une parenthèse ou un retour à la ligne ne peut pas devenir une règle d’`agy` : son refus tient, et la conversation le dit.

## Les modes

Antigravity n’a pas de bac à sable. Il lit sans demander. Les règles qu’Escouade écrit pour l’agent décident du reste :

| Mode | Chez Antigravity CLI |
|---|---|
| Demander | Son mode par défaut. Ses écritures dans le dossier de l’agent sont acceptées par le plugin à la fin du tour, sauf dans `.git`, `.husky`, `.githooks`, `.agents` et `.gemini` ; ses commandes, ses écritures ailleurs et les outils MCP sont demandés. |
| Édits auto | Comme Demander, et `git add` et `git commit` du worktree sont acceptés aussi, quand la commande n’est que cela (rien après un `;`, un `|`, une redirection, un `$(…)`, un retour à la ligne). |
| Plan | Son mode `plan` : il étudie et écrit un plan sans rien changer, qui arrive sur une carte ; approuvé, Antigravity le met en œuvre. |
| Bypass | Les règles d’Escouade lui permettent tout : commandes, fichiers, outils MCP, web. Pour un projet privé, il n’a aucune règle large : chaque refus est accepté par le plugin, sauf ce qui nomme un chemin clôturé. |

Dans tous les modes, une lecture dans les dossiers des fichiers joints à l’agent est acceptée par le plugin.

Comme une écriture ou un `git add` refusés par `agy` reviennent au plugin, ils prennent une reprise du tour : `agy` est relancé, puis les refait.

## La vue Antigravity CLI

La « Vue Antigravity CLI » fait tourner l’agent dans l’interface d’Antigravity, dans un terminal de la fenêtre, sur la même conversation. Ses demandes s’y répondent dans l’interface.

## Une discussion de l’onglet Docs

Une discussion refuse toute écriture et toute commande, par ses règles, quel que soit le mode ; ces refus ne deviennent pas des cartes.

## Les projets privés

Pour un projet privé, les règles de l’agent lui refusent la lecture des chemins des autres projets, ce qui refuse aussi leur écriture, même en Bypass. Ce qu’il a voulu y faire n’arrive jamais en carte : le plugin le refuse d’office, qu’il nomme le chemin ou y arrive par `..`. En Bypass, l’agent n’a pas de `command(*)` : chaque commande revient au plugin.

Sa limite : une commande qui lirait ces chemins sans les nommer n’est pas tenue.
