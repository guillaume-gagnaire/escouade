# Lancer le projet

Les commandes de lancement démarrent les processus dont tu as besoin pour développer un projet (serveur, API, base de données, worker…), chacun dans son propre terminal en lecture seule. Cette page explique comment les définir, les lancer, suivre leur état et laisser Claude les proposer.

![La section Lancement de la barre latérale, avec trois commandes dont une plantée, et le log du serveur de développement](/images/launch.jpg)

## À quoi ça sert

Une commande de lancement est une ligne de commande que tu relances souvent en développant : le serveur du front, l’API, une base locale, un worker. Elle est rangée avec le projet. Elle tourne dans son propre terminal, dont le log reste d’un lancement à l’autre, et tu la lances, la stoppes ou la relances d’un clic, seule ou avec les autres.

Ce n’est pas un [terminal](terminals) ordinaire : le log est en lecture seule, et Escouade suit le processus pour dire s’il tourne, s’il s’est arrêté ou s’il a planté.

## Définir les commandes

Les commandes se règlent par projet, dans Réglages › Projets › Lancement. Le plus court est le bouton d’engrenage de la section « Lancement » de la barre latérale (« Commandes de lancement… »), ou « Configurer » quand il n’y a encore aucune commande : ils ouvrent les réglages du projet sur ce groupe. Chaque commande a quatre champs :

| Champ | Rôle |
|---|---|
| « Nom » | Le nom dans la liste et dans l’en-tête du log (par exemple « Front »). Il est obligatoire. |
| « Shell » | Le shell qui exécute la commande, parmi ceux que l’app détecte. Une commande réglée sur un shell absent de cette machine (copiée d’un autre système, par exemple) tourne dans le premier shell détecté, et la liste la marque « bash (introuvable) ». |
| « Commande » | La ligne de commande (par exemple `npm run dev`). Elle est obligatoire. |
| « Sous-dossier » | Le dossier où elle tourne, relatif au dossier du projet (par exemple `apps/web`). Vide, c’est le dossier du projet. Il doit exister : sinon la commande ne démarre pas et le log dit « Le dossier « apps/web » n’existe pas dans le projet ». |

« + Ajouter une commande » en ajoute une, et la croix de la commande la supprime. Rien n’est pris en compte avant « Enregistrer », qui reste grisé tant qu’une commande n’a pas de nom ou de ligne de commande (« Chaque commande de lancement de « demo-api » demande un nom et une ligne de commande. »). Une commande retirée des réglages et enregistrée s’arrête et son log disparaît. La liste complète des réglages du projet est dans [les réglages des projets](settings-projects).

Les commandes lancées dans le projet ne reçoivent aucune variable `ESCOUADE_…` (dans le worktree d’un agent, elles reçoivent ses ports : voir plus bas). Pour donner une variable d’environnement à un processus qui ne la lit pas dans un fichier `.env`, écris-la devant la commande, avec la syntaxe de son shell : `$env:PORT = '3000'; npm run dev` sous PowerShell, `PORT=3000 npm run dev` sous bash et zsh. Quand « Appliquer aussi le proxy aux terminaux intégrés » est activé (voir [les réglages du réseau](settings-network)), les commandes reçoivent les variables du proxy.

## Lancer, relancer, stopper

La section « Lancement » de la barre latérale liste les commandes du projet. Elle affiche, à côté de son titre, combien tournent (`2/3`). Chaque ligne donne un point de couleur, le nom de la commande, son état et ses boutons ; l’info-bulle de la ligne montre la ligne de commande. Un clic sur la ligne affiche le log de la commande dans la zone principale.

| État | Ce que ça veut dire |
|---|---|
| « prêt » | La commande n’a pas encore été lancée depuis le démarrage d’Escouade. |
| « en cours » | Le processus tourne. |
| « arrêt… » | Tu as demandé l’arrêt, il n’est pas encore fini. |
| « arrêté » | Tu l’as stoppée. |
| « terminé » | Elle s’est terminée toute seule, avec le code 0. |
| « planté (code 2) » | Elle s’est terminée avec une erreur. Un message s’affiche aussi dans la fenêtre : « « Worker » s’est arrêté en erreur (code 2) ». Escouade n’envoie pas de notification du système pour autant. Sans code de sortie, l’état est « planté ». |

Les boutons de la ligne :

- « Lancer » (▶) démarre la commande. Il est remplacé par les deux suivants tant qu’elle tourne.
- « Relancer » l’arrête puis la relance. Escouade attend jusqu’à 10 secondes que le processus s’arrête ; s’il tourne encore, la commande reste comme elle est.
- « Stopper » arrête la commande et les processus qu’elle a lancés. Tant que l’arrêt n’est pas fini, la ligne dit « arrêt… ».

En haut de la section, « Tout lancer » démarre toutes les commandes en même temps. Dès qu’une tourne, le bouton devient « Tout arrêter ».

Quand une commande ne démarre pas, un message le dit (« « Front » n’a pas pu démarrer : … ») et le log écrit la raison en rouge. Fermer le projet arrête ses commandes sans les compter comme plantées.

## Le log d’une commande

Le log d’une commande occupe la zone principale quand tu cliques sa ligne. L’en-tête donne son nom, son shell, son état (« en cours », « depuis 14:02 »), la ligne de commande et le dossier où elle tourne. À droite : la loupe, « Effacer » et les boutons d’action, qui suivent l’état : « Relancer » et « Stopper » pendant qu’elle tourne, « Relancer » une fois finie, « Lancer » quand elle n’a jamais tourné (le log dit « Pas encore lancée. »).

- Le log commence par la ligne de commande (`$ npm run dev`). Le programme écrit ensuite ce qu’il veut, couleurs comprises. Il garde 10 000 lignes.
- Le log est en lecture seule : on le fait défiler (molette, `Maj+PageUp`) et on y cherche, mais les touches ne vont pas au processus. Le processus suit la taille de la fenêtre. Comme dans un terminal, une adresse `http://` ou `https://` du log s’ouvre dans le navigateur d’un simple clic.
- Il reste d’un lancement à l’autre : une relance ajoute « — relancé à 14:32 — » à la suite. Il disparaît quand la commande est retirée des réglages ou quand le projet est fermé.
- La loupe, ou `Ctrl+Maj+F` (`⇧⌘F` sous macOS) dans le log, ouvre un champ de recherche : `Entrée` va à l’occurrence suivante, `Maj+Entrée` à la précédente, `Échap` ferme le champ. « Effacer » vide l’écran du log.

Les lancements de test des agents (le bouton « ▶ Tester », voir [les tests](testing)) apparaissent dans la même section, sous le nom de l’agent, une fois lancés. Ils se suivent de la même façon.

## Dans le worktree d’un agent

Une commande se lance aussi dans le worktree d’un agent, pour voir son travail tourner sans le fusionner.

- Dès qu’un agent du projet a un worktree, « Lancer » (▶) ouvre un menu : « Dans le projet », puis « Dans le worktree de <agent> » pour chaque agent qui en a un (pas les archivés). Sans worktree, ▶ lance dans le projet, comme avant. « Tout lancer » lance toujours dans le projet, et dit d’abord quelles commandes tournent déjà dans un worktree.
- Au clic droit sur la carte d’un agent qui a un worktree, « Lancer… » liste les commandes du projet ; chacune se lance dans son worktree.
- La commande tourne dans le même sous-dossier, pris dans le worktree. S’il n’y existe pas (la branche de l’agent ne l’a pas, ou pas encore), elle ne démarre pas et le message dit pourquoi : « Le dossier « web » de « Front » n’existe pas dans le worktree de « refacto-auth » (sa branche ne l’a pas, ou pas encore). »
- Elle apparaît sous le nom de l’agent, avec son propre état et son propre log ; son terminal s’appelle « Front · refacto-auth ». La ligne du projet garde le sien.
- Elle reçoit `ESCOUADE_PORT_BASE` et `ESCOUADE_PORT_END`, le bloc de 10 ports de l’agent (réservé à ce moment s’il n’en a pas), le même que ses lancements de test. Pour que deux instances ne se disputent pas un port, fais lire ce port à la commande, par exemple `npm run dev -- --port $env:ESCOUADE_PORT_BASE` sous PowerShell ou `npm run dev -- --port ${ESCOUADE_PORT_BASE:-5173}` sous bash.
- Si la même commande tourne déjà ailleurs (dans le projet ou dans un autre worktree), une fenêtre le dit avant de lancer : « Arrêter l’autre et lancer », ou « Lancer quand même ».
- Elle s’arrête avec l’agent : à son archivage, à sa suppression, à la validation de son ticket (qui retire son worktree), et quand le projet se ferme.
- Un worktree neuf n’a pas les dépendances du projet (`node_modules`, environnement Python…) : c’est le rôle des étapes de préparation des worktrees (Réglages › Projets › Git et worktrees, groupe « Worktrees »), qui tournent à sa création.

## Remplir automatiquement

Plutôt que d’écrire les commandes, tu peux demander à Claude de les proposer.

1. Dans la section « Lancement » d’un projet sans commande, clique « Proposer des commandes » : les réglages s’ouvrent sur le groupe « Lancement » et la lecture démarre. Tu peux aussi cliquer « ✦ Remplir automatiquement » dans ce groupe, dans les réglages du projet.
2. Le bouton dit « Claude lit le projet… ». Claude lit les manifestes et les scripts (`package.json`, `Makefile`, `Procfile`, `justfile`, `pyproject.toml`…), les fichiers `docker-compose`, le README et le CONTRIBUTING, à la racine et dans les sous-dossiers. Il n’a que des outils de lecture : il ne modifie rien et n’exécute rien. Il a 4 minutes au plus.
3. Un message dit combien de commandes sont proposées (« 3 commandes proposées : relis-les avant d’enregistrer. »), et un encadré « Commandes de lancement proposées » les montre, chacune avec son nom, sa ligne de commande en entier, son shell et son dossier (« le dossier du projet » quand il est vide).
4. « Remplacer les commandes » met la proposition à la place de la liste du brouillon, et « Ignorer » l’abandonne. Rien n’est gardé tant que tu n’as pas cliqué « Enregistrer ».

> [!WARNING]
> Ces commandes tourneront dans ton shell, avec tes droits. Lis chacune en entier avant d’enregistrer : « Remplacer les commandes » remplace toutes celles de la liste.

Ce que Claude doit proposer : les processus dont on a besoin pour développer le projet (serveurs de développement, watchers, workers et files de tâches, base de données et services locaux), un par commande, avec un nom court. Pas de commande d’installation ni de tests, ni de commande qui se termine d’elle-même (build, lint, migration). Une commande qui rend la main aussitôt (`docker compose up -d`) ne se suit pas dans un terminal : il doit la donner au premier plan (`docker compose up db`). Huit commandes au plus. Toutes tournent dans le premier shell détecté.

### Ce qui est écarté

Une commande proposée est écartée, et comptée dans le message, quand elle tient sur plusieurs lignes, contient un caractère qui n’affiche rien ou qui change l’ordre de lecture (espace de largeur nulle, marque de direction…), contient une suite de 24 espaces ou plus, ou dépasse 300 caractères. La raison : les réglages montrent une commande sur une seule ligne, et ce que tu lis dans le champ doit être ce qui s’exécute. Le message dit alors, par exemple, « 2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant d’enregistrer. ». Une suite plus longue se range dans un script du projet, que la commande appelle.

D’autres commandes sont abandonnées sans être comptées : celles qui n’ont pas de ligne de commande, celles dont le sous-dossier n’est pas dans le projet, et celles qui dépassent huit. Un nom de plus de 40 caractères est raccourci.

Si Claude ne trouve rien à lancer, le message dit « Claude n’a trouvé aucune commande à lancer pour ce projet. ». Si toutes ses commandes sont écartées, l’erreur dit « Claude a proposé des commandes illisibles : aucune n’a été gardée. »
