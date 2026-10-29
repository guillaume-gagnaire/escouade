---
title: "Branches"
subject: "Git & fichiers"
sources: ["docs/SPEC.md"]
tickets: ["ESC-64"]
decisions: []
compared: null
order: 23
---

## Ce que c'est

Le sélecteur de branche, ouvert par le bouton de branche de la barre d'état (voir *Barre de statut*), dans tout dépôt git, avec ou sans dépôt distant. On y change de branche, on en crée, on en supprime, on nettoie les branches mergées et on synchronise avec le dépôt distant. Le graphe de l'historique offre les mêmes gestes sur un commit. Quand l'agent affiché travaille dans un worktree à lui, le bouton et le sélecteur agissent sur la branche de ce worktree.

## Comportement

### Base du projet et branches mergées

- La base du projet est la branche cible du Kanban si elle existe en local, sinon la branche du dossier, sinon HEAD, avec la branche que cette base suit : une branche mergée par une pull request, récupérée par fetch mais pas encore tirée, compte comme mergée.
- Une branche est « mergée » quand son bout est un ancêtre de la base, ou que la merger dans la base ne changerait rien (squash-merge, cherry-pick). La base n'est jamais « mergée en elle-même ».

### Le sélecteur

- Il s'ouvre au-dessus de la barre. Un champ de recherche a le focus, avec un filtre approché comme celui d'« Ouvrir un fichier » (voir *Ouvrir un fichier*). Puis « Locales » (la courante cochée, en tête, puis par nom) et « Distantes » (`origin/HEAD` exclu).
- Chaque ligne dit ses commits à pousser et à tirer sur sa branche distante (« ↑2 ↓5 »), « distante supprimée » quand sa branche suivie a disparu, et « suivie par <locale> » pour une distante qu'une locale suit déjà.
- Une branche prise par le worktree d'un agent (ou un autre worktree) montre « utilisée par l'agent X », n'est pas activable, et dit pourquoi au survol et sous le curseur.
- ↑ ↓, Entrée, Échap, clic. Le clic droit (ou la touche Menu, ou Maj+F10) offre « Lancer un agent sur cette branche » pour une branche libre (voir *Git & fichiers*), puis « Supprimer la branche… » (jamais pour la courante ni une branche de worktree). Sur la branche du worktree d'un agent ordinaire, il offre « Ouvrir une PR… » (voir *Pull requests*).
- Au pied : « Nouvelle branche… », « Branches mergées… », puis la synchro : « Récupérer » (pull), « Pousser » (« Publier » sans branche suivie, ou quand elle a été supprimée du dépôt distant) et « Fetch » (voir *Git & fichiers*). L'action en cours s'affiche sur le bouton de la barre (« Envoi… »), puis un toast la résume (« 3 commits tirés ») et le focus revient au bouton de branche.
- Les menus contextuels ouverts au clavier (sélecteur, graphe, menu des terminaux) prennent le focus.

### Changer de branche (le dossier du projet)

- Refusé s'il reste des changements non commités sur des fichiers suivis ; « Mettre de côté (stash) et changer » les met dans un stash nommé « escouade: avant de passer sur <branche> », ce qu'un toast dit. Les fichiers non suivis ne comptent pas comme changements et ne vont pas dans le stash.
- Refusé aussi quand un agent sans worktree travaille dans le dossier (de ce projet, ou d'un autre projet ouvert sur le même dépôt), pendant un merge, un cherry-pick, un revert ou un rebase en cours, depuis une HEAD détachée dont des commits n'appartiennent à aucune branche (le texte dit combien), et quand des fichiers non suivis seraient écrasés (ils sont nommés).
- Si le changement échoue après le stash, les changements reviennent de leur stash, ou l'erreur dit où ils sont.
- Une branche distante se prend par la branche locale qui la suit, quel que soit son nom, créée au besoin. C'est refusé si une locale du même nom existe sans la suivre.

### Nouvelle branche

- Le nom est vérifié à la frappe comme git le fait ; sont refusés aussi un nom qui commence par `-` ou `@`, et un nom de branche distante (`origin/…`).
- « À partir de » : la branche courante, une autre, ou le commit choisi dans le graphe. Une révision comme `main~1` n'est pas une branche.
- « et y passer », coché par défaut : les changements non commités suivent si on part du commit courant ; sinon le choix du stash est proposé.
- La branche ne suit aucune branche distante, même créée depuis `origin/main`.
- Nom, existence et point de départ sont vérifiés avant tout stash. Si le changement échoue, la branche créée est supprimée et le stash remis.

### Supprimer une branche

- Une confirmation ; si son travail n'est pas dans la base du projet, une seconde dit combien de commits ne sont dans aucune autre branche (« Supprimer quand même » ; s'il n'y en a aucun : « Elle n'est pas mergée dans la base du projet, mais ses commits sont dans une autre branche. »).
- Jamais la branche du dossier du projet (même forcée) ni celle d'un worktree. `origin/HEAD` n'est pas une branche : refusé.
- « Supprimer aussi origin/… » quand la branche distante a le même nom. La distante ne part que si elle est bien celle de la branche (aucune autre locale ne la suit ; ce n'est ni la branche par défaut du dépôt ni la distante de la base), et avec `--force-with-lease` : un commit poussé par quelqu'un d'autre depuis le dernier fetch n'est jamais supprimé sans avoir été vu. Sinon seule la locale part, et l'app le dit.
- « Branches mergées… » liste les branches locales déjà dans la base du projet, toutes cochées (hors la base, la courante et celles d'un worktree), et les supprime ensemble, sans jamais forcer. Après une suppression en partie réussie, les cases décochées le restent.
- Le sélecteur sert aussi à supprimer, avec ses confirmations, les branches qu'Escouade ne supprime jamais seule : celle qu'un ticket a reprise, celle d'un agent supprimé avec « Supprimer aussi le worktree ».

### Depuis le graphe

Clic droit sur un commit de l'« Historique » (ou touche Menu, ou Maj+F10 sur la ligne qui a le focus) :

- « Créer une branche ici… » ouvre la fenêtre de nouvelle branche, ce commit pour point de départ ;
- pour chaque branche locale du commit, « Passer sur <branche> » et « Supprimer la branche… » (le nom en indication quand le commit en porte plusieurs), grisés avec leur raison pour une branche qu'un worktree tient, absents pour la branche du dossier ;
- « Comparer avec la branche courante » (« Comparer avec HEAD » sur une HEAD détachée) ouvre la fenêtre de diff sur `git diff courante cible`, titrée « courante ↔ cible ». La cible est la branche du commit (une locale d'abord, sinon une distante), à défaut le commit (hash court). Rien à comparer sur le commit de HEAD.

Le menu prend le focus quand il s'ouvre au clavier : ↑ ↓, Début, Fin, Entrée ; Échap ou Tab le ferment et rendent le focus à la ligne.

### La branche de l'agent ouvert

- Le bouton de branche suit l'agent dont la conversation est affichée ; le tableau, l'éditeur, un lancement ou un terminal par-dessus rendent celle du dossier du projet. Pour un agent qui a un worktree (celui d'Escouade, ou un worktree où il est entré seul : voir *Pull requests*), la barre montre sa branche, sa branche suivie et ses commits à pousser et à tirer, avec « Branche de l'agent X, dans son worktree » en tête de l'infobulle. Pour un agent du dossier du projet, la branche du dossier.
- Après le bouton, la PR de cette branche : son icône, son numéro et sa couleur comme sur la carte de l'agent, la même infobulle, « +n autres » (voir *Pull requests*). Un clic l'ouvre dans le navigateur.
- Un agent de plusieurs dépôts montre « feat · 3 dépôts », et un clic liste la branche de chacun (voir *Projet à plusieurs dépôts*).
- Le sélecteur ouvert depuis là (« Branche de <agent> ») bascule **le worktree de l'agent**. Jamais vers la branche du dossier du projet, la cible du Kanban ni celle d'un autre worktree. Refusé pendant un tour, avec des changements suivis non commités (rien n'est mis de côté), au milieu d'un merge ou d'un rebase, sur une HEAD détachée qui porte des commits qu'aucune branche n'a, pour un agent de ticket (son Kanban tient sa branche), pour un agent de plusieurs dépôts, et dans un worktree qui n'est pas d'Escouade ; la raison est sur chaque ligne.
- Une branche distante passe par la locale qui la suit, faite au besoin. La branche prise devient celle de l'utilisateur : jamais renommée, gardée à la suppression de l'agent. Celle quittée, faite pour l'agent, est supprimée si elle n'a aucun commit de plus que sa base.
- La conversation le dit (« Branche : X → Y. »). Le prochain message envoyé à l'agent commence, une seule fois, par une note dans la langue des textes rédigés (« Note d'Escouade : l'utilisateur a fait passer ton dossier de travail de la branche « X » à la branche « Y »… », avec sa PR quand elle est connue), absente de la bulle affichée ; aucun tour n'est lancé pour ça. En vue Claude Code, rien n'est ajouté à ce qui est collé dans son terminal : la ligne de la conversation suffit, et aucune note n'attend. Les PR de la nouvelle branche sont lues tout de suite.
- Un agent qui change lui-même de branche dans son worktree (`git switch`, `gh pr checkout`) est suivi : son worktree passe sur cette branche (devenue celle de l'utilisateur ; rien n'est supprimé), ses PR sont relues, la conversation le dit, sans note (une note en attente part : il sait où il est). Tant qu'il n'est pas suivi, Escouade ne renomme pas sa branche. Jamais sur une HEAD détachée, pour un agent de ticket ou de plusieurs dépôts, ni pendant qu'Escouade renomme sa branche.
- « Récupérer », « Pousser » et « Fetch » s'appliquent au worktree de l'agent. « Dossier du projet… » ouvre à sa place le sélecteur du dossier, avec « Nouvelle branche… » et « Branches mergées… ».

### Ailleurs

Un agent sur une branche qui existe déjà, la branche de départ et « Intégrer <base> » : voir *Git & fichiers*. Le champ « Branche » d'un ticket : voir *Agent d'un ticket*.

## Limites

- Reconnaître une branche squash-mergée demande git 2.38 ; avec un git plus ancien, elle compte comme non mergée, jamais l'inverse.
- Les branches distantes d'un commit ne se changent ni ne se suppriment depuis le graphe.
- Les commandes git de branche, de pull et de push tournent en anglais (`LC_ALL=C`), pour reconnaître leurs refus quelle que soit la langue du système.
