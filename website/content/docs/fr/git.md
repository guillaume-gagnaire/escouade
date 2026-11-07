# Git : changements, diff et commit

Cette page explique comment voir ce que tes agents ont modifié, lire un diff, commiter, parcourir l’historique du dépôt et afficher la conversation et les fichiers côte à côte.

## Le panneau des fichiers non commités

Le panneau s’ouvre à droite de la conversation d’un agent. Dans la disposition classique, le compteur « Fichiers » de l’en-tête de la conversation l’ouvre et le ferme, comme `Ctrl+Maj+B` (`⇧⌘B` sous macOS) ; la croix du panneau le referme. Ses onglets : « Non commités », suivi du nombre de fichiers, « Historique », décrit [plus bas](#historique-et-git-graph), « Workflow », et « PR et CI » : la PR de la branche, sa CI, ses revues et ses déploiements (voir [l’interface](interface#pr-et-ci)).

Sous les onglets, deux boutons choisissent ce que la liste montre :

| Bouton | Ce que tu vois |
|---|---|
| « Cet agent » | Les fichiers de l’agent. S’il a un [worktree](worktrees), ceux de son worktree (« worktree .claude/worktrees/refacto-auth · isolé des autres agents »). Sinon, les fichiers qu’il a modifiés dans le dossier du projet, d’après ses éditions. |
| « Tout le projet » | Les fichiers du dossier du projet et ceux du worktree de chaque agent, chacun étiqueté du nom de l’agent à qui il appartient. |

Chaque ligne donne la lettre de l’état du fichier (`M` modifié en ambre, `A` ajouté en vert, `D` supprimé en rouge), son nom, son dossier, puis le nombre de lignes ajoutées (`+29`) et supprimées (`−3`). Un fichier nouveau et pas encore suivi par git compte comme ajouté. La liste se met à jour toute seule pendant que l’agent travaille, et dès que quelque chose change dans le dossier. Elle affiche au plus 500 lignes, puis « … et N autres fichiers » ; le diff et le commit portent sur tous les fichiers.

Quand il n’y a rien, la liste dit « Aucun fichier modifié par cet agent. » ou « Aucune modification non commitée. »

### Ce qu’on peut faire d’un fichier

- Un clic sur une ligne ouvre son diff (disposition classique) ou l’affiche sous la liste ([disposition côte à côte](#cote-a-cote-conversation-et-fichiers)).
- Le bouton `</>` qui apparaît à droite de la ligne quand tu la survoles, ou « Ouvrir dans l’éditeur » dans son menu (clic droit), ouvre le fichier dans [l’éditeur](editor), dans le worktree de l’agent s’il en a un. Pour un fichier supprimé, le bouton n’existe pas et l’entrée du menu est grisée.
- Le clic droit propose aussi de défaire le fichier :

| Entrée du menu | Pour un fichier… | Ce qui se passe |
|---|---|---|
| « Abandonner les modifications… » | modifié | Après confirmation, il revient à son état du dernier commit : ses modifications non commitées sont perdues. |
| « Supprimer le fichier… » | ajouté | Après confirmation, il est supprimé du disque, sans retour possible, parce qu’il n’a jamais été commité. |
| « Restaurer le fichier » | supprimé | Il revient à son état du dernier commit, sans confirmation. |

> [!WARNING]
> « Abandonner les modifications » et « Supprimer le fichier » ne passent pas par la corbeille : on ne revient pas en arrière.

### Les boutons du bas

- « Voir le diff » ouvre la [visionneuse](#la-visionneuse-de-diff) sur tous les fichiers de la liste (pour « Tout le projet », ceux du projet et de chaque worktree).
- « Commit… » (« Cet agent ») ou « Commit tout… » (« Tout le projet ») lance [un commit](#commiter). Les boutons sont grisés quand la liste est vide.
- Pour un agent qui a un worktree, « Merger escouade/refacto-auth → main… » fusionne son travail dans sa branche de base, et « Intégrer main » y fait entrer la base : voir [un worktree par agent](worktrees) et [les branches](branches).

## La visionneuse de diff

La visionneuse est une grande fenêtre. Elle s’ouvre avec « Voir le diff », par un clic sur un fichier de la liste (disposition classique), par un clic sur un commit de l’historique ou par une comparaison de branches. Son titre dit ce qu’elle montre (« Modifications de refacto-auth », « 4f2a9c1 · feat: pagination », « main ↔ feat/login ») et le nombre de fichiers. Avec plusieurs fichiers, la liste est à gauche, avec l’état de chacun, son chemin, l’agent à qui il appartient et ses lignes `+`/`−` ; le diff du fichier choisi est à droite. `Échap` ou la croix la ferme.

Deux présentations, au choix avec les boutons « Unifié » et « Côte à côte » :

- « Unifié » : une seule colonne, avec les numéros de l’ancienne et de la nouvelle ligne, et un signe `+` ou `−` devant chaque ligne changée ;
- « Côte à côte » : l’ancienne version à gauche, la nouvelle à droite, les lignes appariées l’une en face de l’autre. Les longues lignes passent à la ligne pour que les deux colonnes restent alignées.

Le choix est gardé d’une session à l’autre. Il vaut partout : la visionneuse, le diff du fichier choisi dans le panneau et les diffs que la conversation montre sous les appels d’outils d’édition de l’agent.

Quelques cas particuliers :

| Ce que dit la visionneuse | Pourquoi |
|---|---|
| « Aucune différence. » | Le fichier est revenu à son état d’origine. |
| « Fichier binaire. » | Le fichier est binaire : il n’y a pas de diff à lire. |
| « Diff trop volumineux pour être affiché. » | Le diff de ce fichier dépasse environ 4 Mo. |
| « Diff volumineux (N lignes) » et le bouton « Afficher » | Le diff a plus de 1 500 lignes. Il reste replié tant que tu ne le demandes pas. |

Un diff déplié s’affiche par tranches de 500 lignes : le bouton « Afficher 500 lignes de plus » ajoute la suivante.

## Commiter

Les boutons « Commit… » et « Commit tout… » du panneau suivent le réglage « Commit » du projet (Réglages › Projets, groupe « Git »). Il a deux valeurs : « Rédigé par l’agent », par défaut, et « Direct, avec un message proposé ». La liste complète des réglages du projet est dans [les réglages des projets](settings-projects).

### Faire commiter l’agent

C’est le comportement par défaut. « Commit… » envoie un message à l’agent : il doit commiter les modifications qu’il a faites dans le dépôt, avec un message clair au format Conventional Commits, n’inclure que les fichiers qu’il a modifiés et faire plusieurs commits s’il y a plusieurs sujets. Un message « Demande de commit envoyée à refacto-auth » le confirme, et la suite se passe dans la conversation, comme pour tout message : l’agent peut demander des autorisations, par exemple pour lancer `git`.

« Commit tout… » demande à l’agent choisi de commiter toutes les modifications en cours du dépôt, regroupées en commits cohérents. Il faut donc qu’un agent soit sélectionné.

Le message est envoyé dans la langue réglée par « Langue des textes rédigés par les agents » (voir [les réglages de l’application](settings-application)).

### Commiter toi-même, avec un message proposé

Avec « Direct, avec un message proposé », « Commit… » ouvre la fenêtre « Commit ». Elle indique ce qui est commité (« Modifications de refacto-auth »), liste les fichiers du commit et propose le champ « Message ».

1. Haiku lit la liste des fichiers et leur diff, ainsi que les derniers sujets de commit du dépôt, et propose un message dans le même style (« Haiku rédige le message… »). Ça peut prendre quelques secondes, 90 au plus.
2. Relis le message et corrige-le comme tu veux. Si tu commences à écrire avant l’arrivée de la proposition, elle ne remplace pas ton texte. « Régénérer » en demande une autre, qui remplace le contenu du champ.
3. « Commiter » crée le commit, ferme la fenêtre et affiche « Commit 4f2a9c1 créé ».

Ce que tu dois savoir :

- Rien n’est commité sans que tu cliques « Commiter ».
- Le commit prend exactement les fichiers de la liste, nouveaux, modifiés et supprimés, et rien d’autre : ce qui était déjà indexé par ailleurs reste indexé, hors du commit. La liste n’a pas de cases à cocher : le commit prend tous les fichiers listés.
- Les hooks git s’exécutent. Si l’un refuse le commit, son message s’affiche dans la fenêtre, avec ton texte intact.
- Si Haiku ne propose rien, la fenêtre dit « Pas de proposition : … » et tu écris le message.
- « Cet agent » commite le worktree de l’agent, ou les fichiers qu’il a modifiés dans le dossier du projet. « Tout le projet » commite le dossier du projet seul : le worktree d’un agent se commite depuis « Cet agent ».
- Les fichiers que le projet copie dans ses worktrees (les `.env*`, par défaut) ne sont jamais commités. La fenêtre le dit (« Jamais commité : .env (copié dans les worktrees). ») et leur contenu n’est pas montré à Haiku.
- Par défaut, le message proposé reprend la langue des derniers commits du dépôt. Si tu règles « Langue des textes rédigés par les agents », c’est cette langue qui est demandée.
- Fermer la fenêtre avec un message que tu as écrit ou modifié demande confirmation (« Abandonner le message ? »).

## Historique et git graph

L’onglet « Historique » dessine le graphe de tout le dépôt : les 300 derniers commits de toutes les branches, locales et distantes, et les tags, avec les lignes qui montrent d’où chaque branche part et où elle est mergée. Chaque ligne donne le commit, ses étiquettes, son sujet et son âge ; le survol d’un commit affiche son hash court et son auteur.

- La branche de l’agent est mise en avant : ses commits sont en pleine couleur, avec un point plus gros, et le reste est estompé. Au premier affichage, la liste défile jusqu’à elle. Sans agent à worktree, c’est la branche du dossier du projet qui est mise en avant.
- Les étiquettes distinguent la branche en cours (en couleur d’accent), les branches d’agents (affichées sous le nom de l’agent), les branches distantes (contour) et les tags.
- Un clic sur un commit ouvre son diff dans la [visionneuse](#la-visionneuse-de-diff). Pour un commit de merge, le diff est celui par rapport au premier parent.
- Un clic droit (ou la touche Menu, ou `Maj+F10`) ouvre le menu du commit.

| Entrée du menu | Effet |
|---|---|
| « Créer une branche ici… » | Ouvre « Nouvelle branche », qui part de ce commit. |
| « Passer sur feat/login » | Bascule le dossier du projet sur cette branche locale. Une entrée par branche locale qui pointe sur le commit, sauf celle du dossier du projet. |
| « Comparer avec la branche courante » (« Comparer avec HEAD » sur une HEAD détachée) | Ouvre la visionneuse avec ce qui change entre la branche courante et ce commit, titrée « main ↔ feat/login ». L’entrée est absente sur le commit où le dossier du projet se trouve. |
| « Supprimer la branche… » | Supprime une branche locale qui pointe sur le commit (pas celle du dossier du projet), après confirmation. Avec plusieurs branches sur le commit, l’entrée nomme celle qu’elle supprime. |

Une branche tenue par le worktree d’un agent ne peut ni recevoir de bascule ni être supprimée : l’entrée est grisée et dit pourquoi. Les questions que posent ces actions (changements non commités, branche pas encore mergée) sont décrites dans [les branches](branches).

## Côte à côte : conversation et fichiers

`Ctrl+Maj+L` (`⇧⌘L` sous macOS), ou le bouton « Conversation et fichiers côte à côte » à droite de l’en-tête de la conversation, change la disposition. Le bouton voisin, « Disposition classique », revient en arrière. Le choix est gardé d’un redémarrage à l’autre.

![La conversation d’un agent à gauche, ses fichiers modifiés et le diff d’un fichier côte à côte à droite](/images/git.jpg)

Dans cette disposition, la conversation prend la moitié gauche et le panneau, toujours affiché, la moitié droite. La liste des fichiers est en haut (environ un tiers de la hauteur au plus), le diff du fichier choisi est en dessous, sur toute la largeur disponible. Un clic sur une ligne change de fichier ; le premier est choisi au départ, et le fichier choisi reste affiché quand la liste change. Liste et diff se rafraîchissent pendant que l’agent travaille : tu lis ses modifications au fur et à mesure.

Le panneau étant toujours là, `Ctrl+Maj+B` n’a plus d’effet et « Fichiers » devient un simple compteur dans l’en-tête. Les onglets « Non commités » et « Historique », les boutons du bas et la visionneuse (« Voir le diff ») sont les mêmes.

## Les modifications du projet d’un coup d’œil

- Le nom du projet affiché, dans la barre de titre, et l’infobulle de la tuile de chaque projet affichent « Δ 7 » quand le projet a des fichiers non commités (l’info-bulle dit « Modifications git non commitées »). Le nombre compte les fichiers modifiés, ajoutés ou supprimés dans le dossier du projet et dans les worktrees de ses agents. Il disparaît à zéro.
- Dans la barre latérale, la carte d’un agent dont le tour est fini indique les fichiers qu’il a changés (« 6 fichiers modifiés »), et le pied de la barre détaille les totaux du projet (`~2`, `+1`, `−0`, leur détail en infobulle).
- La branche du dossier du projet, avec son avance et son retard sur le dépôt distant, est dans la barre d’état : c’est le bouton qui ouvre [le sélecteur de branches](branches).

## Un projet à plusieurs dépôts

Un projet peut être un dossier qui n’est pas lui-même un dépôt git et qui en tient plusieurs, un par sous-dossier (`front`, `api`, `infra`, `docs`), ou un dépôt qui en tient d’autres que ses sous-modules. Escouade les trouve seul : à l’ajout du projet, au démarrage, quand la fenêtre revient au premier plan, et avec « Rechercher à nouveau ». Il regarde les sous-dossiers et un niveau plus bas (`services/billing`), sans jamais entrer dans `node_modules`, `target`, `dist`, `build`, les dossiers cachés ni le dossier des worktrees ; un sous-module reste à son dépôt. Un projet à un seul dépôt ne change en rien.

- **L’assistant « Nouveau projet »** dit « 4 dépôts git : api, docs, front, infra », et signale un dépôt qui n’a encore aucun commit (les agents n’en auront pas de worktree tant qu’il n’en a pas). Le dossier n’est jamais transformé en dépôt.
- **Réglages › Projets, groupe « Dépôts »** liste les dépôts et leur branche. Un dépôt apparu est annoncé, et les prochains agents en ont un worktree ; un dépôt disparu reste marqué « retiré » jusqu’à « Oublier ». Le fichier `.escouade.yml` du projet est à la racine du dossier : il n’est versionné nulle part tant que tu ne le mets pas toi-même dans un dépôt.
- **La barre d’état** dit « main · 4 dépôts » quand tous sont sur la même branche, sinon « 3 branches · 4 dépôts ». Son clic ouvre la liste des dépôts, avec la branche et l’avance de chacun ; choisir un dépôt ouvre son [sélecteur de branches](branches), le même qu’un projet à un dépôt. Le « Δ » de la barre de titre et du pied de la barre latérale est la somme des dépôts, le détail de chacun en infobulle.
- **Le panneau « Non commités »** groupe les fichiers par dépôt, chaque groupe avec sa branche et ses compteurs, repliable ; les dépôts sans changement tiennent en une ligne. « Commit… » direct ouvre une fenêtre avec un message par dépôt changé, chacun proposé dans le style de ce dépôt. « Historique » montre un dépôt à la fois, choisi dans le menu en tête.
- **Les agents** : sans worktree, un agent travaille à la racine du projet et voit tous les dépôts. Avec, il a un dossier à lui avec un worktree de chaque dépôt, tous sur la même branche, et les fichiers de la racine du projet y sont copiés ; la branche de départ du projet vaut pour chaque dépôt. Les commandes de lancement et de préparation gardent leur sous-dossier, qui est alors un dépôt.
- **Fusionner** vérifie d’abord chaque dépôt que la branche a changé : rien n’est mergé tant que l’un d’eux ne peut pas l’être (un conflit, des changements non commités). La fenêtre propose alors « Merger api seul », ou « L’agent résout le conflit ». Chaque dépôt va dans sa propre branche de base.
- **Les pull requests** : une par dépôt changé, ouvertes ensemble depuis une seule fenêtre, chaque description nommant les autres. La carte de l’agent et l’en-tête de sa conversation les montrent par dépôt (« api #43 · front #12 »).
- **Le Kanban** valide un ticket dans chaque dépôt que sa branche a changé : merge, pull request ou push, selon ton réglage. Pour un merge, tout est vérifié d’abord, et un conflit dans un dépôt laisse le ticket « À tester » sans rien merger, avec « L’agent résout ». Le résultat dit chaque dépôt (« api : Mergé dans main · front : Mergé dans develop »).
- **Le nettoyage** juge chaque dépôt à part : le worktree d’un dépôt où il reste du travail est gardé, les autres partent avec leur branche.
