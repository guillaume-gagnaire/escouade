# Les workflows d’un projet

Un workflow est la suite d’étapes que tes agents suivent dans un projet (GSD, Superpowers, une boucle `PROMPT.md`…). Escouade le lit sur ta machine, sans rien dépenser, et dit où en est chaque agent. Cette page dit ce qu’elle suit de chaque harness et ce que le serveur MCP en donne.

> [!NOTE]
> Le mot est « workflow ». Une « boucle » reste le tour d’un ticket (« boucle 2/5 »), et un retour en arrière dans un workflow s’appelle un retour.

## Ce qu’Escouade lit

Escouade ne demande rien à l’agent. Elle lit les commandes (`.claude/commands`), les skills, les sous-agents et les prompts de chaque harness (`.claude/`, `.agents/` en entier, `.github/`, `.opencode/`, `.cursor/`), sous le dossier du projet et dans les dossiers propres des harness, puis les fichiers d’état que les étapes écrivent (`.planning/STATE.md`, `fix_plan.md`…). Un workflow, ce sont au moins deux étapes reliées, ou un prompt rejoué ; une commande seule (`/commit`) n’en est pas un.

Pour chaque workflow, elle garde :

- ses **portes d’entrée** : les commandes ou skills par où l’on entre ;
- ses **étapes**, chacune avec sa commande, ses sous-agents et les fichiers qu’elle lit et écrit ;
- ses **retours** : un retour vers une étape déjà passée, avec la ligne qui dit quand et le nombre de fois au plus ;
- ses **fichiers d’état**, et ce qu’ils disent (« phase 3 sur 6 », « 12 cases cochées sur 20 ») ;
- sa **condition de sortie**.

Un projet à plusieurs dépôts a le workflow de chaque dépôt. La lecture se refait quand un fichier lu change, et à la demande.

## Où c’est gardé

Ce qu’Escouade comprend d’un workflow tiré de la config du dépôt (ses portes, ses étapes, leurs sous-agents, ses retours, ses fichiers d’état) et tes corrections sont gardés dans le dépôt, un fichier par workflow dans `.escouade/workflows/`, à côté des [tickets partagés](shared-board). Escouade écrit ces fichiers et les tient à jour, mais ne les commite jamais : commite-les avec ton travail, et toute l’équipe qui utilise Escouade voit les mêmes workflows et les mêmes corrections. Un workflow est du dépôt quand sa porte d’entrée l’est, même si une étape appelle une skill de tes propres dossiers (elle est nommée telle quelle).

- Les fichiers ne gardent rien d’une machine : ni date, ni chemin absolu, ni l’état du travail (« phase 3 sur 6 » est relu sur place). Une relecture sans changement ne les touche pas.
- Le dossier fait foi : un coéquipier dont la machine ne lit pas un workflow le voit et le suit quand même. Une relecture ajoute un workflow nouveau et met à jour celui qui a changé, sans jamais supprimer un fichier ni recréer celui que tu as supprimé (voir [supprimer un workflow](#supprimer-un-workflow-ou-n-en-avoir-aucun)).
- Les fichiers sont relus quand ils changent (un `git pull`, une autre branche) ; un fichier illisible est signalé avec son chemin et sa raison.
- Les workflows de tes propres dossiers (`~/.claude`, `~/.codex`, GSD ou Superpowers installés là) sont lus, montrés et suivis de la même façon, mais leur fichier reste sur ta machine, dans `~/.escouade/projects/<id>/workflows/`, jamais dans le dépôt. « Sur cette machine », dans Réglages › Projets › Workflow, y garde aussi ceux du dépôt.
- L’historique des agents dans les workflows reste sur ta machine, dans le dossier de données d’Escouade.
- Le format est décrit par un schéma JSON : `https://escouade.dev/schemas/escouade-workflow.schema.json`.

## Supprimer un workflow, ou n’en avoir aucun

- **Supprimer ce workflow**, sous ses étapes dans Réglages › Projets › Workflow, supprime son fichier tout de suite. Effacer son fichier à la main fait la même chose : à la relecture suivante, Escouade voit qu’il manque. Dans les deux cas, le workflow sort d’Escouade et la relecture ne le recrée plus, ni au redémarrage.
- Ta machine s’en souvient pour ce projet, dans `~/.escouade/projects/<id>/`. La liste « Supprimés » les montre, chacun avec « Rétablir », qui le relit et réécrit son fichier.
- Si ce qui l’a fait trouver change (une commande ou une skill modifiée), il est proposé à nouveau, sans être écrit : « Reprendre » le rétablit, « Laisser supprimé » le garde écarté tel qu’il est maintenant.
- Un fichier qui revient (un `git pull`, une autre branche) rend le workflow au projet.
- « Ce n’est pas un workflow » reste l’autre façon de l’écarter : son fichier est gardé, marqué, pour que toute l’équipe l’écarte aussi.

« Pas de workflow pour ce projet », dans l’assistant « Nouveau projet » (étape Kanban) ou en haut de Réglages › Projets › Workflow, coupe tout. Escouade ne lit plus les workflows du projet, n’écrit aucun fichier et n’y suit aucun agent. Ce qui est déjà dans `.escouade/workflows/` reste tel quel. L’onglet Workflow le dit, avec où le défaire : « Lire les workflows », puis « Enregistrer ». Dans un projet partagé, c’est la clé `workflows.enabled: false` de son [`.escouade.yml`](escouade-yml).

## Où en est chaque agent

Chaque agent est placé d’après ce qu’il fait : la porte qu’il a tapée, les sous-agents qu’il lance, les skills qu’il appelle, les fichiers d’état qu’il écrit, la fin de ses tours. Une étape commence au premier fait qui la signale et finit au premier fait d’une autre ; sa durée et son coût sont ceux des tours entre les deux.

Chaque passage dans une étape reste dans l’historique de l’agent : l’heure, la durée, le coût, ce qui s’y est fait, l’issue, et pour un retour, la raison que l’étape donne. L’historique est gardé après un redémarrage, avec l’agent. Un agent qui fait autre chose (son message ne vient d’aucune porte et ses outils ne signalent aucune étape) est « hors du workflow » jusqu’à la porte suivante.

## Ce que chaque harness permet de suivre

| Harness | L’étape | Le sous-agent en cours | Le coût par étape |
|---|---|---|---|
| Claude Code (conversation) | oui | oui | oui |
| Claude Code (vue Claude Code) | oui, par ses hooks | oui, par ses hooks | à la fin du tour |
| OpenCode | oui | oui | oui |
| Copilot CLI | oui | oui | le coût du tour, selon l’offre du compte |
| Codex | par ses prompts et les fichiers d’état | non | par tour |
| Cursor CLI | par ses commandes et les fichiers d’état | oui, selon ACP | non |
| Cursor CLI (vue Cursor) | par son transcript, à quelques secondes près | non | non |

Un suivi partiel est dit comme tel dans la fenêtre (« Suivi partiel »), avec ce qui manque. Rien n’est inventé : ce qu’un harness ne montre pas n’est pas deviné.

Trois précisions vues en vrai : une porte que l’agent lance seul (Claude Code appelle `/ecrire` par son outil `Skill` dans le tour de `/etude`) compte comme une porte, l’étape suit donc sans que tu aies tapé la seconde ; le coût d’un modèle local (OpenCode sur Ollama) est de 0 $, ce qui n’est pas un trou du suivi ; et dans la vue Cursor, une commande tapée est reconnue même si l’interface la remplace par son fichier.


## Par le serveur MCP

Trois outils, tous en lecture seule (voir [le serveur MCP](mcp) pour les autres) :

- **`get_workflow`** (`project`) : les workflows du projet, comme l’onglet Workflow les montre. Pour chacun, ses portes, ses étapes (commande, sous-agents, fichiers lus et écrits), ses retours (`back`, avec leur `condition` et leur `max`) et, sur chaque étape, les agents qui y sont maintenant (`agentsHere` : id, nom, depuis quand, Nᵉ passage). Les agents qui suivent un workflow sans être sur une étape sont à part (`agentsElsewhere`).
- **`list_agents`** : chaque agent gagne `workflow`, son nom, son étape, son tour (celui d’un prompt rejoué, ou la boucle de son ticket) et depuis quand ; `null` quand il n’en suit aucun.
- **`get_agent_summary`** : le même `workflow`, et l’historique des étapes (`history`, les 100 derniers passages, du plus ancien au plus récent).

Un agent ne se met pas dans une étape par un outil : il y entre en faisant le travail, et c’est ce que ces outils lisent. Rien d’autre n’est exposé que ce que montre l’onglet : jamais un fichier du projet, ni une conversation.

### Projets privés

Un [projet privé](settings-projects) ne dit rien de ses workflows aux agents des autres projets : `get_workflow` ne le trouve pas (dans les mêmes mots qu’un projet qui n’existe pas), et ses agents, leur étape et leur historique n’apparaissent dans aucune liste des autres. Ses propres agents, eux, ne voient que lui.

## À lire aussi

- [Le serveur MCP](mcp) : tous les outils, et ce qu’un agent d’Escouade voit.
- [Le bandeau Plan](plan) : la liste de tâches et les sous-agents d’un agent, qui complètent son étape.
- [Les harness](harness) : ce que chacun sait faire.
