# Un worktree par agent

Un worktree donne à chaque agent son propre dossier et sa propre branche, pour que plusieurs agents travaillent sur le même dépôt sans se gêner. Cette page explique comment l’activer, le préparer, fusionner son travail et le nettoyer.

## À quoi sert un worktree

Un worktree git est un second dossier de travail du même dépôt, sur sa propre branche. Avec l’option « Un worktree par agent », chaque nouvel agent reçoit le sien : ses modifications restent à part, ni les autres agents ni ton propre dossier ne les voient tant que tu ne les as pas mergées, et le panneau des fichiers les liste séparément (voir [Git](git)).

Sans worktree, tous les agents d’un projet travaillent dans le dossier du projet et leurs modifications se mélangent : Escouade les attribue alors à chaque agent d’après ses éditions.

Les agents des tickets du [Kanban](board) travaillent toujours dans un worktree à eux, sur la branche de leur ticket, que l’option soit activée ou non. Ils suivent les mêmes réglages de dossier, de fichiers copiés et de commandes que les autres.

## L’activer

L’option se règle par projet, et ne concerne que les agents créés ensuite : ceux qui existent déjà restent où ils sont. Trois endroits la proposent :

- la fenêtre « Nouveau projet », avec l’interrupteur « Un worktree git par agent » (désactivé par défaut) ;
- un clic droit sur l’onglet du projet, puis « Activer le worktree par agent » ou « Désactiver le worktree par agent » (l’entrée précise « nouveaux agents ») ;
- Réglages › Projets, groupe « Git », « Un worktree par agent ».

Le dossier du projet doit être sur une branche qui a au moins un commit : un dépôt neuf, sans aucun commit, ne peut pas encore avoir de worktree. Quand un worktree ne peut pas être créé, l’agent travaille dans le dossier du projet et la conversation affiche « Worktree non créé, l’agent travaille dans le dossier du projet : … » avec la raison.

## Où il va et comment il s’appelle

Quatre réglages du projet décident de l’emplacement et des noms (Réglages › Projets, groupe « Git »). La liste complète des réglages est dans [les réglages des projets](settings-projects).

| Réglage | Par défaut | Ce qu’il fait |
|---|---|---|
| « Dossier des worktrees » | `.claude/worktrees` | Où vont les nouveaux worktrees : relatif au dossier du projet (`../app-worktrees`, `.worktrees`), à ton dossier personnel (`~/worktrees/app`) ou complet. Sous le champ, « Le prochain : … » montre où irait le suivant. Ce ne peut pas être le dossier du projet lui-même, un dossier qui le contient ni un dossier dans `.git`. Les worktrees déjà créés restent où ils sont. |
| « Préfixe des branches des agents » | `escouade/` | La branche d’un nouvel agent s’appelle `<préfixe><nom>`. Vide, c’est son nom seul. |
| « Préfixe des branches des tickets » | `ticket/` | La branche d’un ticket, celle de sa pull request, s’appelle `<préfixe><clé>`. Vide, c’est sa clé seule. |
| « Fichiers copiés dans les worktrees » | `.env*` | Les fichiers que git ignore et qu’un nouveau worktree reprend du projet (voir [plus bas](#les-fichiers-copies)). |

Le dossier du worktree est `<dossier des worktrees>/<nom de l’agent>`, sur une branche nouvelle `<préfixe><nom de l’agent>`. Si l’un des deux existe déjà, Escouade ajoute `-2`, `-3`… Par défaut, le worktree d’un nouvel agent attend son premier message, qui lui donne son nom : branche et dossier prennent ce nom (voir [plus bas](#au-premier-message)). Avec ce réglage éteint, un agent s’appelle d’abord `agent-1`, `agent-2`… et reçoit son worktree tout de suite ; quand Escouade le nomme d’après son premier message (par exemple `refacto-auth`), sa branche devient `escouade/refacto-auth` si ce nom est libre, alors que le dossier garde son premier nom. Un nom que tu lui donnes toi-même change aussi sa branche, aux mêmes conditions, sauf si elle est déjà poussée (elle a une branche amont, ou le dépôt distant en a une du même nom) : la branche garde alors son nom, et une notification le dit. Le dossier ne suit jamais le nom : pour un dossier nommé d’après la branche, crée l’agent avec « Nouvel agent sur une branche… » (plus bas). La carte de l’agent montre la branche sans son préfixe (`⎇ refacto-auth`), et son nom entier s’affiche au survol.

### Au premier message

Le réglage « Créer le worktree au premier message, au nom de l’agent » (Réglages › Projets, groupe « Git », et l’étape « Worktrees » de l’assistant) est activé par défaut, pour les projets existants aussi. Avec lui, « Nouvel agent » crée l’agent tout de suite, mais sans worktree ni branche : sa carte affiche « ⎇ au premier message » et sa conversation « Le worktree sera créé au premier message, au nom de l’agent. » Au premier message :

1. Le modèle des textes rédigés nomme l’agent, d’après ce message (par exemple `refacto-auth`). Il a 15 secondes : passé ce délai, ou s’il échoue, l’agent garde `agent-N` et rien n’attend. Un nom qui arrive plus tard renomme la branche, comme quand le réglage est éteint.
2. Le worktree est créé : la branche `<préfixe><nom>` et le dossier `<nom>`, avec `-2`, `-3`… si l’un des deux est pris.
3. Les fichiers sont copiés et les commandes de préparation tournent dans ce dossier.
4. Le message part une fois la préparation finie.

Pendant ce temps, la conversation montre l’étape en cours : « Nom de l’agent », « Création du worktree refacto-auth », puis les commandes de préparation. Avant le premier message, le terminal de l’agent et « Ouvrir dans l’éditeur » attendent : il n’y a pas encore de dossier à ouvrir.

Ne sont pas concernés, et reçoivent leur worktree tout de suite comme avant : les agents des tickets, un agent auquel tu donnes un nom à sa création, un agent créé avec « Nouvel agent sur une branche… », une copie d’agent, et un agent en vue Claude Code (son interface tourne dès sa création). Éteins le réglage pour un projet dont la préparation est longue, si tu préfères qu’elle tourne pendant que tu écris le premier message.

Le worktree part du commit où se trouve le dossier du projet à ce moment-là. La branche de ce dossier devient la **branche de base** de l’agent : c’est elle que le merge et « Intégrer » visent.

Quand le dossier des worktrees est dans le dépôt (le cas par défaut), Escouade l’ajoute au fichier `.git/info/exclude` du dépôt : les worktrees n’apparaissent pas dans les changements du projet. Un dossier qui contient déjà des fichiers suivis par git est refusé : « Le dossier des worktrees … contient des fichiers suivis par git : choisis-en un autre dans les réglages du projet. »

## Les fichiers copiés

Un fichier comme `.env`, que git ignore, n’existe pas dans un worktree neuf. Juste après la création du worktree, Escouade y copie les fichiers du projet qui sont ignorés par git et qui correspondent à un motif du réglage « Fichiers copiés dans les worktrees ».

- Un motif par ligne, avec `*` et `?` dans un nom et `**` pour traverser des dossiers : `.env*` prend les fichiers de la racine, `**/.env*` ceux de tous les dossiers. Par défaut, `.env*`.
- Seuls les fichiers ignorés par git sont copiés, donc aucun ne peut finir dans un commit. Un fichier suivi par git n’est jamais copié.
- Les copies apparaissent dans l’arborescence de [l’éditeur](editor), en grisé (« Ignoré par git »). Le commit direct ne les prend jamais.
- Si une copie échoue, la conversation le dit (« Fichiers non copiés dans le worktree : … ») et l’agent travaille sans ce fichier.

## Préparer le worktree

Un worktree neuf n’a ni dépendances installées ni code généré. Le groupe « Worktrees » des réglages du projet liste les commandes que lance Escouade pour le préparer, dans la liste « À l’ouverture d’un worktree » : elles tournent dans l’ordre, dans le nouveau worktree, avant le premier message de l’agent, et ses messages attendent la fin.

Chaque commande a un shell (le premier shell détecté par défaut), une ligne de commande (par exemple `npm ci`) et un « Sous-dossier » du worktree (vide, c’est sa racine). « + Ajouter une commande » en ajoute une, la croix la supprime. Pour changer l’ordre, utilise les boutons ↑ et ↓ de la commande, ou `Alt+↑` et `Alt+↓` (`⌥↑` et `⌥↓` sous macOS) depuis n’importe quel champ de la commande. Une commande vide est ignorée.

Les commandes reçoivent ces variables d’environnement :

| Variable | Valeur |
|---|---|
| `ESCOUADE_PROJECT_DIR` | Le dossier du projet. |
| `ESCOUADE_WORKTREE_DIR` | Le dossier du worktree. |
| `ESCOUADE_BRANCH` | La branche du worktree. |
| `ESCOUADE_PORT_BASE`, `ESCOUADE_PORT_END` | Les ports réservés au worktree d’un ticket, quand il en a (voir [les tests](testing)). |

Le proxy du réglage « Proxy HTTP(S) » est transmis aux commandes quand « Appliquer aussi le proxy aux terminaux intégrés » est activé (voir [les réglages du réseau](settings-network)). Une commande qui dépasse 20 minutes est arrêtée.

### Suivre la préparation

- La carte de l’agent dit « Préparation… » ; son info-bulle donne la commande en cours.
- La conversation affiche « Préparation du worktree · npm ci — tes messages partiront une fois terminée. » Le bouton « Voir la sortie » déplie les dernières lignes de ce que la commande écrit, en direct. La sortie suit les nouvelles lignes tant que tu es en bas, et reste en place si tu remontes.
- Un message que tu envoies pendant la préparation attend : il part une fois la préparation finie.
- À la fin, la conversation note « Worktree préparé (3 commandes, 12 s). »
- La sortie complète de la dernière préparation, commande après commande, est écrite dans `~/.escouade/logs/`, un fichier par agent nommé `<identifiant de l’agent>-setup.log`. Il est remplacé à chaque préparation et supprimé avec l’agent.

Une commande de préparation peut se servir du nom du dossier du worktree, par exemple pour une adresse locale (`<dossier>.localhost`) ou le nom d’une base de données : avec le réglage [« Créer le worktree au premier message, au nom de l’agent »](#au-premier-message), ce nom est celui de l’agent, et non `agent-N`. Le dossier ne change plus ensuite, même si l’agent est renommé.

Archiver ou supprimer l’agent pendant sa préparation l’arrête.

### Quand une commande échoue

La première commande qui échoue arrête la préparation : les suivantes ne tournent pas. La conversation affiche un avertissement, par exemple « La préparation du worktree a échoué sur `npm ci` (code 1). », suivi des 30 dernières lignes de la sortie et du chemin de la sortie complète (« Sortie complète : … »). L’échec peut aussi venir d’un sous-dossier qui n’existe pas dans le worktree, d’une commande qui ne démarre pas ou d’une commande arrêtée après 20 minutes.

L’agent est prévenu dans son premier message, avec la commande qui a échoué et ses dernières lignes, et il démarre quand même : il peut en tenir compte.

### Remplir automatiquement

« ✦ Remplir automatiquement », en haut du groupe « Worktrees », demande à Claude de lire le projet (manifestes, lockfiles, README…) et de proposer les commandes. Il ne modifie rien : il n’a que des outils de lecture, et il dispose de 4 minutes au plus. Sa proposition s’affiche à part, sous « Commandes de worktree proposées », avec chaque commande en entier, son shell et son dossier :

- « Remplacer les commandes » remplace les deux listes du brouillon (préparation et démontage). Rien n’est gardé tant que tu n’as pas cliqué « Enregistrer » dans la fenêtre des réglages ;
- « Ignorer » abandonne la proposition.

Claude propose huit commandes au plus par liste. Une commande sur plusieurs lignes, avec des caractères invisibles, avec une suite de 24 espaces ou plus, ou de plus de 300 caractères est écartée, et le message qui suit la lecture le dit (« 2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant d’enregistrer. »).

> [!WARNING]
> Une fois enregistrées, ces commandes tournent seules dans chaque nouveau worktree. Lis chacune en entier avant d’enregistrer.

## Démonter le worktree

La liste « Avant sa suppression » contient ce qu’il faut défaire hors du worktree avant de le supprimer : une base de données ou des conteneurs que la préparation a créés pour lui. Le plus souvent, elle est vide. Elle s’édite comme celle de la préparation, et se remplit aussi avec « ✦ Remplir automatiquement ».

Ces commandes tournent quand un agent est supprimé avec son worktree, quand le Kanban nettoie le worktree d’un ticket terminé ou supprimé, et avec « Nettoyer ». Chacune tourne même si la précédente a échoué, avec 5 minutes au plus, et leur sortie n’est pas affichée. Si le projet utilise isola (un fichier `.isola.toml`), `isola destroy` passe ensuite. Un problème est signalé une fois l’agent supprimé : « Agent supprimé, mais le démontage du worktree a échoué sur … ».

## Travailler sur une branche choisie

Un agent peut aussi travailler dans un worktree sur une branche que tu choisis, qui existe déjà ou que tu crées pour lui :

- « Nouvel agent sur une branche… » dans le menu du bouton ▾, à droite de « Nouvel agent », ouvre le sélecteur de branches (« Choisir une branche ») ; un nom tapé qu’aucune branche ne porte y propose « Créer la branche « feat/signup » et y lancer un agent » ;
- dans le sélecteur de branches de la barre d’état, un clic droit sur une branche propose « Lancer un agent sur cette branche » ;
- dans le formulaire d’un ticket, le champ « Branche » propose « Reprendre une branche existante… ».

Une branche distante est reprise par la branche locale qui la suit, créée si elle n’existe pas. Le dossier du projet n’est pas touché. Trois branches sont refusées, avec la raison : celle du dossier du projet, la branche cible du Kanban (les tickets y sont mergés) et celle qu’un autre worktree occupe déjà.

Le dossier du worktree porte le nom de la branche, sa dernière partie : `feat/login` donne `.claude/worktrees/login`, puis `login-2` si ce dossier est déjà pris. Un script de préparation peut s’en servir pour une adresse locale (`login.localhost`) ou un nom de base de données, et savoir ainsi quelle instance tourne sur quelle branche. Ce nom reprend la branche telle quelle, majuscules, `_` et points compris ; `ESCOUADE_BRANCH` donne la branche entière.

### Créer la branche depuis le sélecteur

Tape le nom de la nouvelle branche dans le sélecteur de « Nouvel agent sur une branche… ». Si aucune branche, locale ou distante, ne porte ce nom, la dernière ligne propose « Créer la branche « feat/signup » et y lancer un agent », suivie de son point de départ (« depuis main »). La branche part du même endroit qu’un nouvel agent : la branche cible du Kanban si « Les agents partent de la branche cible » est coché (voir [les réglages du Kanban](settings-board)), sinon la branche du dossier du projet. La ligne sous la liste le dit quand le curseur est sur l’entrée.

Le nom est vérifié comme git le fait. Un nom refusé grise l’entrée, avec la raison, et rien n’est créé. Un nom qui commence comme une branche d’un dépôt distant (`origin/…`) est refusé aussi.

Une branche créée ainsi appartient à l’agent, comme celle d’un nouvel agent : supprimer l’agent avec son worktree la supprime aussi. Elle garde toujours le nom que tu lui as donné, même quand l’agent est nommé ou renommé.

### Une branche qui existait déjà

Une branche qui existait avant l’agent n’est jamais créée ni supprimée par Escouade : en supprimant l’agent, la case propose « Supprimer aussi le worktree (la branche … est conservée) ». Sa branche de base est la branche cible du Kanban si elle existe, sinon la branche du dossier du projet. Pour un ticket, l’infobulle de l’entrée dit « Escouade ne supprime jamais cette branche, même une fois le ticket validé. » Les branches sont détaillées dans [la page des branches](branches).

## Fusionner dans la branche de base

Quand le travail d’un agent te convient, tu le fusionnes dans sa branche de base depuis le panneau des fichiers :

1. L’agent doit avoir commité son travail (voir [Git](git)). Le merge ne prend que des commits.
2. Dans le panneau, onglet « Non commités », bouton « Cet agent », clique « Merger escouade/refacto-auth → main… ».
3. La fenêtre demande « Merger escouade/refacto-auth dans main ? » et précise que les commits de l’agent sont intégrés dans la branche « main » du projet. La case « Squash (un seul commit) » est décochée par défaut. Clique « Merger ».

Le merge se fait dans le dossier du projet.

| Choix | Résultat |
|---|---|
| Sans « Squash » | Un commit de merge (`Merge branch 'escouade/refacto-auth'`) qui garde tous les commits de l’agent. |
| Avec « Squash (un seul commit) » | Un seul commit, nommé d’après l’agent, qui liste les sujets des commits regroupés. |

Escouade refuse le merge, avec un message, dans ces cas :

- l’agent a des fichiers non commités (« L’agent a 2 fichiers non commités : demande-lui de commiter avant de merger. ») ;
- le dossier du projet a des modifications non commitées sur des fichiers suivis : commite-les ou mets-les de côté ;
- la branche de l’agent n’a aucun nouveau commit (« Rien à merger ») ;
- la branche de base n’existe plus.

Si le dossier du projet n’est pas sur la branche de base, une question s’affiche : « Basculer sur « main » ? » Elle dit sur quelle branche est le projet. « Basculer et merger » bascule le dossier sur la base, puis merge.

Un merge qui a des conflits est annulé : le dossier du projet reste comme avant et un message d’erreur s’affiche. Dans ce cas, fais d’abord entrer la base dans la branche de l’agent avec « Intégrer main », qui confie les conflits à l’agent (voir [les branches](branches)), puis recommence.

Le merge ne supprime ni le worktree ni la branche de l’agent. Pour les ranger, supprime l’agent avec son worktree (voir ci-dessous) ; pour nettoyer d’un coup les branches déjà mergées, utilise « Branches mergées » dans [le sélecteur de branches](branches). Pour les tickets du Kanban, la validation se charge du commit, du merge ou de la pull request selon les réglages (voir [les réglages du Kanban](settings-board)).

## Ouvrir une pull request

Plutôt que de merger toi-même, tu peux proposer le travail d’un agent en pull request, sur GitHub pour l’instant (GitLab, Bitbucket et Gitea viendront). Il faut que le projet ait son dépôt distant chez l’hébergeur et un compte connecté dans [Réglages › Intégrations](settings-integrations) ; sinon, rien ne le propose.

« Ouvrir une PR » est dans l’en-tête de la conversation, dans le menu d’un clic droit sur la carte de l’agent, et dans le menu de sa branche du [sélecteur de branches](branches). La fenêtre propose un titre et une description, rédigés par le modèle des textes rédigés d’après les commits et le diff de la branche : relis-les, corrige-les, choisis la branche cible et le brouillon si tu veux. Rien ne part avant « Pousser et ouvrir », qui pousse la branche puis ouvre la PR. Les fichiers non commités du worktree n’y sont pas, et la fenêtre le dit.

La carte de l’agent montre ensuite ses PR sous ses lignes, trois au plus, celles à merger d’abord : leur numéro ouvre la PR dans le navigateur, et son infobulle dit son état, sa CI et ses relectures. Escouade les relit quand tu affiches l’agent, puis chaque minute tant qu’il est affiché, et plus du tout une fois mergées. Une PR ouverte par quelqu’un d’autre sur la même branche compte aussi.

Un agent sans worktree d’Escouade qui travaille dans un worktree à lui, créé par Claude Code ou où il est allé par un `cd`, a aussi ses PR : Escouade suit la branche du dossier où sa session travaille, et son en-tête la dit. Jamais celle du dossier du projet, ni le worktree d’un autre agent.

Quand la CI échoue, « L’agent corrige » envoie à l’agent les checks en échec avec la fin de leurs journaux. Quand une relecture demande des changements ou laisse des fils ouverts, « L’agent corrige les retours » lui envoie les demandes et les commentaires, chacun avec son fichier et sa ligne. Ces messages ne partent que quand tu cliques : l’agent corrige, commite et pousse, et la PR se met à jour. Il ne répond pas sous les fils à ta place.

Une fois une PR ouverte, la branche de l’agent garde son nom, même s’il est renommé ensuite.

## Archiver, restaurer, supprimer

Les actions sont dans le menu d’un clic droit sur la carte de l’agent.

- « Archiver » arrête l’agent et garde sa conversation. Son worktree et sa branche restent sur le disque (le Kanban nettoie celui d’un ticket fini, voir plus bas). La confirmation n’apparaît que pour un agent dont le ticket est en cours ou à tester.
- « Restaurer » ramène un agent archivé. Si son dossier de worktree a disparu entre-temps, par exemple après une pull request ou un push, Escouade le recrée à partir de sa branche, recopie les fichiers du projet et relance la préparation. Si un autre worktree a pris son dossier, le sien est recréé à côté (`-2`) et la conversation l’indique. Si la branche n’existe plus (mergée puis supprimée), l’agent reste tel quel et sa conversation le dit.
- « Supprimer… » demande confirmation. La case « Supprimer aussi le worktree et la branche escouade/refacto-auth » est cochée par défaut. Cochée, Escouade arrête l’agent, lance les commandes de démontage, supprime le dossier du worktree puis la branche. Décochée, l’agent disparaît d’Escouade et le worktree et la branche restent sur le disque. Si des fichiers de ce worktree sont ouverts et non enregistrés dans l’éditeur, la fenêtre prévient qu’ils seront perdus.

> [!WARNING]
> La branche de l’agent est supprimée même si elle n’a pas été mergée : ses commits n’existent alors plus nulle part. Merge-la d’abord si tu veux garder son travail.

Si le nettoyage rencontre un problème (un processus qui tient encore le dossier, un démontage qui échoue), l’agent est supprimé quand même et un message le signale après coup. Fermer un projet ne touche pas à ses worktrees sur le disque.

## Les worktrees qui restent

Le worktree d’un ticket terminé ou supprimé part tout seul (réglage « Supprimer le worktree d’un ticket terminé ou supprimé » de l’onglet [« Kanban »](settings-board)). Pour les autres agents archivés, ou ce qui reste d’avant, Réglages › Projets › « Worktrees restés » › « Nettoyer » supprime les worktrees des agents archivés du projet, après leurs commandes de démontage, et oublie ceux dont le dossier a déjà disparu (`git worktree prune`).

Aucun travail n’y est perdu : un worktree qui a des changements non commités, ou des commits qui ne sont ni dans la branche cible ni dans une autre branche (poussée, proposée en PR), est gardé. La rangée dit combien de worktrees sont supprimés, puis ceux qu’elle garde et pourquoi ; la carte de l’agent archivé le dit aussi. La branche d’un worktree supprimé part avec lui si son travail est dans la branche cible ; sinon elle reste, et « Restaurer » recrée le worktree à partir d’elle.

## Dupliquer un agent

« Dupliquer la conversation » (menu de l’agent) copie un agent qui a un worktree dans un worktree à lui, créé à partir du commit où se trouve l’original, sur une nouvelle branche dont le nom reprend celui de la copie. Les modifications non commitées de l’original n’y sont pas, et la conversation le dit. Voir [les agents](managing-agents).
