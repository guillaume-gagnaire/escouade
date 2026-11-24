---
title: "Les parties d'un projet"
subject: "projets"
sources: ["Ticket ESC-658 (demande de l'utilisateur du 2026-11-23 : « pouvoir activer ou désactiver, par projet, la partie docs, mémoire partagée, etc. »)","crates/escouade-model/src/parts.rs (le registre et l'ordre)","src-tauri/src/parts.rs (ce que chacune emporte dans le cœur)","src/lib/parts.ts, src/components/settings/PartsPage.svelte (dans la fenêtre)"]
tickets: ["ESC-658","ESC-657"]
decisions: [311]
compared: null
---

# Les parties d'un projet

## Ce que c'est

Un projet prend d'Escouade quatre **parties**, que chacun allume ou coupe **pour ce projet seul** : les **Docs**, la **mémoire partagée**, le **Kanban**, le **lancement et les navigateurs**. Là où une fonctionnalité (*Fonctionnalités*) se coupe pour toute l'app, une partie se coupe dans un projet, les autres projets gardent tout.

Une partie coupée **disparaît de ce projet** : son interface, ses outils MCP, ce que ses agents en entendent et son travail de fond. **Rien n'est effacé** : ses fichiers, ses tickets, ses souvenirs restent, et la rallumer les retrouve tels quels.

## Le registre

Un seul registre dans le cœur (`escouade_model::parts`), dans cet ordre : `docs`, `memory`, `kanban`, `launch`. La fenêtre en a le miroir (`src/lib/parts.ts`). Les Docs sont une part de la fonctionnalité « Espace Docs », le Kanban de la fonctionnalité « Kanban » : la fonctionnalité coupée dans Réglages › Fonctionnalités, la partie l'est dans tous les projets, quel que soit son interrupteur (sa rangée le dit, grisée). La mémoire partagée et le lancement ne sont d'aucune fonctionnalité.

## Qui décide

Le premier qui le dit l'emporte :

1. **L'équipe qui l'impose** (`enforced` du `.escouade.yml` d'équipe, rang 5) : l'interrupteur est grisé, « Imposé par l'équipe Acme : il ne se change pas sur ce poste. » ; le cœur refuse de le changer.
2. **Ce poste** : le choix fait dans Réglages › Projets › Parties du projet, gardé avec le projet dans l'état de la machine (`Project.parts`), **jamais dans un fichier**. « Choisi sur ce poste. » ; quand le dépôt propose aussi quelque chose : « Choisi sur ce poste : il passe devant le défaut que propose le dépôt (.escouade.yml). » ; « Reprendre le défaut » l'oublie.
3. **Le dépôt** : `parts.docs`, `parts.memory`, `parts.kanban`, `parts.launch` du `.escouade.yml` (voir *.escouade.yml*). Un défaut seulement, tant que le poste n'a rien choisi : « Proposé par le dépôt (.escouade.yml) : ton choix ici passe devant. » Un choix fait avant que le fichier n'arrive, ou avant qu'il ne change, tient.
4. **L'équipe qui le recommande** (`defaults`, rang 2) : « Recommandé par l'équipe Acme : ton choix ici passe devant. »
5. **Personne** : allumée. Un projet où rien n'est dit a toutes ses parties, comme avant : aucun projet existant ne change.

Les clés `parts.*` sont des clés du fichier seul : jamais appliquées aux réglages du projet, jamais écrites par l'export, jamais marquées « Réglage local ». Une valeur qui n'est pas un booléen ne propose rien. Le fichier relu (un `git pull`, une modification) suit aussitôt ce qu'il propose là où le poste n'a rien choisi.

## Ce que chacune emporte, dans ce projet

| Partie | Interface | Outils MCP (absents de la liste de ses agents, refusés pour lui) | Ce que ses agents n'entendent plus | Travail de fond arrêté | Ce qui reste |
| --- | --- | --- | --- | --- | --- |
| **Docs** | L'onglet Docs (spec, décisions, documents, discussions), « Branche des docs » de Git et worktrees ; un renvoi vers une page, une décision ou un document n'ouvre rien. | Ceux de l'Espace Docs (`get_spec_tree`… `share_docs`). | La phrase de la spec dans Escouade (« La spec et les décisions de ce projet sont dans Escouade… »), la règle de la spec du protocole d'un ticket, la phrase des documents. | La comparaison au code (à la main et après un ticket validé), « Planifier », les discussions et brainstorms (refusés à leur départ). | Les pages, les décisions, les documents, les discussions, la branche des docs. |
| **Mémoire partagée** | La section « Mémoire partagée » de Réglages › Projets › Équipe ; l'offre de reprendre la mémoire de Claude Code. | `search_memory`, `read_memory`, `write_memory`, `forget_memory` pour ce projet (la mémoire propre de l'Overseer, sans projet, reste). | La phrase de la mémoire partagée. | Aucun travail à elle (la copie d'équipe suit le journal des docs, rien n'est perdu). | `.escouade/memory/` tel quel, la mémoire d'équipe. |
| **Kanban** | L'onglet Kanban, les pages « Kanban » et « Tickets importés » (et leur recherche), les tâches programmées de la barre latérale ; un renvoi vers le Kanban n'ouvre rien. Le projet n'a plus que des agents. | Ceux du Kanban, des workflows (`get_workflow`) et des tâches programmées. | Un brainstorm des Docs ne propose plus de tickets ; aucun protocole de ticket n'est donné (aucun ne démarre). | Aucun de ses tickets ne démarre, ni par le pilote auto ni forcé ; ses workflows ne sont ni lus ni suivis ; « Planifier » (qui fait des tickets) est refusé. Un agent de ticket déjà au travail finit, comme un agent. | Les tickets, leurs colonnes, leurs fichiers, les réglages du Kanban, les fichiers des workflows. Rallumé, le pilote auto repart. |
| **Lancement et navigateurs** | La section « Lancement » de la barre latérale, la page « Lancement » des réglages, « Lancer » du menu d'un agent, « Tester » et « Préparer le lancement », la section « Navigateurs » (ESC-657) et ses navigateurs : la page du lancement d'un agent, celle d'une commande lancée dans le projet, un navigateur ouvert seul ; ouverts, ils se ferment (dans la vue d'un agent comme dans un volet de l'écran partagé). | Aucun aujourd'hui. | La recette `lancement` du protocole d'un ticket (ses ports restent dits). | Les lancements de ses commandes et de test refusés, ceux de test de ses agents arrêtés, leurs sondes (`http_ready`) muettes, ses navigateurs refusés à l'ouverture (`devpage_open`, quelle que soit la page). | Les commandes de lancement, leurs adresses, les recettes des agents, « Ouvrir les pages des lancements », la largeur gardée de chaque page. |

Un appel d'un outil d'une partie coupée est refusé avant de tourner, pour le projet qu'il nomme (`project`), celui de son ticket (`ticket`, `key`), ou sinon celui de l'agent qui appelle ; un autre projet le garde. La raison est dite : « La partie « Docs » est coupée pour le projet « demo » (Réglages › Projets › Parties du projet) : ce qu'elle fait n'est pas disponible dans ce projet. L'utilisateur peut la rallumer, ses données sont gardées. » ; « … comme le propose son .escouade.yml … » quand c'est le dépôt ; « … par l'équipe Acme … » quand elle l'impose. Le journal de l'activité MCP le compte comme refusé ; ce refus ne nourrit pas les propositions des fonctionnalités coupées (la fonctionnalité est allumée).

## Réglages › Projets › Parties du projet

Une page de chaque projet, juste après « Projet », dans la ligne de Réglages › Fonctionnalités : « Ce que ce projet prend d'Escouade : ses Docs, sa mémoire partagée, son Kanban, son lancement et ses navigateurs ». Une ligne par partie : son icône, son nom, à quoi elle sert, ce que la couper change (la phrase décrit l'interrupteur pour un lecteur d'écran), qui en décide (ci-dessus), et l'interrupteur à droite, enregistré au geste par sa propre commande (`project_set_part`, de la liste unique des commandes). En pied : « Une partie coupée l'est pour ce projet seul, sur ce poste : rien n'est effacé, et la rallumer retrouve tout. Allumée, elle ne lance rien d'elle-même : son travail de fond suit ses propres réglages. »

Les pages d'une partie coupée quittent la barre et la recherche du projet (Kanban et Tickets importés, Lancement), leurs rangées aussi (« Branche des docs », « Mémoire partagée ») ; une page ouverte d'une partie qu'on coupe laisse la place à « Parties du projet ».

## Sécurité : un dépôt ne fait rien tourner de plus, n'éteint aucune protection

- Une partie **ne fait que retirer**. Allumée, elle laisse chaque travail de fond à ses propres interrupteurs : le pilote auto, « En parallèle », la confiance accordée aux commandes du `.escouade.yml`, la comparaison au code et « Planifier » qui partent d'un geste. Un dépôt qui dit `true` ne lance donc rien que l'utilisateur n'ait déjà allumé ailleurs, et ne rallume rien que ce poste a coupé (le poste passe devant).
- Un dépôt qui dit `false` coupe une partie, rien d'autre : la confiance des commandes, le projet privé, les modes permis, les stratégies du poste et de l'équipe restent ce qu'ils sont. Les commandes de lancement proposées attendent l'accord comme avant, partie allumée ou non.
- La lecture du fichier reste celle de `projectfile` (sans lien, plafonnée).

## Un projet d'équipe

Le `.escouade.yml` d'équipe peut porter les clés `parts.*` comme toute clé du `.escouade.yml` d'un projet (sorte **valeur**, d'une signature) : `enforced` passe devant le poste, `defaults` vient après le dépôt. Elles ne valent que pour les projets de l'équipe ; une valeur qui n'est pas un booléen est écartée.

## À distance

L'état vient du cœur : un client de l'API distante ou une fenêtre sur un serveur montre les parties du projet telles que ce cœur les a, et le cœur refuse ce qu'une partie coupée ferait, quel que soit le client.

## Limites

- Les navigateurs n'ont pas d'outil MCP : s'ils en reçoivent un, il s'ajoute au registre (`parts::of_tool`) pour partir avec la partie.
- Un client MCP extérieur (Claude hors Escouade) voit toujours la liste entière : ses appels pour un projet dont la partie est coupée sont refusés avec la raison.
- Les commandes de la fenêtre qui lisent la mémoire ou les Docs d'un projet (hors MCP) ne sont plus montrées, mais le cœur ne les refuse pas toutes : ce qu'elles font n'est jamais du travail de fond.
