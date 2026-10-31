# Les agents

Un agent est une session Claude Code qui travaille dans un projet. Cette page explique comment créer des agents, choisir leur modèle, leur effort et leur mode de permission, les suivre dans la barre latérale, puis les renommer, les dupliquer, les archiver ou les supprimer.

![La barre latérale d’un projet avec cinq agents, leurs statuts, et la conversation de l’agent « refacto-auth »](/images/agents.jpg)

## Qu’est-ce qu’un agent

Chaque agent est une session Claude Code : une conversation, un modèle, un effort, un mode de permission, un compte Claude et, si tu le veux, un worktree git à lui. Un projet peut avoir plusieurs agents, qui travaillent chacun de leur côté, et la barre latérale de gauche les liste. La conversation d’un agent est décrite dans [la page de la conversation](chat).

## Créer un agent

| Pour… | Fais ceci |
|---|---|
| Créer un agent dans le projet affiché | `Ctrl+N`, ou le bouton « Nouvel agent » en haut de la liste |
| Créer le premier agent d’un projet qui n’en a pas | « Créer un agent » dans la liste vide, ou « Nouvel agent » au milieu de la fenêtre (« Aucun agent dans ce projet. ») |
| Travailler sur une branche qui existe déjà, ou sur une nouvelle branche que tu nommes | La flèche à droite de « Nouvel agent » (« Autres façons de créer un agent »), puis « Nouvel agent sur une branche… » : choisis la branche dans le sélecteur, ou tapes-y le nom d’une nouvelle |
| Créer un agent sans worktree, ou dans un worktree, une fois | La flèche à droite de « Nouvel agent », puis « Nouvel agent sans worktree » (projet à « Un worktree par agent ») ou « Nouvel agent dans un worktree » (les autres) |
| Partir d’une branche depuis le sélecteur de branches | Clic droit sur une branche, puis « Lancer un agent sur cette branche » |
| Créer un agent en même temps qu’un projet | Dans la fenêtre « Nouveau projet », l’interrupteur « Créer un premier agent » (activé par défaut) propose aussi le modèle de cet agent |

Sous macOS, `Ctrl` devient `⌘` pour les raccourcis (sauf `Ctrl+Tab`).

Le nouvel agent est sélectionné tout de suite, et le curseur est dans son champ de saisie : décris la tâche, c’est tout. Il prend le modèle, l’effort et le mode de permission par défaut des réglages, que tu changes ensuite pour cet agent seul (voir plus bas). Tant qu’il n’a pas de message, la conversation affiche « Agent prêt » et le dossier dans lequel il travaille.

Les agents de ticket sont créés par le Kanban (voir [le Kanban](board)), et un agent peut en lancer d’autres par le [serveur MCP](mcp).

## Le nom d’un agent

Un nouvel agent s’appelle `agent-1`, `agent-2`… Dès que tu lui envoies son premier message, Escouade demande à Haiku un nom court pour la tâche, en minuscules et sans accents, par exemple `refacto-auth`. Ce nom remplace `agent-N` dans la liste après quelques secondes ; s’il est déjà pris dans le projet, Escouade ajoute `-2`, `-3`…

- Un message qui est une commande de Claude Code sans rien à en tirer (`/clear`, `/model sonnet`, un skill sans argument) ne nomme pas l’agent : c’est le message suivant qui le fait. Pour un skill ou une commande personnalisée avec des arguments, le texte de la commande, sans la barre oblique, sert de base au nom.
- Si l’agent travaille dans un worktree que ce même agent a créé, sa branche prend le nom trouvé (`escouade/refacto-auth` avec le préfixe par défaut), tant que ce nom de branche est libre. Une branche qui existait avant l’agent, une branche que tu as nommée, une branche avec une PR ou une branche déjà poussée garde son nom.
- Si la réponse de Haiku n’est pas un nom utilisable, l’agent garde `agent-N`.
- Un agent que tu as renommé, ou qui est la copie d’un autre, n’est jamais renommé automatiquement.

## Modèle, effort et mode de permission

Sous le champ de saisie, une barre de menus règle l’agent : « Modèle », « Effort » et « Mode » (plus « Compte » quand tu as plusieurs comptes). Les valeurs des nouveaux agents sont celles de leur compte, dans sa fiche de Réglages › Comptes, « Nouveaux agents » : « Modèle », « Effort » et « Mode », parmi ceux de son harness (par défaut ceux du harness : Sonnet, Moyen et Auto pour Claude Code). Voir [les réglages des comptes](settings-accounts).

### Modèle

Le menu liste les modèles que le harness de l’agent annonce pour son compte. Pour Claude Code : Fable, Opus, Sonnet et Haiku, chacun affiché avec la version que Claude Code lance pour lui, par exemple « Opus 5.5 », et les autres modèles que le compte annonce, sous le nom qu’il leur donne (« Mythos ») ; tant que Claude Code n’a pas annoncé ses modèles (au premier démarrage d’un agent), seul le nom de la famille apparaît. La carte de l’agent et l’en-tête de la conversation rappellent le modèle utilisé.

Quand la liste est longue (plus de 8 modèles, ou ceux des fournisseurs d’OpenCode), le menu a un champ « Rechercher » et range les modèles par groupe, chacun sous son intitulé : « Favoris » d’abord, puis un groupe par famille, et « Autres » pour un modèle sans famille reconnue. Les familles sont Fable, Opus, Sonnet, Haiku, Mythos (et toute autre famille que le compte annonce) pour Claude Code, du modèle le plus récent au plus ancien ; le fournisseur pour OpenCode ; l’éditeur (GPT, Claude, Gemini…) pour Codex et Copilot CLI. La recherche filtre dans les groupes et cache ceux qui restent vides ; `↑` et `↓` passent d’un modèle à l’autre, et `Entrée` choisit. L’étoile d’un modèle en fait un favori (pleine, dans la couleur d’accent) ou le retire (un contour discret) ; les favoris valent pour tous les agents du même harness.

### Effort

L’effort dit à quel point Claude réfléchit avant de répondre.

| Niveau | Ce qu’il veut dire |
|---|---|
| Bas | Réponses rapides, peu de réflexion |
| Moyen | Équilibré |
| Élevé | Réflexion approfondie |
| Très élevé | Réflexion très approfondie |
| Max | Réflexion maximale, plus de tokens |

Haiku ne gère pas l’effort : le menu est grisé (« Haiku ne gère pas l’effort »).

### Modes de permission

Le mode décide de ce que Claude fait sans te demander.

| Mode | Ce qu’il fait |
|---|---|
| Auto | Un classifieur approuve les actions sûres et demande pour le reste. Indisponible avec un modèle que le Claude Code du compte de l’agent dit sans ce mode, comme Haiku 4.5 (Claude Code 2.1.292 et avant) ; Haiku 5.5 le prend. |
| Demander | Claude demande ton accord avant chaque action sensible. |
| Plan | Claude analyse et propose un plan sans rien modifier. Tu l’approuves, ou tu lui demandes de continuer à planifier (voir [les questions et autorisations](chat#questions-et-autorisations)). |
| Édits auto | Les modifications de fichiers sont acceptées sans demander. |
| Bypass | Aucune demande de permission. À réserver aux environnements sûrs : en le choisissant, un message te rappelle que Claude agira sans aucune demande, et le menu passe en rouge tant qu’il est actif. |

### Changer en cours de conversation

Les menus de modèle, d’effort et de mode restent utilisables pendant toute la vie de l’agent, même pendant qu’il travaille. Le choix s’applique à l’agent tout de suite, la conversation continue, et il vaut pour cet agent seulement. Si le processus de l’agent est arrêté (voir [plus bas](managing-agents#fermer-la-fenetre-quitter-et-redemarrer)), le choix est pris en compte à son prochain démarrage.

## Le compte d’un agent

Quand tu as ajouté plusieurs comptes Claude, un menu « Compte » s’ajoute aux autres. Son premier choix, « Automatique », laisse Escouade prendre le compte du projet (son « Compte préféré »), sinon le premier compte actif « bon » (5 h sous les seuils de pause et de changement de compte, semaine au rythme) ; le nom du compte choisi est écrit entre parenthèses. Le compte d’un agent ne change plus après son premier message, puisque sa conversation est rangée dans ce compte : le menu est alors en lecture seule.

La page sur [les comptes](accounts) explique comment en ajouter et ce que fait « Reprendre sur… » quand un agent atteint la limite d’usage. Les options sont dans [les réglages des comptes](settings-accounts).

## Travailler dans un worktree ou sur une branche existante

Par défaut, un agent travaille dans le dossier du projet, comme toi. Avec l’option « Un worktree par agent », chaque nouvel agent reçoit son propre worktree : un dossier à lui, sur une nouvelle branche `escouade/<nom>` créée à partir de la « Branche de départ des nouveaux worktrees » du projet : la branche du dossier par défaut, la branche cible du Kanban ou une branche de ton choix (voir [la branche de départ](worktrees#la-branche-de-depart)). « Nouvel agent depuis une autre branche… », dans le menu à côté de « Nouvel agent », en choisit une autre pour un agent seul. Ses modifications restent à part, et tu les merges quand tu veux. L’option se choisit à la création du projet, dans les réglages du projet, ou par un clic droit sur la tuile du projet puis « Activer le worktree par agent » ; elle ne concerne que les agents créés ensuite.

- La carte de l’agent affiche sa branche après un `⎇`.
- Si le projet a des commandes de préparation (dépendances, code généré…), elles tournent dans le nouveau worktree avant le premier message de l’agent : la carte dit « Préparation… », et ton premier message part une fois la préparation finie.
- Si le worktree ne peut pas être créé (un dépôt sans commit, par exemple), l’agent travaille dans le dossier du projet et la conversation te le dit.
- Avec « Nouvel agent sur une branche… », l’agent travaille dans un worktree sur une branche **qui existe déjà**, ou sur une branche que le sélecteur crée sous le nom que tu tapes. Ce ne peut pas être la branche du dossier du projet, ni la branche cible du Kanban, ni une branche qu’un autre worktree occupe. Supprimer cet agent ne supprime jamais une branche qui existait déjà ; celle créée pour lui part avec son worktree. Dans les deux cas, le dossier du worktree porte le nom de la branche (`feat/login` donne `login`).

Tout le détail est dans [les worktrees](worktrees) et [les branches](branches).

## Suivre un agent dans la barre latérale

Chaque agent a une carte dans la liste :

- son nom, une pastille de couleur et son statut ;
- le modèle, la durée pendant laquelle il a travaillé et, s’il a un worktree, sa branche ;
- ses tokens, son coût (précédé de « ≈ » tant qu’un tour tourne : c’est alors une estimation) et le nombre de fichiers modifiés dans son dossier ;
- la ligne « Plan 2/7 » quand il suit une liste de tâches (voir [l’avancée d’un plan](plan)) ;
- l’étiquette `▸ CLÉ` de son ticket, quand il en a un ;
- une icône d’onde quand le remote control est activé, dont l’infobulle dit aussi d’où il vient : de toi, du réglage du projet ou du réglage de tous les agents ; en [vue Claude Code](claude-code-view), elle dit l’état de l’interface (voir [le remote control](remote-control)) ;
- la ligne que l’agent publie lui-même quand il utilise le serveur MCP d’Escouade.

Un clic sur la carte l’affiche ; `Ctrl+Tab` et `Ctrl+Maj+Tab` passent à l’agent suivant ou précédent du projet, sans les agents archivés (voir [suivre ses agents](overview)). Quand un agent pose une question, termine ou tombe en erreur alors que tu ne le regardais pas, sa carte clignote jusqu’à ce que tu l’affiches (voir [les notifications](notifications)).

### Les statuts

| Statut | Pastille | Ce que ça veut dire |
|---|---|---|
| En cours | verte | Claude travaille. |
| Question | ambre | Claude attend une réponse à une question ou à une autorisation. |
| Prêt | grise | L’agent vient d’être créé, il n’a encore rien fait. |
| Terminé | verte | Son dernier tour est fini, ou tu l’as interrompu. |
| Erreur | rouge | Son dernier tour s’est terminé en erreur, ou son processus s’est arrêté. Envoie un message pour relancer Claude sur la même session. |

À la place du statut, la carte peut afficher « Préparation… » (le worktree se prépare) ou « Reprise à 15:00 » (l’agent a atteint la limite d’usage et repart seul à la remise à zéro du quota).

## Renommer, dupliquer, archiver, supprimer

Un clic droit sur la carte d’un agent ouvre son menu.

| Entrée | Ce qu’elle fait |
|---|---|
| Renommer | Le nom devient un champ : `Entrée` valide, `Échap` annule. Un double-clic sur le nom de la carte fait la même chose ; un double-clic sur le nom dans l’en-tête de la conversation ouvre une fenêtre « Renommer l’agent ». La branche du worktree prend le nouveau nom (`escouade/<nom>`), sauf une branche qui existait avant l’agent, une branche que tu as nommée, une branche avec une PR, une branche déjà poussée ou un nom de branche déjà pris : une notification le dit, et l’agent est renommé quand même. Le dossier du worktree ne change pas. |
| Dupliquer la conversation | Crée une copie : « nom (copie) », puis « (copie 2) »… Voir plus bas. Grisée pendant un tour (« Attends la fin de son tour. »). Absente pour un agent archivé. |
| Archiver | Range l’agent (« garde la conversation »). Voir plus bas. |
| Restaurer | Remet un agent archivé dans la liste. |
| Préparer le lancement | Demande à l’agent comment lancer son worktree sur ses propres ports (voir [les tests](testing)). Seulement pour un agent qui a un worktree, sans ticket en cours, et dont isola ne gère pas le worktree. |
| Intégrer main | Fait entrer la branche de base de l’agent dans la sienne (le nom de la base est écrit dans l’entrée, ici `main`). Seulement pour un agent qui a un worktree. Voir [les branches](branches). |
| Ouvrir dans l’éditeur | Ouvre l’[éditeur](editor) sur le worktree de l’agent, ou sur le projet. |
| Ouvrir un terminal | Ouvre un [terminal](terminals) dans le worktree de l’agent, ou dans le dossier du projet. |
| Vue Claude Code, ou Vue Escouade | Fait tourner l’agent dans l’interface de Claude Code elle-même, ou le ramène dans la conversation d’Escouade, comme le bouton de l’en-tête (voir [la vue Claude Code](claude-code-view)). Absente pour un agent de ticket. |
| Activer le remote control, ou Désactiver le remote control | Rend l’agent joignable depuis claude.ai et l’app Claude, ou le retire (voir [le remote control](remote-control)). « Désactiver » met toujours l’agent à part : un réglage allumé ensuite ne lui rend pas le remote control avant que tu le rallumes. En [vue Claude Code](claude-code-view), le changement vaut au prochain lancement de l’interface. |
| Supprimer… | Retire l’agent après confirmation. Voir plus bas. |

Un agent archivé n’a que « Renommer », « Restaurer » et « Supprimer… ».

### Dupliquer la conversation

« Dupliquer la conversation » crée un nouvel agent qui reprend la conversation de l’original : même modèle, même effort, même mode, même compte, et la session Claude Code est dupliquée là où en est l’original, qui reste tel quel. La copie s’appelle « nom (copie) ».

- Si l’original travaille dans un worktree, la copie reçoit un worktree à elle, créé à partir du commit où se trouve l’original. Ses modifications non commitées n’y sont pas, et un avis à la fin de la conversation copiée le dit. Sans worktree, la copie travaille dans le même dossier.
- Un agent de ticket est copié comme un agent ordinaire : sans ticket.
- Pour le [remote control](remote-control), la copie est un nouvel agent : un réglage allumé la couvre, mais pas le remote control que tu avais activé à la main sur l’original. Si l’original est [mis à part](remote-control#mettre-un-agent-a-part), la copie l’est aussi.
- L’entrée est grisée tant que l’original travaille ou attend une réponse.

### Archiver et restaurer

Archiver garde la conversation mais arrête l’agent : son processus Claude s’arrête, un tour en cours est interrompu, son remote control est coupé, ses lancements de test s’arrêtent. L’agent quitte la liste et rejoint le dépliant « Archivés (N) » en bas ; un clic sur lui l’ouvre en lecture, avec un bandeau « Agent archivé. » et un bouton « Restaurer ».

Si l’agent travaille sur un ticket « En cours » ou « À tester », une confirmation te prévient que le ticket repartira « À faire ». Restaurer un agent dont le worktree a été supprimé entre-temps (son ticket terminé ou supprimé, par exemple) le recrée à partir de sa branche, si elle existe encore ; sinon sa conversation le dit. Restaurer un agent lui rend son remote control si un réglage le couvre, mais pas s’il l’avait activé à la main.

### Supprimer

« Supprimer… » demande confirmation. Le processus Claude est arrêté et la conversation est retirée d’Escouade ; la session Claude Code, elle, reste sur le disque. Une case, cochée par défaut, supprime aussi le worktree de l’agent et sa branche (« Supprimer aussi le worktree et la branche escouade/refacto-auth ») ; pour un agent sur une branche qui existait avant lui, la case ne supprime que le worktree (« Supprimer aussi le worktree (la branche … est conservée) »). Si des fichiers de ce worktree sont ouverts et non enregistrés dans l’éditeur, la fenêtre te dit qu’ils seront perdus. Si le nettoyage du worktree rencontre un problème, il est signalé après coup, et l’agent est supprimé quand même.

## Fermer la fenêtre, quitter et redémarrer

Fermer la fenêtre (le bouton ✕ de la barre de titre) ne coupe pas les agents : elle se cache et Escouade reste dans la zone de notification (la barre des menus sous macOS). Les agents continuent à travailler, et leurs notifications arrivent comme d’habitude. Un clic sur l’icône ramène la fenêtre ; le menu de l’icône propose « Afficher », « Recharger l’interface » et « Quitter ». « Recharger l’interface » ramène la fenêtre sur l’interface d’Escouade si elle reste un jour bloquée ou blanche, et te demande d’abord confirmation s’il reste des fichiers non enregistrés dans l’éditeur. « Quitter » arrête tous les processus Claude ; s’il reste des fichiers non enregistrés dans l’éditeur, Escouade te le demande d’abord.

Au redémarrage, chaque agent retrouve sa conversation, et aucun processus Claude n’est lancé d’avance (sauf pour les agents au remote control). Un tour qui tournait quand l’app s’est arrêtée apparaît « Terminé » ; les agents de ticket reprennent seuls leur travail (voir [le Kanban](board)), les autres attendent ton prochain message. Quand tu affiches un agent dont le processus est arrêté, Escouade le relance en arrière-plan pour que la première réponse soit rapide (sauf s’il est archivé, en erreur ou dans [la vue Claude Code](claude-code-view)) ; sinon, c’est ton prochain message, ta prochaine réponse ou la prochaine action sur l’agent qui le relance. Dans tous les cas, Claude Code reprend la **même session** (`--resume`) : l’agent n’a rien oublié.

### Les processus inactifs

Un processus Claude occupe de la mémoire. Quand un agent n’a rien fait depuis un certain temps, Escouade arrête donc son processus, conversation et session intactes. Le délai, 30 minutes par défaut, se règle dans Réglages › Harness : « Arrêter les processus des agents inactifs après (minutes) » ; `0` désactive l’arrêt. Cela ne concerne ni un agent qui travaille ou qui attend une réponse, ni un agent dont le remote control est activé, ni un agent qui a encore en arrière-plan une commande, une surveillance, un sous-agent ou un workflow (ils vivent dans son processus, que l’arrêt tuerait). La prochaine action sur l’agent relance son processus sur sa session.

### Si un agent s’arrête

Si le processus d’un agent qui travaillait se termine sans que tu l’aies demandé, sa carte passe en « Erreur », une ligne rouge de la conversation donne le code de sortie, et un bandeau dit : « L’agent s’est arrêté. Envoie un message pour relancer Claude sur la même session. » Si Claude Code ne retrouve plus la session, un avis te le dit (« Session Claude introuvable : une nouvelle session sera démarrée au prochain message. ») et l’agent repart sur une nouvelle session, sans l’historique que Claude avait en mémoire.
