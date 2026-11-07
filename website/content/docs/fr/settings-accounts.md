# Réglages : Comptes

L’onglet « Comptes » liste les comptes sur lesquels partent les agents, un groupe par harness, dans l’ordre où Escouade les essaie. Chaque changement y est enregistré tout de suite, sans passer par « Enregistrer ».

## Comment Escouade choisit un compte

Un compte est un harness lancé avec son propre dossier de configuration (pour Claude Code, son `CLAUDE_CONFIG_DIR`), donc avec sa connexion, ses sessions et son quota. Chaque compte appartient à un seul harness, et l’onglet les groupe par harness : « Comptes Claude Code », puis ceux des autres harness quand ils en ont. Aucun compte n’existe au départ : seuls ceux que tu ajoutes comptent. Un harness installé sans compte a son groupe, avec « Aucun compte » et « Ajouter un compte Codex… ». Le compte de la machine d’un harness, son « Principal », est ton installation habituelle : une fois ajouté, il est lancé comme toujours, sans dossier à part, et il se retire comme les autres, sans déconnecter la machine. Le fonctionnement des comptes et de leurs quotas est expliqué dans [les comptes](accounts) et dans [l’usage](usage).

La note sous le titre du premier groupe le résume : « Les nouveaux agents partent sur le premier compte actif de leur harness dont la 5 h est sous les seuils « Pause au-delà du quota » et « Changer de compte à » et dont la semaine tient son rythme, dans cet ordre. » En détail :

- Un nouvel agent part sur un compte de son harness, jamais sur celui d’un autre harness : le premier compte actif du groupe qui est « bon » : aucune fenêtre courte (la 5 h) n’a atteint le seuil de pause ni celui de « Changer de compte à » (le plus bas des deux compte), et aucune fenêtre longue (la semaine, les 30 jours, les crédits du mois) n’est à sa limite ni ne finirait au-delà de 100 % à son rythme actuel. Une semaine au-dessus du seuil mais qui tient son rythme reste bonne.
- Si tous les comptes actifs ont atteint le seuil, il part sur le premier compte actif, et le [pilote auto](autopilot) se met en pause.
- Le « Compte préféré » d’un projet, dans l’onglet [« Projets »](settings-projects), passe devant tant qu’il est actif.
- Un compte « Réservé aux projets » ne reçoit que les agents et les tickets de ces projets.

## La liste

Chaque compte a son bloc dans le groupe de son harness.

| Réglage | Effet | Par défaut |
|---|---|---|
| Nom (champ « Nom ») | Le nom du compte, enregistré quand tu quittes le champ ou appuies sur `Entrée`. Un nom vide ou déjà pris par un autre compte est refusé : le champ retrouve l’ancien. | « Principal » pour le compte Principal ; le nom donné à la création pour les autres |
| « Actif » | Interrupteur. Un compte inactif ne reçoit aucun nouvel agent. Il en faut au moins un d’actif par harness : l’interrupteur du dernier compte actif d’un groupe est grisé (« Il faut au moins un compte actif par harness. »). | Activé |
| Flèches ↑ et ↓ | Montent ou descendent le compte dans son groupe : l’ordre du groupe est l’ordre de priorité de son harness. | — |
| « Dossier » | Le dossier de configuration du compte, affiché sans pouvoir être modifié. | — |
| « Exécutable » | Le `claude` de ce compte. Vide, c’est celui de l’onglet « Harness » (le champ dit « Celui des réglages »). Enregistré quand tu quittes le champ. | Vide |
| « Se connecter… » ou « Se reconnecter… » | Ouvre la connexion du compte. | — |
| « Nouveaux agents » | « Modèle », « Effort » et « Mode » des nouveaux agents de ce compte, parmi ceux de son harness : les modèles que son harness liste pour lui, les efforts de son modèle (absent pour un harness sans effort), les modes de son harness. « Celui du harness » prend le défaut de son harness. Un nouvel agent les prend, sauf choix dans son champ de message ; un ticket, après ceux du ticket et du Kanban. Enregistré tout de suite. | « Celui du harness » (à la mise à jour, un compte Claude Code gardé prend les anciens réglages « Nouveaux agents » de l’onglet « Harness ») |
| « Supprimer » | Retire le compte de la liste, le compte de la machine compris (la confirmation dit qu’il reste connecté sur la machine). | — |
| « Réservé aux projets » | Une case par projet ouvert, plus une case cochée, marquée « (fermé) », pour chaque projet fermé auquel le compte reste réservé : le compte ne sert que les projets cochés. | Aucune case cochée : « Tous les projets » |

### Nom et état

Sous le nom, un point coloré et un texte disent où en est la connexion :

- « Vérification… » le temps de la lire ;
- « Connecté · » suivi de l’e-mail du compte claude.ai, ou « Connecté » seul quand l’e-mail n’est pas connu ;
- « Pas connecté » ;
- « Pas connecté par un compte claude.ai (clé d’API ?) » pour Principal, qui tourne alors sans doute avec une clé d’API ;
- « Inactif » pour un compte désactivé.

Quand la connexion existe mais n’est plus à jour, une ligne en dessous en donne la raison.

### Dossier et exécutable

Pour Principal, le dossier est celui de Claude Code lui-même (`~/.claude`, ou celui que désigne `CLAUDE_CONFIG_DIR`). Chaque autre compte a le sien, que crée Escouade dans `~/.escouade/claude/`, un sous-dossier par compte. Un compte peut faire tourner un autre `claude` que celui des réglages, par exemple un Claude Code de WSL pour l’un et un Claude Code de Windows pour l’autre.

### Se connecter

« Se connecter… » (« Se reconnecter… » quand le compte est déjà connecté, par exemple pour passer à un autre compte claude.ai) ouvre une fenêtre avec un terminal où tourne le `claude` du compte. Connecte-toi dedans, avec la commande `/login` si Claude Code ne la propose pas. Escouade guette la connexion toutes les 2 secondes : dès qu’elle est faite, la fenêtre affiche « Connecté au compte » suivi de l’e-mail, ferme le terminal et lit le quota du compte. Si tu fermes la fenêtre avant, le compte reste, pas connecté. Si Claude Code s’arrête avant la connexion, la fenêtre le dit et propose « Relancer ».

### Réservé aux projets

Les cases servent à garder un compte à ces projets seuls (ceux d’un client, par exemple) : les autres projets n’y partent jamais. Sans case cochée, le compte sert tous les projets, ce que la rangée dit par « Tous les projets ». Les comptes de tout harness se réservent de la même façon, et Principal peut être réservé comme les autres.

Un projet fermé en gardant son historique reste réservé au compte : il apparaît dans la rangée, coché et marqué « (fermé) », par exemple « Client (fermé) ». Décoche-le pour le retirer de la liste du compte ; il en sort aussi tout seul quand son historique est effacé. Tant qu’il est coché, un compte réservé à ce seul projet ne sert aucun projet ouvert, et « Tous les projets » ne s’affiche pas. La rangée apparaît même s’il n’y a aucun projet ouvert, du moment que le compte est réservé à un projet fermé.

Si aucun compte actif de son harness par défaut ne sert un projet (tous sont réservés à d’autres, ou désactivés), une ligne sous les groupes nomme le projet et dit que ses agents et ses tickets ne démarrent pas. Un projet qu’aucun compte actif d’un harness ne sert ne démarre rien sur ce harness : un agent ou un ticket qui le demande est refusé, avec le harness et le réglage à changer.

### Supprimer un compte

« Supprimer » demande confirmation. La fenêtre rappelle que les nouveaux agents ne partiront plus sur le compte, et que son dossier reste sur le disque, avec sa connexion et ses conversations. Tant que des agents non archivés tournent sur le compte, la suppression est refusée. Les projets qui le préféraient repassent sur « Automatique ».

## Ordre des harness

Dès que deux harness ont des comptes, la liste « Ordre des harness » apparaît sous les groupes, un harness par ligne avec ses flèches ↑ et ↓. Elle range les groupes de l’onglet et du panneau des quotas, et donne à chaque projet ses harness de secours : après son harness par défaut, les suivants dans cet ordre, seulement ceux qui ont un compte actif qui le sert. Changer l’ordre est enregistré tout de suite.

## Ajouter un compte

« Ajouter un compte… », sous les groupes (ou « Ajouter un compte Codex… » dans le groupe d’un harness sans compte), ouvre la fenêtre « Ajouter un compte Claude Code », avec « Harness » dès que deux harness sont installés. Elle propose d’abord :

- « Le compte de la machine » : le compte du dossier habituel du harness, quand le harness y est connecté (« Claude Code est connecté sur cette machine · ada@exemple.dev ») et qu’il n’est pas déjà ajouté (« Déjà ajouté ») ; sinon grisé. « Ajouter ce compte » l’ajoute tel quel.
- « Un nouveau compte » : un dossier à lui sous `~/.escouade`, et sa propre connexion, réglé ainsi :

1. « Nom » : le nom du nouveau compte (« Pro, Équipe… » en exemple). Deux comptes ne peuvent pas porter le même nom.
2. « Partager avec… » : des cases, toutes cochées au départ, pour les éléments du dossier du compte de la machine que le compte reprend. Les fichiers sont `settings.json` et `CLAUDE.md`, les dossiers `skills`, `agents`, `commands`, `plugins`, `hooks` et `output-styles` ; seuls ceux que ce dossier possède sont proposés.
3. « Partage » : « Lier (un changement vaut pour les deux comptes) », par défaut, ou « Copier ». Sous Windows, les dossiers sont liés par une jonction, qui ne demande pas de droits d’administrateur, et les fichiers sont copiés, un lien de fichier en demandant. Sous macOS et Linux, dossiers et fichiers sont liés par des liens symboliques ; la mention « copiés (un lien de fichier demande des droits d’administrateur) » que la fenêtre affiche à côté des fichiers ne vaut que pour Windows, même si elle apparaît aussi sous Linux.
4. « Créer et se connecter » crée le compte, actif et en dernier dans la liste, avec son dossier dans `~/.escouade/claude/`, puis ouvre sa connexion.

La connexion (`.credentials.json`) et `.claude.json` ne sont jamais partagés : chaque compte a les siens.

## Changer de compte à

Dans le groupe « Quotas », en bas de l’onglet, « Changer de compte à » fait passer un agent sur un autre compte avant que le sien ne soit à court.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Changer de compte à » | Un agent passe sur un autre compte de son harness à la fin de son tour quand une fenêtre courte (5 h) de son compte atteint ce seuil, ou qu’une fenêtre longue (semaine, 30 jours, crédits du mois) l’atteint sans tenir jusqu’à sa remise à zéro au rythme actuel. Les nouveaux agents et les tickets partent sur le premier compte de l’ordre dont la 5 h n’est pas pleine et dont la semaine tient son rythme, même sur « À la limite ». Choix : 80 %, 90 %, 95 % ou « À la limite ». | À la limite |

Sur « À la limite », un agent déjà lancé ne bouge pas avant la limite (les nouveaux agents suivent la règle du choix), puis « Reprendre sur un autre compte un agent de ticket arrêté par la limite » (onglet [« Harness »](settings-claude-code)) décide. Les deux réglages se complètent : le seuil agit à la fin d’un tour, la reprise à la limite sur un tour que la limite a arrêté en route. Le détail est dans [les comptes](accounts#changer-de-compte-avant-la-limite).

## Le seuil de pause

Le seuil se règle dans l’onglet [« Kanban »](settings-board), pas ici, mais il décide quels comptes Escouade écarte.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Pause au-delà du quota » (pour tous les projets) | Le pilote auto ne démarre aucun ticket tant que la fenêtre de 5 h ou la fenêtre hebdomadaire atteint ou dépasse ce seuil ; un ticket lancé à la main part quand même. Choix : 80 %, 90 %, 95 % ou 100 %. | 100 % |

Un compte dont une fenêtre courte a passé le seuil (ou dont une fenêtre longue est à sa limite) est écarté au profit du suivant de son harness, et le panneau des quotas le marque « au-delà du seuil de pause ou de changement de compte » (il compte aussi « Changer de compte à ») ; un compte que seul son rythme met derrière est marqué « rythme trop rapide pour sa fenêtre ». Le seuil de pause lui-même reste un pourcentage, fenêtre par fenêtre. Le pilote auto se met en pause harness par harness : un ticket attend que l’un des comptes de son harness repasse sous le seuil, pendant que ceux d’un autre harness démarrent. Un harness sans quota ne le met jamais en pause.

## Reprendre sur un autre compte

Quand un tour s’arrête sur la limite d’usage, la carte de l’agent propose « Reprendre sur » suivi d’un compte, ou « Reprendre sur un autre compte » avec un menu quand il y en a plusieurs : l’agent repart avec sa session sur l’autre compte. Sont proposés les comptes actifs de son harness, autres que celui de l’agent, qui servent son projet, sont connectés et restent sous le seuil de pause : la bascule automatique entre comptes reste dans le même harness.

Pour un agent de ticket, le réglage « Reprendre sur un autre compte un agent de ticket arrêté par la limite » de l’onglet [« Harness »](settings-claude-code) le fait tout seul. Une session ne passe pas d’un compte qui fait tourner Claude Code sous Windows à un compte qui le fait tourner dans WSL, ni l’inverse.
