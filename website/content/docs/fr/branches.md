# Les branches

Le bouton de branche de la barre d’état ouvre un sélecteur pour changer de branche, en créer, en supprimer et se synchroniser avec le dépôt distant. Cette page décrit aussi comment un agent reprend une branche existante et comment faire entrer la base dans la branche d’un agent.

## Le bouton de branche

Dans la barre d’état, en bas de la fenêtre, le bouton de branche montre la branche du dossier du projet affiché, par exemple `⎇ main ↑2 ↓5`. Il n’existe que si le dossier est un dépôt git. C’est toujours la branche du dossier du projet, pas celle du worktree d’un agent. Sur une HEAD détachée, il dit « HEAD détachée ».

| Ce qu’il montre | Ce que ça veut dire |
|---|---|
| `↑2` | 2 commits à pousser : la branche a 2 commits que sa branche distante n’a pas. |
| `↓5` | 5 commits à tirer : la branche distante en a 5 que la branche n’a pas. Le chiffre est en ambre quand il n’est pas nul. |
| « non publiée » | La branche n’a pas de branche distante : tu ne l’as jamais publiée. |
| « distante supprimée » | La branche distante qu’elle suivait n’existe plus. |
| « Récupération… », « Envoi… » ou « Fetch… » | Une synchronisation est en cours. Le bouton est grisé jusqu’à sa fin. |

Les indications de synchronisation n’apparaissent que si le dépôt a un dépôt distant. L’info-bulle les écrit en entier (« Suit origin/main : 5 à tirer, 2 à pousser ») et ajoute le moment du dernier fetch (« Dernier fetch : il y a 3 min »).

Escouade fait elle-même un fetch de chaque projet qui a un dépôt distant toutes les 5 minutes, sans jamais demander d’identifiants, pour que les chiffres restent à jour. Ce fetch automatique se coupe, et son intervalle et son délai d’attente se règlent, pour tous les projets ou pour un seul : voir [les réglages d’un projet](settings-projects#recuperer-automatiquement-git-fetch).

## Le sélecteur de branches

Un clic sur le bouton ouvre le sélecteur au-dessus de lui. Il liste les branches du dépôt en deux groupes, « Locales » puis « Distantes », la branche du dossier en premier, marquée d’un ✓. Chaque ligne montre le nom, une étiquette éventuelle et l’avance et le retard de la branche (`↑2 ↓5`).

- Le champ « Chercher une branche… » a le focus à l’ouverture. Il classe les branches comme `Ctrl+P` classe les fichiers : ce que tu tapes peut être le début d’un nom, un morceau ou seulement des lettres dans l’ordre. Un groupe affiche 300 branches au plus : au-delà, « N autres branches : tape pour les chercher. »
- `↑` et `↓` se déplacent, `Entrée` choisit la branche, `Échap` ferme. La touche Menu ou `Maj+F10` ouvre le menu de la ligne.
- Une ligne en dessous de la liste décrit la branche sous le curseur : ce qu’elle suit (« Suit origin/feat : 2 à pousser, 5 à tirer ») ou pourquoi elle ne peut pas être choisie.

Les étiquettes disent d’où vient une branche :

| Étiquette | Sens |
|---|---|
| « utilisée par l’agent refacto-auth » | Le worktree de cet agent a cette branche : on ne peut ni la choisir ni la supprimer. |
| « dans un autre worktree » | Un worktree qui n’est celui d’aucun agent l’a. |
| « suivie par feat/login » | Une branche distante qu’une branche locale suit : choisir la distante extrait la locale. |
| « distante supprimée » | Le dépôt distant n’a plus la branche que celle-ci suivait. |

Au pied du sélecteur : « Nouvelle branche… » et « Branches mergées… », puis la synchronisation.

### Récupérer, pousser, fetch

Ces boutons n’apparaissent que si le dépôt a un dépôt distant. Ils portent sur la branche du dossier du projet. Escouade les lance sous ton nom, donc git peut te demander des identifiants. Un message dit ensuite ce qui a été fait (« 3 commits tirés », « 2 commits poussés », « Déjà à jour »), ou pourquoi ça n’a pas pu l’être.

| Bouton | Ce qu’il fait |
|---|---|
| « Récupérer » | Fait un fetch puis intègre les nouveaux commits de la branche distante, en avance rapide seulement. Il est grisé quand la branche ne suit rien ou n’a aucun retard. Si les deux branches ont divergé, il refuse (« … ont divergé : pull impossible en avance rapide ») : fais le rebase ou le merge toi-même, ou demande-le à un agent. Il refuse aussi si des modifications non commitées seraient écrasées. |
| « Pousser » | Pousse la branche. Il est grisé quand elle n’a aucune avance. Si le dépôt distant a des commits que tu n’as pas, il refuse et te dit de les récupérer d’abord. |
| « Publier » | Remplace « Pousser » quand la branche n’a pas de branche distante, ou que la sienne a été supprimée. Il la publie sur le dépôt distant que la branche suivait, sinon sur `origin`, sinon sur l’unique dépôt distant, et la fait suivre la branche publiée. Avec plusieurs dépôts distants dont aucun n’est `origin`, il refuse : publie à la main. |
| « Fetch » (« maintenant ») | Récupère le dépôt distant sans rien intégrer, et dit ce qu’il y a à tirer (« Fetch terminé : 3 commits à tirer »). |

Sous ces boutons, une ligne dit quand le dépôt a été récupéré pour la dernière fois (« Récupéré il y a 3 min », l’heure exacte au survol). Si le dernier fetch automatique a échoué, elle le dit aussi, discrètement, avec sa raison et ce qu’il faut faire :

| Ce qu’elle dit | Ce qu’il faut faire |
|---|---|
| « il demande des identifiants » | Fais « Fetch » une fois : git te les demande. Le fetch automatique ne réessaie pas en boucle : il attend ce fetch réussi, puis reprend. |
| « pas fini en 5 min » | Le dépôt est lourd ou le réseau lent : allonge le délai d’attente (« Régler » ouvre le bon réglage). |
| « le réseau ou le dépôt distant est injoignable » | Vérifie ta connexion, ton VPN ou ton proxy. Il réessaie à l’intervalle suivant. |
| « git a refusé » | La raison de git est au survol. |

Sous Windows, Git for Windows 2.47.0(1) a un ssh qui cale sur les gros fetch. Avec lui, et seulement si tu n’as pas choisi ton propre ssh (`core.sshCommand`, `GIT_SSH` ou `GIT_SSH_COMMAND`), le fetch automatique passe par le ssh de Windows. La ligne te dit alors de mettre Git à jour (2.47.0(2) ou plus). Pour le reste, il passe par ta configuration de git telle qu’elle est (proxy, certificats, `insteadOf`) : Escouade n’y ajoute rien.

## Changer de branche

Un clic sur une ligne, ou `Entrée`, change la branche du dossier du projet. Pour une branche distante, Escouade passe sur la branche locale qui la suit, et la crée, en la faisant suivre, s’il n’y en a pas. Si tu es déjà dessus, le sélecteur se ferme simplement.

Escouade vérifie avant de bouger le dossier, et refuse avec un message dans ces cas :

- un agent qui n’a pas de worktree travaille dans le dossier du projet (« L’agent refacto-auth travaille dans le dossier du projet : attends la fin de son tour. ») ;
- un merge, un cherry-pick, un revert ou un rebase est en cours dans le dossier ;
- le dossier est sur une HEAD détachée dont des commits ne sont dans aucune branche : le changement les laisserait seulement dans le reflog ;
- la branche est celle d’un worktree : la ligne est grisée et dit pourquoi ;
- le changement écraserait des fichiers non suivis du dossier : le message les nomme, déplace-les ou supprime-les.

Quand le dossier a des modifications non commitées sur des fichiers suivis, Escouade ne les perd pas, et te demande : « Changer pour « feat/login » ? », « Il reste des changements non commités dans le dossier du projet. » Le bouton « Mettre de côté (stash) et changer » les met dans un stash git nommé « escouade: avant de passer sur feat/login » et affiche « Changements mis de côté : « escouade: avant de passer sur feat/login » (git stash). » Escouade ne les remet pas en place : reprends-les plus tard avec `git stash pop`. Si le changement échoue, elles sont remises où elles étaient.

## Créer une branche

« Nouvelle branche… », au pied du sélecteur, ou « Créer une branche ici… » dans le menu d’un commit de [l’historique](git), ouvre la fenêtre « Nouvelle branche ».

- « Nom » : le nom de la branche (`feat/ma-branche`). Escouade le vérifie pendant que tu tapes, selon les règles de git, et dit en rouge sous le champ pourquoi un nom est refusé, par exemple parce qu’il est déjà pris.
- « À partir de » : le point de départ. Par défaut la branche courante, au choix une autre branche ou, quand la fenêtre vient de l’historique, le commit choisi (« Le commit 4f2a9c1 »). Sur une HEAD détachée, c’est « Le commit courant ».
- « et y passer » : coché par défaut, il bascule le dossier sur la nouvelle branche.
- « Créer » (ou `Entrée` dans le champ) crée la branche et dit « Branche « feat/ma-branche » créée : tu es dessus. » ou, sans le passage, « Branche « feat/ma-branche » créée. »

Le passage obéit aux mêmes règles et aux mêmes refus qu’un changement de branche, y compris la question du stash. Quand la branche part du commit où se trouve le dossier, aucun fichier ne change : les modifications non commitées t’accompagnent et la question n’est pas posée.

## Supprimer des branches

Le menu d’une ligne du sélecteur (clic droit, touche Menu ou `Maj+F10`) propose « Supprimer la branche… » pour une branche locale et « Supprimer la branche distante… » pour une distante. Il ne propose rien pour la branche locale du dossier ni pour celle d’un worktree : supprime d’abord l’agent, avec son worktree, voir [un worktree par agent](worktrees).

La **base du projet** sert de repère : c’est la branche cible du Kanban tant qu’elle existe, sinon la branche du dossier du projet.

### Une branche locale

La confirmation « Supprimer la branche « feat/login » ? » dit que la branche sera supprimée en local. Une case « Supprimer aussi origin/feat/login », décochée par défaut, apparaît quand la branche suit une branche distante de même nom. Si tu la coches alors que cette branche distante est la branche par défaut du dépôt distant, la branche distante de la base du projet, ou que d’autres branches locales la suivent aussi, seule la branche locale est supprimée et un message dit pourquoi.

Une branche dont le travail est déjà dans la base (mergée, ou mergée par squash) est supprimée tout de suite. Sinon, Escouade demande « Supprimer « feat/login » quand même ? » et dit ce qui serait perdu : « 3 commits ne sont dans aucune autre branche. », ou « Elle n’est pas mergée dans la base du projet, mais ses commits sont dans une autre branche. » « Supprimer quand même » force la suppression.

### Une branche distante

La confirmation « Supprimer la branche distante « origin/feat/login » ? » prévient qu’elle sera supprimée du dépôt distant, pour tout le monde. Escouade refuse la branche par défaut du dépôt distant et la branche distante de la base. Elle refuse aussi une branche que tu n’as pas récupérée (fais un fetch d’abord), ou qui a bougé sur le dépôt distant depuis ton dernier fetch.

## Nettoyer les branches mergées

« Branches mergées… », au pied du sélecteur, ouvre la fenêtre « Branches mergées » : « Ces branches locales sont déjà dans la base du projet. » En général, ce sont celles des tickets validés.

- Toutes les branches sont cochées au départ ; décoche celles que tu veux garder. S’il n’y en a aucune, la fenêtre dit « Aucune branche mergée à nettoyer. »
- La base elle-même, la branche du dossier du projet et celles qu’un worktree occupe n’y sont jamais.
- Une branche mergée par squash y figure aussi : Escouade compare ce que son merge changerait à la base.
- « Supprimer N branches » les supprime une à une, sans jamais forcer. Celle qui ne peut pas l’être reste dans la liste avec la raison (« feat/login » n’a pas pu être supprimée : …).

## Reprendre une branche existante

Un agent ou un ticket peut travailler sur une branche qui existe déjà plutôt que sur une nouvelle :

- « Nouvel agent sur une branche… », dans le menu de la flèche à droite de « Nouvel agent », ouvre le sélecteur en mode « Choisir une branche » : il n’a ni les actions du pied ni les menus de ligne, et les branches impossibles à choisir sont grisées avec la raison. Un nom tapé qu’aucune branche ne porte y propose aussi « Créer la branche « … » et y lancer un agent », depuis le point de départ d’un nouvel agent (voir [les worktrees](worktrees)) ;
- dans le sélecteur de la barre d’état, « Lancer un agent sur cette branche » est dans le menu d’une ligne, sauf pour la branche du dossier du projet et celles qu’un worktree occupe ;
- le champ « Branche » du formulaire d’un ticket propose « Reprendre une branche existante… ».

Dans ce sélecteur, la branche du dossier du projet et la branche cible du Kanban sont grisées, avec la raison : un agent n’y travaille pas directement. Le reste se passe dans un worktree : voir [un worktree par agent](worktrees). Escouade ne supprime jamais une branche reprise, même quand le ticket est validé.

## Intégrer la base dans la branche d’un agent

Pendant qu’un agent travaille sur sa branche, la base peut avancer. « Intégrer main » (le nom de la branche de base de l’agent est écrit dans l’entrée) fait entrer ses nouveaux commits dans la branche de l’agent, dans son worktree. C’est dans le menu d’un clic droit sur la carte de l’agent et sous « Merger … » dans le panneau des fichiers de [Git](git). L’entrée n’existe que pour un agent qui a un worktree et qui n’est pas archivé.

Escouade fait un merge de la base dans la branche, ou un rebase de la branche sur la base quand la stratégie du Kanban est « Rebase » (voir [les réglages du Kanban](settings-board)). Pour les stratégies « Merge commit » et « Squash », c’est un merge : il n’y a rien à réduire en un commit dans une branche.

Elle refuse, avec un message, dans ces cas :

- l’agent travaille (« Attends la fin du tour de refacto-auth pour intégrer main. ») ;
- son worktree a des changements non commités (« Commite ou mets de côté les changements de l’agent d’abord. ») ;
- un merge, un rebase ou une autre opération est déjà en cours dans son worktree ;
- l’agent est archivé (« refacto-auth est archivé : restaure-le pour intégrer main. ») ;
- la branche de base n’existe plus.

Le résultat s’affiche dans un message :

| Message | Ce qui s’est passé |
|---|---|
| « main intégrée dans escouade/refacto-auth. » | L’intégration a réussi. |
| « escouade/refacto-auth a déjà tout ce qu’il y a dans main. » | Il n’y avait rien à intégrer : rien n’a été touché. |
| « main a un conflit avec escouade/refacto-auth sur 1 fichier : refacto-auth doit le résoudre. » | Le merge s’est arrêté sur des conflits. Il est laissé tel quel dans le worktree, et l’agent reçoit un message qui nomme les fichiers : il doit les résoudre, puis commiter le merge. |
| « Le rebase sur main a des conflits et a été annulé : refacto-auth doit le refaire. » | Le rebase est défait, rien n’est à moitié fait, et l’agent reçoit la consigne de rebaser sa branche et de résoudre les conflits lui-même. |

> [!TIP]
> Intègre la base avant de merger : les conflits se résolvent dans le worktree de l’agent, qui connaît son code, et le merge dans la base se fait ensuite sans accroc. Pour merger, voir [un worktree par agent](worktrees).
