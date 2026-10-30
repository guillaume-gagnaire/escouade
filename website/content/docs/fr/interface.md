# L’interface

Un tour de la fenêtre d’Escouade, région par région : la barre des projets et la barre de titre, la barre latérale des agents, l’en-tête et la conversation, le panneau latéral, les deux dispositions, la barre d’état, les réglages et ce qui change sous macOS.

## La fenêtre d’un coup d’œil

![La fenêtre d’Escouade : les onglets des projets en haut, la liste des agents à gauche, la conversation de l’agent choisi au centre et la barre d’état en bas](/images/agents.jpg)

La fenêtre a six zones :

- **la barre des projets**, tout à gauche : une tuile par projet ouvert, la vue d’ensemble, les statistiques et les réglages ;
- **la barre de titre**, en haut : le nom du projet affiché, la recherche et l’éditeur ;
- **la barre latérale**, à côté de la barre des projets : les agents du projet affiché, le lancement du projet et ses terminaux ;
- **la zone principale**, au centre : la conversation de l’agent choisi, ou une autre vue du projet (Kanban, éditeur, terminal, journal d’une commande de lancement) ;
- **le panneau latéral**, à droite, quand il est ouvert : les fichiers non commités, l’historique git, le workflow du projet, et la PR de la branche avec sa CI ;
- **la barre d’état**, en bas : les agents actifs, les quotas, le coût du jour et la branche.

La fenêtre s’ouvre en 1440 × 900 et ne descend pas sous 1000 × 600. Escouade retient sa taille et sa position d’un lancement à l’autre.

Les messages brefs (une confirmation, une erreur, un changement de compte) s’affichent en bas, au centre, au-dessus de la barre d’état, précédés d’une icône (une coche verte pour une réussite, un bouclier rouge pour une erreur), et disparaissent seuls : au bout de 3 secondes et demie, de 7 secondes pour une erreur, de 12 pour un message qui porte un bouton.

Fermer la fenêtre ne quitte pas Escouade et ne coupe pas les agents : elle se range dans la zone de notification (la barre des menus sous macOS). Un clic sur l’icône, « Afficher » dans son menu, ou le lancement d’Escouade une seconde fois la ramène. « Quitter » dans le menu de l’icône arrête l’application ; si des fichiers de l’éditeur ne sont pas enregistrés, la fenêtre revient d’abord et demande « Quitter Escouade ? » (bouton « Quitter quand même »). L’infobulle de l’icône dit combien d’agents attendent une réponse.

## La barre des projets

Une colonne étroite, à gauche, de la barre de titre à la barre d’état, comme les espaces de travail de Slack. De haut en bas : « Vue d’ensemble », une tuile par projet ouvert, le bouton `+` (« Ajouter un projet »), puis, en bas, « Stats » et « Réglages ». Au premier démarrage, sans projet, elle ne garde que `+` et « Réglages ».

### Une tuile de projet

Une tuile est un carré de la couleur du projet avec deux lettres : les initiales des deux premiers mots de son nom (« demo-api » donne « DA »), sinon ses deux premières lettres. La tuile du projet affiché a un trait au bord de la barre et un anneau de sa couleur. Des signes s’ajoutent quand ils ont lieu d’être :

| Signe | Ce qu’il veut dire |
|---|---|
| Pastille ambre avec un nombre, en haut à droite | Autant d’agents du projet attendent ta réponse. |
| Point vert, en bas à droite | Un agent du projet travaille. |
| Cadenas, en bas à gauche | Le projet est privé. Voir [Réglages : Projets](settings-projects). |
| Anneau qui respire | Un agent du projet a posé une question, a terminé ou a échoué pendant que tu regardais ailleurs : rouge pour une erreur, ambre pour une question, vert pour une fin, la plus pressante l’emportant. Il s’arrête quand tu affiches l’agent, la fenêtre au premier plan. |

L’infobulle, à droite de la tuile, donne le nom du projet et son raccourci (`Ctrl 1` à `Ctrl 9`), puis les agents qui attendent et ceux qui travaillent, les agents à voir (« À voir : … ») et les modifications git non commitées (« Δ 7 · modifications non commitées ») ; « Rien en cours, rien de modifié » sinon.

- Un clic affiche le projet. `Ctrl+1` à `Ctrl+9` affichent le projet n.
- Glisse une tuile pour changer l’ordre des projets : sa place reste en creux et un trait dit où elle tombe. `Alt+↑` et `Alt+↓` déplacent la tuile qui a le focus. L’ordre est gardé.
- Un clic droit, ou un clic du milieu, ouvre le menu du projet ; le nom du projet dans la barre de titre ouvre le même :

| Entrée | Ce qu’elle fait |
|---|---|
| « Renommer… » | Ouvre la fenêtre « Renommer le projet ». |
| « Couleur » | Une rangée de pastilles : la couleur du projet, qui teinte l’interface quand il est affiché. |
| « Activer le worktree par agent » ou « Désactiver le worktree par agent » | Bascule l’option pour les nouveaux agents du projet. Voir les [worktrees](worktrees). |
| « Réglages du projet… » | Ouvre les réglages, onglet Projets, sur ce projet. |
| « Ouvrir le dossier » | Ouvre le dossier du projet dans le gestionnaire de fichiers du système. |
| « Monter » et « Descendre » | Déplacent la tuile d’une place (`Alt+↑`, `Alt+↓`). |
| « Fermer le projet… » | Retire le projet de l’application après confirmation : ses agents, ses terminaux et ses lancements s’arrêtent. Son historique (conversations, tickets, réglages) est gardé, et `+` le rouvre ; une case de la confirmation permet de l’effacer aussi. Les fichiers et les worktrees sur le disque ne sont pas touchés. Voir [Réglages : Projets](settings-projects#projets-fermes). |

Quand les projets ne tiennent pas dans la colonne, la liste défile entre « Vue d’ensemble » et `+`, ses bords fondus disant qu’il y en a d’autres. La tuile du projet affiché revient d’elle-même dans la vue, quelle que soit la façon de l’afficher (clic, notification, raccourci, « Vue d’ensemble »). Un projet hors de vue dont un agent attend se signale par une pastille ambre sous la liste (ou au-dessus) : un clic l’amène dans la vue.

### Sa largeur

Le bord droit de la barre se tire pour l’élargir, de 52 à 300 px. Les tuiles gardent leur taille et leurs signes ; jusqu’à 160 px, un projet qui a son libellé (voir [Réglages : Projets](settings-projects)) le montre sous sa tuile, sur une ligne, coupé en « … », celui du projet affiché dans sa couleur. Au-delà, chaque tuile a son nom à sa droite, une rangée par projet : son libellé s’il en a un, sinon le nom du projet, allumé ou non ; « Vue d’ensemble », `+`, « Stats » et « Réglages » ont aussi le leur, et les agents qui attendent passent au bout de la rangée. L’infobulle s’ouvre alors à droite de la barre.

La largeur choisie vaut pour toute l’application et reste après un redémarrage. Un double clic sur le bord revient à la largeur par défaut : 52 px, ou 72 px dès qu’un projet a son libellé. Au clavier, le bord prend le focus avec `Tab` ; `←` et `→` le déplacent de 16 px, `Début` et `Fin` vont aux deux bouts. Dans une fenêtre étroite, la barre se dessine moins large que choisi (au plus la largeur de la fenêtre moins 948 px : 80 px à 1028 px, 300 px dès 1248 px), pour que les agents, la conversation et le panneau gardent leur place ; la largeur choisie revient avec la place.

### Les boutons de la barre

- **« Vue d’ensemble »** (`Ctrl+Maj+A`) affiche les agents de tous les projets sur une page ; le même bouton, ou le même raccourci, la referme. Voir [suivre ses agents](overview).
- **`+`** ouvre l’assistant « Nouveau projet ». Quand des projets ont été fermés, il ouvre d’abord un menu court : « Ajouter un projet… », les derniers projets fermés avec « Rouvrir », et « Tous les projets fermés… » (Réglages › Projets).
- **« Stats »** affiche les statistiques de consommation ; un second clic revient au projet. Voir les [statistiques](stats).
- **« Réglages »** (`Ctrl+,`) ouvre la fenêtre des réglages.

## La barre de titre

Au-dessus de la barre des projets, le logo (sous Windows et Linux). Au-dessus de la barre latérale, le nom du projet affiché : sa pastille, son nom, le cadenas d’un projet privé, le nombre de modifications git non commitées (« Δ 7 ») et une flèche ; un clic ouvre le menu du projet. Au centre, « Rechercher » (`Ctrl+K`) cherche dans les conversations de tous les agents. À droite, « Éditeur » ouvre l’éditeur sur le dossier du projet (« Parcourir et éditer les fichiers du projet », voir [l’éditeur](editor)), puis, sous Windows et Linux, les boutons de la fenêtre (« Réduire », « Agrandir » ou « Restaurer », « Fermer »). Les parties vides de la barre servent à déplacer la fenêtre.

## La barre latérale

### Agents et Kanban

En haut, un sélecteur à deux boutons : « Agents » affiche la conversation d’un agent, « Kanban » le tableau de tickets du projet. Quand des tickets attendent ton test, un badge sur « Kanban » donne leur nombre (« N à tester »). Voir [le Kanban](board).

### La liste des agents

Sous le sélecteur, « Nouvel agent » (`Ctrl+N`) crée un agent sur le harness du projet. La flèche à sa droite, ou `Ctrl+Maj+N`, ouvre le menu « Harness » : chaque harness qu’Escouade connaît, numéroté (`1` à `4` le choisissent tant que le menu est ouvert), avec le compte, le modèle et le mode que prendrait son nouvel agent ; celui du projet est marqué « Celui du projet ». Un harness sans compte reste à sa place avec « Aucun compte » et « Ajouter un compte… » ; un harness introuvable est grisé avec l’endroit où régler son exécutable. Sous « Worktree », « Nouvel agent sans worktree » (ou « dans un worktree », selon le réglage du projet) et « Nouvel agent sur une branche… », puis « Harness par défaut du projet… ». Sans agent, la liste dit « Aucun agent. » avec un bouton « Créer un agent ».

L’agent qui attend ta réponse passe en tête de la liste. Chaque agent est une carte, sans cadre ; celle de l’agent affiché est en relief :

- son **nom** (double-clic pour le renommer sur place : `Entrée` valide, `Échap` annule), le nombre de ses sous-agents et de ses tâches de fond au travail, une icône d’ondes quand le [remote control](remote-control) est activé (teal une fois connecté ; son infobulle dit d’où il vient : de toi, du réglage du projet ou du réglage de tous les agents ; en [vue Claude Code](claude-code-view), elle dit l’état de l’interface), et son **statut** ;
- ce que l’agent annonce faire, quand il l’a dit par le [serveur MCP](mcp) ;
- la marque de son harness (« Claude », « Codex »… dès que les agents tournent sur deux harness), son **modèle**, le temps pendant lequel il a travaillé (« Aucun message » tant que tu ne lui as rien écrit) et une mini-barre « Plan 2/7 » quand il suit un plan (voir [l’avancée d’un plan](plan)), sinon sa branche s’il a un worktree ; pour l’agent d’un ticket, la clé du ticket et sa boucle (« DEM-3 boucle 1/5 », avec une case par boucle) ou « à tester » ;
- **ce qu’il fait** pendant un tour (« Modifie src/auth/jwt.ts », « Réfléchit »), ou après, ce qu’il a changé (« 2 fichiers modifiés »), puis ses **tokens** et son **coût** (précédé de « ≈ » tant qu’un tour tourne : c’est alors une estimation).

La carte d’un agent qui attend ta réponse montre sa demande et ses réponses, numérotées : un clic sur une réponse, ou sa touche `1`, `2`… n’importe où hors d’un champ de saisie, répond sans quitter la conversation affichée. Plusieurs questions viennent l’une après l’autre (« Question 1 sur 2 »). Une autorisation montre la commande ou le fichier, ce que dit le harness, et « Autoriser », « Toujours autoriser » (Claude Code, OpenCode, qui dit le motif qu’il gardera) ou « Pour cette session » (Codex, Copilot CLI), et « Refuser ». Les touches répondent à la carte qui a le focus, sinon à la première de la liste ; elles attendent une demi-seconde après l’arrivée d’une demande. L’agent repart, sa carte brille en vert un instant et reprend sa place. Ce qui ne tient pas sur la carte (une réponse à écrire, un choix multiple, un plan, une demande trop longue ou que Claude Code refuserait par défaut) en montre le début et « Répondre dans la conversation », qui ouvre l’agent.

Sous la carte de l’agent affiché, la liste de ses sous-agents et celle de ses tâches de fond (voir [l’avancée d’un plan](plan)). Sous la carte d’un agent, ses PR (« #41 → main », l’état de leur CI, qui ouvre l’onglet « PR et CI », et le bouton de la correction utile), et les agents qu’il pilote par le serveur MCP.

Un clic sur une carte affiche l’agent. `Ctrl+Tab` et `Ctrl+Maj+Tab` passent à l’agent suivant ou précédent du projet, sans les agents archivés.

### Les statuts et leurs couleurs

| Statut | Point | Texte |
|---|---|---|
| « Prêt » | Anneau gris | Gris |
| « En cours » | Vert | Vert |
| « Question » | Ambre, entouré d’un anneau qui respire | Ambre, la carte en ambre |
| « Attend » ou « Réveil à 15:30 » | Anneau vert vide : le tour est fini, pas le travail | Clair ; l’infobulle dit ce qui continue |
| « Terminé » | Anneau gris | Gris |
| « Erreur » | Rouge | Rouge |

À la place du statut, la carte peut dire « Préparation… » (le worktree de l’agent se prépare) ou « Reprise » suivi d’une heure (l’agent, arrêté par la limite d’usage, reprend alors tout seul), en ambre.

Une carte dont l’agent a posé une question, a terminé ou a échoué pendant qu’il n’était pas affiché s’entoure de sa couleur, en ambre, en vert ou en rouge, jusqu’à ce que tu l’ouvres. `Ctrl+J` met le focus sur la carte de l’agent suivant qui attend ta réponse ou qui est à voir, sans l’ouvrir : `1`, `2`… y répondent, `Entrée` l’ouvre. Avec « Réduire les animations » du système, rien ne bouge.

### Le menu d’un agent

Un clic droit sur une carte ouvre son menu :

| Entrée | Ce qu’elle fait |
|---|---|
| « Renommer » | Renomme l’agent. |
| « Dupliquer la conversation » | Crée une copie de l’agent qui reprend tout son contexte dans une nouvelle session, sans toucher à l’original. Grisée pendant un tour (« Attends la fin de son tour. »). |
| « Archiver » ou « Restaurer » | Range l’agent dans la liste « Archivés (N) » en gardant sa conversation, ou l’en ressort. Si son ticket est en cours ou à tester, une confirmation prévient qu’il repart « À faire ». |
| « Préparer le lancement » | Pour un agent dans un worktree, sans ticket en cours : lui demande comment lancer son worktree, sur ses propres ports. |
| « Intégrer … » | Pour un agent dans un worktree : fait entrer la branche de base du projet dans la sienne. |
| « Ouvrir dans l’éditeur » | Ouvre l’éditeur sur son worktree, sinon sur le dossier du projet. |
| « Ouvrir un terminal » | Ouvre un terminal dans son worktree, sinon dans le dossier du projet. |
| « Vue Claude Code » ou « Vue Escouade » | Change de vue, comme le bouton de l’en-tête de sa conversation. Absente pour un agent de ticket et pour un agent archivé. Voir [la vue Claude Code](claude-code-view). |
| « Activer le remote control » ou « Désactiver le remote control » | Rend la session accessible depuis claude.ai et l’app Claude (« claude.ai, mobile »). Quand le lien est disponible, « Ouvrir sur claude.ai » et « Copier le lien claude.ai » apparaissent. « Désactiver » met toujours l’agent à part : un réglage allumé ensuite ne lui rend pas le remote control. En [vue Claude Code](claude-code-view), « Ouvrir sur claude.ai » et « Copier le lien claude.ai » n’apparaissent qu’une fois l’interface connectée, et le changement vaut au prochain lancement de l’interface. |
| « Supprimer… » | Arrête le processus Claude et retire la conversation de l’application ; la session Claude Code reste sur le disque. Pour un agent avec un worktree, une case propose de supprimer aussi le worktree, et sa branche si elle a été créée pour l’agent. |

Les agents archivés sont sous « Archivés (N) », un titre que tu déplies d’un clic ; leur carte montre le nom et le coût, et en ambre ce que garde un worktree resté (« worktree gardé : 2 commits ni mergés ni poussés »).

### Lancement

La section « Lancement » liste les commandes de lancement du projet (serveur de développement, watchers…) avec leur état (« en cours » en vert, « planté (code 1) » en rouge). Au survol d’une commande, « Relancer » et « Stopper » si elle tourne, sinon « Lancer », qui demande « Dans le projet » ou « Dans le worktree de … » dès qu’un agent a un worktree ; « Tout lancer » et « Tout arrêter » agissent sur toutes. Les lancements d’un agent (dans son worktree, ou ceux d’un test) sont groupés sous son nom. Un clic sur une commande affiche son journal dans la zone principale. Le bouton des réglages ouvre « Commandes de lancement… » ; sans commande, « Configurer » ouvre leurs réglages et « Proposer des commandes » laisse Claude en proposer. Voir [lancer le projet](launch).

### Terminaux

La section « Terminaux » liste les terminaux ouverts du projet. Le bouton `+` (« Nouveau terminal ») propose les shells détectés sur la machine, puis « Dans le worktree de… », qui liste les agents du projet ayant un worktree (pas les archivés) pour y ouvrir un terminal ; l’entrée n’existe pas quand aucun agent n’en a. `Ctrl+T` en ouvre un tout de suite. Un clic sur un terminal l’affiche dans la zone principale, « Fermer » au survol le ferme, et un terminal dont le shell s’est arrêté est marqué « terminé ». Voir [les terminaux](terminals).

Dans une fenêtre de 820 pixels de haut ou moins, « Lancement » et « Terminaux » se replient sur leur titre pour laisser la place aux agents ; une commande plantée y reste dite (« 1 planté »).

### Le pied de la barre

Le pied rappelle le dossier du projet, sa branche git et le décompte de ses fichiers modifiés (`~2`, `+1`, `−0` ; leur détail en infobulle), ou « Pas de dépôt git ». Un clic sur le dossier (« Parcourir ») ouvre l’éditeur sur le dossier du projet.

## L’en-tête et la conversation

![L’en-tête d’un agent, avec ses boutons et ses chiffres, au-dessus de sa conversation et du champ de message](/images/chat.jpg)

L’en-tête de la conversation d’un agent montre, de gauche à droite :

| Élément | Ce qu’il montre ou fait |
|---|---|
| Nom et statut | Le nom de l’agent (double-clic pour le renommer) et son statut, dans une pastille. |
| Harness, `projet / branche` | Le harness de l’agent (quand plusieurs sont installés), le projet, et la branche de l’agent s’il a un worktree, sinon celle du dossier du projet ; puis le numéro de ses pull requests, et « Bac à sable » pour un agent Codex. |
| « Contexte » | Les tokens du contexte sur la taille de la fenêtre du modèle (« 76,2 k / 200 k »), avec une jauge fine dessous. Le chiffre devient ambre à partir de 80 %, quand le contexte est presque plein. |
| « Tokens », « Coût », « Durée » | Ce que l’agent a consommé et le temps qu’il a travaillé. |
| « Fichiers » | Le nombre de fichiers non commités de l’agent. |
| « Ouvrir une PR », « Tester » ou « Préparer le lancement » | Une seule place : ouvrir la pull request de sa branche tant qu’elle a des commits sans PR ; sinon, pour un agent dans un worktree, lancer son travail pour l’essayer, ou demander à l’agent comment le lancer. Voir [tester et valider](testing). |
| « Escouade » et « Claude Code » | Les deux vues de l’agent, en segments : la conversation d’Escouade, ou l’interface de Claude Code elle-même. Absentes pour un agent de ticket. Voir [la vue Claude Code](claude-code-view). |
| « … » | Le menu de l’agent, le même que le clic droit sur sa carte : l’éditeur, un [terminal](terminals) dans son worktree, renommer, archiver… |
| Deux boutons | Le choix de la disposition (voir plus bas). |

Sous l’en-tête, la conversation montre tes messages, les réponses de Claude en markdown, ses réflexions repliables, ses appels d’outils en lignes compactes dépliables, les cartes de questions et d’autorisations, et une ligne ou une carte à la fin de chaque tour. Quand l’agent suit un plan, un bandeau « Plan » s’intercale entre l’en-tête et les messages : voir [l’avancée d’un plan](plan). Une longue conversation n’affiche que ses 80 derniers éléments : « Afficher les N précédents » en ajoute. Si de nouveaux messages arrivent pendant que tu lis plus haut, le bouton « Nouveaux messages » te ramène en bas. `Ctrl+K` cherche dans les conversations.

Le champ de message flotte en bas, de la largeur de la conversation : ses puces « Modèle », « Effort » et « Mode », puis joindre un fichier, dicter et converser à voix haute, « Stop » pendant un tour et « Envoyer ». Un agent archivé affiche à la place « Agent archivé. » avec un bouton « Restaurer ». Un agent en erreur le dit (« L’agent s’est arrêté. ») : un nouveau message relance Claude sur la même session. Voir [la conversation](chat) et [les agents](managing-agents).

## Le panneau latéral

Le panneau latéral a quatre onglets : « Non commités », suivi du nombre de fichiers, « Historique », « Workflow » et « PR et CI » (« MR et CI » sur GitLab). Large de 340 px, le panneau n’a pas la place du dernier : l’état de la CI sur la carte de l’agent l’ouvre alors.

### Non commités

- Le sélecteur « Cet agent » et « Tout le projet » choisit la portée de la liste. Sous lui, une ligne dit d’où viennent les fichiers : le worktree de l’agent (« isolé des autres agents »), ses éditions, ou tous les agents du projet.
- Chaque ligne est un fichier : sa lettre (M, A ou D), son nom, son dossier, ses lignes ajoutées et retirées, et pour « Tout le projet » le nom de l’agent. Le bouton `</>` d’une ligne ouvre le fichier dans l’éditeur. Un clic droit propose « Ouvrir dans l’éditeur » puis, selon la lettre du fichier, « Abandonner les modifications… » (M) ou « Supprimer le fichier… » (A), toujours après confirmation, ou « Restaurer le fichier » (D).
- En bas : « Voir le diff », « Commit… » (« Commit tout… » pour « Tout le projet ») et, pour un agent dans un worktree, « Merger … → … » et « Intégrer … ».

La liste n’affiche que 500 fichiers ; le diff et le commit les prennent tous. Voir [Git et commits](git).

### Historique

L’onglet « Historique » montre le graphe git de tout le dépôt, avec la branche de l’agent mise en avant. Un clic sur un commit ouvre son diff ; un clic droit propose des actions sur les branches. Voir [les branches](branches).

### PR et CI

L’onglet « PR et CI » montre ce que l’hébergeur du dépôt (GitHub, Azure DevOps) dit de la branche de l’agent ouvert. Une pastille rouge sur l’onglet dit que la CI de sa PR a échoué.

- **Sa PR** en tête : numéro, titre, état (ouverte, brouillon, mergée, fermée), branche → cible, un conflit avec la cible, son âge et son auteur ; l’icône à droite l’ouvre dans le navigateur. « L’agent corrige » et « L’agent corrige les retours » lui donnent les échecs ou les retours, sur ton clic seulement. Ses autres PR sont listées en bas.
- **La CI** : un résumé (« 2 échecs sur 14 ») puis chaque vérification, les échecs d’abord, avec son état (en attente, en cours, réussie, échouée, annulée) et sa durée. Un clic sur une vérification échouée ouvre la fin de son journal. « Relancer les échecs » relance les jobs en échec du dernier commit, sur ton clic seulement. Tant qu’une vérification tourne et que l’onglet est affiché, la CI se relit toutes les 20 s ; jamais en arrière-plan.
- **Les revues et commentaires**, en lecture : qui a approuvé ou demandé des changements, les commentaires avec leur fichier et leur ligne. Escouade n’y publie rien en ton nom.
- **Les déploiements** de la branche, quand l’hébergeur les donne (GitHub) : l’environnement, son état et l’adresse de prévisualisation à ouvrir.

Sans PR sur sa branche, l’onglet propose « Ouvrir une PR… », ou dit pourquoi il ne peut pas : pas de branche à l’agent, pas de dépôt distant, un hébergeur qu’Escouade ne lit pas encore (GitLab, Bitbucket, Gitea), ou pas de compte, avec le lien vers Réglages › Intégrations. Dans un projet sans agent, « PR et CI du projet » ouvre la liste des PR ouvertes du projet, chacune avec l’état de sa CI ; un clic sur celle d’un agent ouvre cet agent. Un jeton refusé, la limite d’API de l’hébergeur (jusqu’à quelle heure) ou un hébergeur injoignable sont dits dans l’onglet, avec « Réessayer ».

## Les deux dispositions

Le choix se fait avec les deux boutons à droite de l’en-tête, « Disposition classique » et « Conversation et fichiers côte à côte », ou avec `Ctrl+Maj+L`. Escouade garde ton choix d’un lancement à l’autre.

| Élément | Classique | Côte à côte |
|---|---|---|
| Panneau latéral | Optionnel, large de 330 px : « Fichiers » dans l’en-tête ou `Ctrl+Maj+B` l’ouvre, le × le ferme. | Toujours affiché, sur la moitié droite de la zone principale. `Ctrl+Maj+B` ne fait rien. |
| Clic sur un fichier | Le diff s’ouvre dans une grande fenêtre. | Le diff s’affiche sous la liste, dans le panneau. |
| « Fichiers » dans l’en-tête | Un bouton qui ouvre le panneau. | Un simple chiffre. |

![La disposition côte à côte : la conversation à gauche, les fichiers non commités et le diff du fichier choisi à droite](/images/git.jpg)

Quand la conversation est étroite, ce qui arrive en disposition côte à côte, l’en-tête retire d’abord « Tokens » et « Fichiers », puis « Durée » et le texte de ses boutons, puis ses chiffres. Les cartes de la barre latérale montrent les mêmes chiffres.

## La barre d’état

De gauche à droite :

| Élément | Ce qu’il montre |
|---|---|
| `N actifs` | Le nombre d’agents « En cours ». |
| `N en attente Ctrl J` | Le nombre d’agents « Question », en ambre quand il y en a. Un clic met le focus sur la carte de l’agent suivant qui attend ou qui est à voir (`Ctrl+J`). |
| `N terminés` | Le nombre d’agents « Terminé ». |
| `N agents · mémoire · CPU` | Les processus des agents en cours, avec les outils et les serveurs MCP qu’ils lancent ; l’infobulle détaille agent par agent. |
| Mise à jour | « Mise à jour X prête · Redémarrer » quand une mise à jour est téléchargée : un clic ouvre la fenêtre de la mise à jour. Pendant le téléchargement, la barre dit « Mise à jour X… » ; quand un redémarrage automatique est prévu, « Redémarrage dans N s » et le bouton « Plus tard » prennent la place. Voir les [mises à jour](updates). |
| `Claude 5 h 22 % 7 j 36 %` | Un groupe de jauges par harness qui fait travailler des agents (celui de l’agent affiché d’abord) : le nom du harness, puis chaque fenêtre de quota du compte où partent ses nouveaux agents, une barre fine (ambre au-delà de 80 %) et son pourcentage. La remise à zéro est dans l’infobulle. Une projection (« → 86 % ») ne s’affiche qu’au-delà de 80 %, en orange, en rouge au-delà de 100 %. Un harness sans quota (OpenCode) montre son coût du jour sur son budget (« 0,42 $ / 5 $ »). « usage supplémentaire », en ambre, dit qu’un compte du harness répond au-delà de son quota, facturé à part. Un clic ouvre le panneau « Quotas des comptes » : chaque compte, groupé par harness, avec toutes ses fenêtres, leurs projections et leur remise à zéro, son coût du jour, le budget d’OpenCode et les crédits de Copilot CLI. Sous 1240 pixels de large, les pourcentages, les projections et « usage supplémentaire » laissent la place : le panneau les écrit. |
| `Aujourd'hui 3,48 $` | Ce que tes agents ont coûté aujourd’hui, précédé de « ≈ » quand une part en est estimée. |
| `main ↑1 ↓3` | La branche du projet affiché, avec les commits à pousser et à tirer. Un clic ouvre le sélecteur de branches. « non publiée » ou « distante supprimée » remplace les flèches quand la branche n’a pas de suivi. |
| Haut-parleur | Active ou coupe le son des notifications. |

Voir [les quotas et le coût](usage) et [les branches](branches).

## Les réglages

`Ctrl+,` ou le bouton « Réglages », en bas de la barre des projets, ouvre la fenêtre des réglages : les onglets à gauche, le contenu à droite. Un onglet modifié est marqué ; « Enregistrer » enregistre tous les onglets d’un coup et « Annuler » (ou `Échap`) ferme la fenêtre sans enregistrer. L’onglet « Comptes » fait exception : il enregistre à chaque changement. Les onglets « Projets », « Kanban » et « Intégrations » règlent un projet à la fois, choisi en haut ; « Réglages du projet… » dans le menu d’un projet ouvre la fenêtre sur ce projet.

| Onglet | Page |
|---|---|
| « Application » | [Réglages de l’application](settings-application) |
| « Harness » | [Réglages des harness](settings-claude-code) |
| « Comptes » | [Réglages des comptes Claude](settings-accounts) |
| « Notifications » | [Réglages des notifications](settings-notifications) |
| « Projets » | [Réglages des projets](settings-projects) |
| « Kanban » | [Réglages du Kanban](settings-board) |
| « Intégrations » | [Réglages des intégrations](settings-integrations) |
| « Terminaux » | [Réglages des terminaux](settings-terminals) |
| « Réseau » | [Réglages du réseau](settings-network) |
| « À propos » | [Réglages « À propos »](settings-about) |

## Sous macOS

- **Barre de titre native.** macOS dessine lui-même les boutons de la fenêtre (rouge, jaune, vert) à gauche de la barre de titre, au-dessus de la barre des projets, à la place du logo ; Escouade n’affiche pas les siens. Le bouton rouge range la fenêtre dans la barre des menus, comme sous Windows et Linux, et `⌘Q` ou « Quitter Escouade » arrête l’application.
- **`⌘` à la place de `Ctrl`.** Tous les raccourcis de cette page prennent `⌘` (`⌘N`, `⌘,`, `⌘K`, `⇧⌘L`…), sauf `Ctrl+Tab` et `Ctrl+Maj+Tab`, parce que `⌘Tab` change d’application. Dans les terminaux, `⌘C` et `⌘V` copient et collent, et les touches `Ctrl` vont au shell.
- **Barre des menus.** Escouade a ses menus macOS : le sien (« À propos d’Escouade », « Masquer Escouade », « Quitter Escouade »…), « Édition », « Présentation », « Fenêtre » et « Aide », dans la langue de l’interface.
- **Notifications.** L’icône du Dock rebondit quand un agent a besoin de toi, là où la barre des tâches clignote sous Windows. Voir [les notifications](notifications).
