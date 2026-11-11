---
title: "Workflows"
subject: "Kanban"
sources: ["docs/SPEC.md","Retour d'un utilisateur sur le Discord (2026-10-30), ticket ESC-309","docs/PLUGINS-DESIGN.md §4.5, choix 158 de design/app-redesign/UX-NOTES.md","Demande de l'utilisateur du 2026-11-10 (ticket ESC-406) et son choix sur aperçu : direction A de design/workflow-path/index.html"]
tickets: ["ESC-195","ESC-309","ESC-200","ESC-245","ESC-406"]
decisions: [143]
compared: "2026-10-31T10:04:30+01:00"
gaps: [{"text":"La page dit que rien d'une machine ni aucune empreinte n'entre dans le fichier. Quand un workflow a été affiné, le code écrit dans `workflow.refined` l'empreinte de la lecture d'origine, et cette empreinte est versionnée dans le dépôt.","code":["src-tauri/src/workflowfiles.rs","src-tauri/src/workflow.rs"]}]
---

## Ce que c'est

Escouade lit les workflows d'un projet : les enchaînements que suivent ses agents (étapes, commandes, sous-agents, portes, retours, boucles de prompt), tirés de la configuration du dépôt et de celle de l'utilisateur. Ce qu'elle en comprend est gardé dans un fichier par workflow, dans le dépôt, pour que l'équipe le partage et le corrige. L'historique de leurs exécutions reste sur le poste.

## Comportement

### Le fichier

- Un fichier par workflow détecté : `.escouade/workflows/<nom>.yml`, à côté de `.escouade/tickets/`. Pour un projet de plusieurs dépôts dont le dossier n'en est pas un, c'est dans le premier dépôt, comme les tickets partagés ; le dépôt de chaque workflow est dans son fichier.
- `<nom>` : l'id du workflow réduit à `[a-z0-9._-]` (`|claude|gsd` devient `claude-gsd`) ; l'id exact est dans le fichier.
- `workflow` : la définition lue (id, nom, sorte, harness, dépôt, portes, étapes avec leurs commandes, sous-agents, lectures et écritures, transitions et retours, fichiers d'état, sortie, boucle de prompt). `fixes` : les corrections de l'utilisateur, à part.
- Le schéma, `schemas/escouade-workflow.schema.json`, est publié par le site.
- Rien d'une machine : ni date, ni empreinte, ni chemin absolu (un chemin hors du projet et de `~` n'est que son nom), ni l'état du travail, relu sur place. L'écriture est déterministe, les clés dans un ordre fixe : une relecture sans changement ne touche aucun fichier.
- Restent sur le poste, dans `~/.escouade/projects/<id>/` : ce qui dit si la lecture est périmée (les dates des sources), les workflows supprimés (`workflows-removed.json`) et l'historique des agents.

### Le dossier fait foi

- Les workflows montrés et suivis (réglages, onglet, suivi, outil `get_workflow` du serveur MCP) sont ceux des fichiers, avec leurs corrections.
- Une lecture de la machine ajoute un workflow nouveau et met à jour celui qui a changé (« mis à jour par la relecture », avec les étapes ajoutées ou disparues). Elle ne supprime jamais un fichier, et ne recrée jamais celui d'un workflow supprimé (voir plus bas).
- « Ce n'est pas un workflow » le garde, marqué comme tel, pour toute l'équipe. « Retirer du dépôt » supprime le fichier d'un workflow que la machine ne lit pas ; il compte alors comme supprimé.
- Les fichiers sont relus au démarrage et quand le dossier change, comme les tickets partagés. Un fichier illisible est dit avec son chemin et sa raison, sans toucher aux corrections gardées.
- Escouade ne commite ni ne pousse jamais ces fichiers ; l'utilisateur peut les commiter.

### Supprimer un workflow

- « Supprimer ce workflow », sous ses étapes dans Réglages › Projets › Workflow, supprime son fichier tout de suite. Un fichier effacé à la main vaut la même chose : la machine retient les workflows qui avaient un fichier à la dernière lecture, et celui dont le fichier manque depuis est supprimé.
- Un workflow supprimé sort d'Escouade (ni montré, ni suivi) et aucune lecture ne réécrit son fichier, ni la relecture, ni le démarrage, ni la première lecture d'un projet. La machine le retient pour ce projet, avec l'empreinte de la lecture qui l'avait trouvé et son nom.
- Si ce qui l'a fait trouver change (l'empreinte de la lecture n'est plus la même), il est proposé à nouveau, jamais écrit : « supprimé, mais ce qui l'a fait trouver a changé : le reprendre ? », avec « Reprendre » et « Laisser supprimé » (qui retient la nouvelle empreinte).
- La liste « Supprimés » de Réglages › Projets › Workflow les montre, chacun « supprimé, ne sera pas recréé » avec « Rétablir », qui le relit et réécrit son fichier. Un toast dit la suppression et comment la défaire.
- Un fichier qui revient (un `git pull`, une autre branche) rend le workflow au projet. Un fichier illisible, ou un dossier refusé, ne vaut pas une suppression.
- Pourquoi : chaque lecture écrivait tout workflow lu sans fichier, si bien qu'un workflow supprimé (fichier effacé, « Retirer du dépôt ») revenait à la lecture suivante (retour d'un utilisateur, ESC-309).

### Pas de workflow pour ce projet

- Un choix du projet, « Workflows du projet » : « Lire les workflows » (par défaut) ou « Pas de workflow pour ce projet ». Dans l'assistant « Nouveau projet » (étape Kanban) et en tête de Réglages › Projets › Workflow ; partagé par la clé `workflows.enabled` de `.escouade.yml` (`false` : pas de workflow).
- Tant qu'il est choisi : aucune lecture (ni relecture, ni au démarrage, ni première lecture), aucune écriture (ni fichiers, ni corrections, ni lecture gardée), aucun workflow montré ni suivi. Ce qui est déjà dans `.escouade/workflows/` reste tel quel.
- Les réglages le disent avec comment le défaire : « Pas de workflow pour ce projet : Escouade ne lit ni n'écrit ses workflows et n'y suit aucun agent ; .escouade/workflows/ reste tel quel. Pour le défaire, choisis « Lire les workflows » et enregistre. » L'onglet Workflow aussi (« Pas de workflow pour ce projet », « Ouvrir les réglages du workflow »).
- Le défaire relit les workflows du projet, ses suppressions gardées.

### Dépôt ou machine

- Ne vont dans le dépôt que les workflows tirés de sa configuration : `.claude/`, `.agents/` en entier, `.github/`, `.opencode/`, `.cursor/`, `PROMPT.md`. On regarde leur porte d'entrée, même si une étape appelle une skill de l'utilisateur.
- Ceux des dossiers de l'utilisateur (`~/.claude`, `~/.codex`, GSD ou Superpowers installés là) sont lus, montrés et suivis, mais leurs fichiers restent dans `~/.escouade/projects/<id>/workflows/`, au même format.

## Réglages

- « Workflows du projet » : « Lire les workflows » (par défaut) ou « Pas de workflow pour ce projet » (voir plus haut).
- « Où garder la configuration des workflows », réglage du projet : « Dans le dépôt » par défaut. Il ne porte que sur les workflows du dépôt ; changer de choix déplace leurs fichiers, sans perte ni doublon.

## Les plugins

### Les faits `workflow.*`

Le chemin de chaque agent dans son workflow est dit aux plugins, sur le bus de faits, avec l'agent, le ticket et le projet comme les autres faits :

- `workflow.step_entered` : l'agent entre dans une étape (`workflow`, `step`, `door` si une commande l'a ouverte, `pass`, son n-ième passage) ;
- `workflow.step_left` : il la quitte, avec `durationMs`, `cost` et `outcome` (`done`, `back`, `failed`, `stopped`, `left`) ;
- `workflow.looped_back` : il revient à une étape déjà passée (`from`, `to`, `reason`, la ligne où l'étape s'est arrêtée) ;
- `workflow.left` : il sort du workflow, il fait autre chose (`doing`) ;
- `workflow.changed` : une relecture des workflows du projet donne autre chose (`workflows` : les ids de ceux qui ont changé, apparu ou disparu).

Ils sont émis d'après les passages du suivi des agents, pas d'après la fenêtre : un fait n'est dit qu'une fois, dans l'ordre où il est arrivé. Un projet privé n'envoie ces faits qu'aux plugins activés pour lui.

### Le lecteur de workflow

Un plugin peut apprendre à Escouade un workflow qu'elle ne lit pas seule : `contributes.workflowReaders[]` (un `id`, un `name`, `files`, des globs depuis le dossier du projet, sans `..` ni chemin absolu). Escouade lui donne par `workflow/read` seulement les fichiers que ses globs désignent (texte, au plus 200 de 256 Ko ; ceux du dossier de chaque dépôt pour un projet de plusieurs) ; il rend `{ workflows: [{ id, name, steps, transitions, exit }] }`.

- Le workflow est suivi comme ceux qu'Escouade lit : onglet, carte, faits `workflow.*`. Son id est `plugin:<plugin>:<id>` : il ne prend jamais la place d'un workflow lu par l'app.
- Un graphe qui ne tient pas (étape sans nom, deux étapes du même nom, transition vers une étape absente, plus de 50 workflows) est refusé en entier, avec la raison. Un plugin qui plante ne fait pas perdre ce qui était lu. Délai : 60 s.
- Le registre qui appellera les lecteurs lit les manifestes avec INT-6 ; le lecteur et son contrat existent, sans être encore appelés par l'app.

## Le diagramme de l'onglet Workflow

La vue « Diagramme » de l'onglet Workflow (à côté de la vue « Colonne », le choix retenu pour toute l'app) dessine le workflow en couches, dans la colonne de l'onglet et agrandi sur tout le panneau de travail. Elle part toujours d'un point d'entrée, montre où le workflow peut aller depuis lui, et où l'agent lu est vraiment allé.

### Le point d'entrée

- **Avec un agent lu** (l'agent ouvert, ou celui choisi dans la liste sous le diagramme) : la porte par laquelle il est entré, d'après le premier passage de sa phase en cours. C'est la porte tapée (commande ou skill), sans ses arguments ; sinon la commande de l'étape où il a commencé ; pour une boucle de prompt, son prompt. Une porte que le workflow ne connaît pas est montrée comme elle a été tapée, vers l'étape où elle a mené.
- Un agent sans historique (« Tickets », un suivi juste lu) : la première entrée du workflow qui atteint son étape, sinon son étape.
- **Sans agent**, ou quand l'entrée ne peut pas être connue (ses étapes n'existent plus dans le workflow) : un choix des portes en tête, « Entrée : par où l'agent commence », la première par défaut. Ce sont toutes les portes d'entrée du workflow, chacune vers l'étape où elle entre ; la première étape y figure en tête si aucune porte n'y mène. Une seule entrée : pas de choix. Les flèches du clavier passent d'une porte à l'autre.
- La porte est dessinée en tête, « Entrée » puis ce qui se tape ; une porte qui entre plus loin est juste au-dessus de son étape.

### Où ça peut aller

- Seules sont dessinées les étapes atteignables depuis l'entrée, par les transitions et les retours ; la sortie quand son étape est atteinte, et le tour suivant seulement s'il revient à une étape atteinte.
- Les autres étapes se replient sous le diagramme en « n étapes hors de ce chemin ». Ouvert, le repli liste leur nom et leur commande et dit « Pas atteignable depuis <porte> ».
- Ce qui est possible est dessiné léger : trait fin et pointillé, étape au bord pointillé, conditions des choix et des retours lisibles sur leur trait.
- « Copier en Mermaid » copie toujours le workflow entier, sans les agents.

### Où ça va vraiment

- Le chemin de l'agent lu est tracé par-dessus, dans l'ordre où il l'a parcouru, d'après les passages que son suivi garde : il revient tel quel après un redémarrage. Les passages hors du workflow, ou d'une étape qu'il n'a plus, sont sautés.
- Chaque passage porte son numéro dans le chemin sur son étape (« 1, 2, 5 » sur une étape où il est revenu). Au-delà de quatre passages d'une même étape, les trois derniers et « +n » ; l'infobulle de l'étape les dit tous.
- Ce qui a été pris est en trait plein et épais : la porte, les transitions, et les retours pris, en rouge, avec la raison que l'étape a donnée (sinon la condition du workflow). Le plein contre le pointillé distingue le pris du possible sans la couleur.
- Un tour de boucle (la même étape de nouveau) se lit sur la pastille de l'étape : « 1/3 » pour un tour pris sur trois, sa raison dans l'infobulle.
- L'étape en cours est entourée de vert, son numéro en vert, et dit « en cours » ; la prochaine possible dit « ensuite ». Une étape qu'un retour a défaite garde ses numéros sans être « faite ».
- Un agent sans historique (« Tickets ») n'a pas de numéros : ses étapes faites sont cochées et les traits entre elles pleins.

### Plusieurs agents

Seul l'agent lu a son chemin. Les autres agents du workflow restent des pastilles sur leur étape (verte pour l'agent lu, ambre pour celui qui attend, « +n » au-delà de trois) ; en choisir un dans la liste lit le sien, depuis sa propre entrée.

### En mots

- Le chemin se lit aussi dans une liste ordonnée pour un lecteur d'écran, « Chemin de <agent>, dans l'ordre », dans la colonne comme en grand : « Planifier, entrée par /gsd:plan-phase », « Planifier, tour de boucle : <raison>, 2e passage », « Planifier, retour depuis Vérifier : <raison>, 3e passage », « Exécuter, 2e passage, en cours ».
- La colonne et la vue agrandie suivent la même logique ; à 1024 px de large, rien ne déborde du panneau (340 px) ni de la vue agrandie.
