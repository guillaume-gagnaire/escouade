# Réglages : Projets

L’onglet « Projets » règle un projet à la fois : son nom, sa couleur et sa tuile, son remote control, ses worktrees, la façon dont ses commits sont rédigés et ses commandes de préparation, de démontage et de lancement.

## Choisir le projet

L’onglet agit sur le projet choisi dans la rangée « Projet », sous l’en-tête de la [fenêtre des réglages](settings-application) : un bouton par projet ouvert. Ce que tu changes sur un projet reste dans le brouillon quand tu passes à un autre, et tout s’enregistre ensemble avec « Enregistrer ».

## Plugins proposés

Quand le `.escouade.yml` du projet propose des plugins (`plugins.recommended`) qui ne sont pas installés sur ta machine, une bande en tête de l’onglet les nomme, chacun avec ce qu’il demande. « Voir dans le store » ouvre sa page dans la vue Plugins, d’où tu l’installes après avoir lu ses permissions : la bande n’installe ni n’active rien. « Pas pour ce projet » la fait taire sur cette machine. Dans un projet privé, elle rappelle qu’un plugin installé n’y est actif qu’une fois activé pour lui. Voir [.escouade.yml](escouade-yml#plugins).

## Identité

| Réglage | Effet | Par défaut |
|---|---|---|
| « Nom » | Le nom du projet, tel que le montrent la barre de titre, l’infobulle de sa tuile et le sélecteur de projet. Il ne peut pas rester vide : « Enregistrer » est alors bloqué. | Le nom donné à la création, celui du dossier si tu n’en as pas donné |
| « Dossier » | Le dossier du projet, affiché sans pouvoir être modifié. Le bouton « Ouvrir » l’ouvre avec le gestionnaire de fichiers du système. | — |
| « Couleur » | Une pastille parmi 14 (13 teintes et un ton neutre). La couleur repère le projet et teinte l’interface quand il est affiché. | La première couleur qu’aucun projet n’a prise, à la création |
| « Lettres de la tuile » | Ce que dit la tuile du projet dans la barre des projets : une à trois lettres ou chiffres (« API », « v2 »), ou un emoji seul. Le reste de ce que tu tapes (espaces, signes, une quatrième lettre) ne s’écrit pas. La tuile est montrée à côté du champ, telle que la barre la dessinera. | Vide : les initiales du nom (`demo-api` → « DA », `infra` → « In »), montrées en indication dans le champ |
| « Logo » | Une image à la place des lettres : « Choisir une image… » la prend (PNG, JPEG, WebP, SVG ou ICO, 1 Mo au plus), « Retirer » revient aux lettres. Elle prend la forme de la tuile, entière, sur la couleur du projet là où elle est transparente ; les agents qui attendent, ceux qui travaillent, le cadenas d’un projet privé et l’alerte restent dessinés par-dessus. | Aucun |
| « Libellé sous la tuile » | Un libellé sous la tuile du projet, dans la barre des projets, sur une ligne et coupé en « … » s’il est trop long (son nom entier reste dans l’infobulle). L’interrupteur l’allume ; le champ donne un texte plus court que le nom, 12 caractères au plus. Élargie au-delà de 160 px, la barre montre ce texte (ou le nom) à droite de chaque tuile, allumé ou non. | Éteint ; vide, le texte est le nom du projet |
| « Compte préféré » | Le compte sur lequel partent les nouveaux agents et les tickets du projet. Visible seulement avec plusieurs comptes. Choix : « Automatique » (le premier compte actif sous le seuil de pause, dans l’ordre des comptes) ou un des comptes actifs qui servent ce projet. | « Automatique » |

Le logo est reconnu par son contenu, quelle que soit son extension. Une image trop lourde, illisible ou d’un autre format est refusée avec un message qui dit pourquoi (« « big.png » pèse 3 Mo : un logo pèse 1 Mo au plus. »), et la tuile reste comme elle était. Escouade en garde une copie dans son dossier de données, sous le projet : déplacer ou effacer le fichier d’origine ne change rien. Un SVG est affiché comme une image : rien de ce qu’il contient ne s’exécute. Les lettres et le logo restent sur ce poste, comme le nom et la couleur : ils ne vont pas dans `.escouade.yml`. Un projet fermé puis rouvert les retrouve ; fermé en effaçant son historique (ou son historique effacé plus tard), il efface sa copie du logo. Les mêmes champs sont dans l’assistant « Nouveau projet », sous le nom et la couleur.

Le libellé reste sur ce poste lui aussi : il ne va pas dans `.escouade.yml`. Tant que tu n’as pas tiré le bord de la barre des projets, elle passe de 52 à 72 px dès qu’un projet a son libellé allumé ; voir [la barre des projets](interface#sa-largeur). Le même réglage est dans l’assistant « Nouveau projet », sous les lettres et le logo.

Les comptes sont décrits dans [les comptes](accounts) et réglés dans l’onglet [« Comptes »](settings-accounts). Un compte désactivé depuis qu’il a été choisi reste affiché avec « (inactif) » tant qu’il est choisi, et il est ignoré en attendant.

## Remote control

| Réglage | Effet | Par défaut |
|---|---|---|
| « Remote control pour les agents de ce projet » | Chaque agent de ce projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le processus `claude` de chacun, au plus 3 à la fois, et il reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. | Désactivé |

Pour mettre un agent à part, fais un clic droit sur sa carte, puis « Désactiver le remote control ». Éteint, le réglage retire le remote control aux agents qui le tenaient de lui, sans toucher à ceux que tu as activés à la main ni à ceux que le réglage global couvre encore.

Quand « Remote control pour tous les agents » est allumé dans l’onglet [« Harness »](settings-claude-code), l’interrupteur du projet se montre allumé et grisé, avec « Activé pour tous les projets dans Réglages › Harness ». Le choix propre du projet est gardé : une fois le réglage global éteint, l’interrupteur retrouve son état. Voir [le remote control](remote-control).

## Confidentialité

| Réglage | Effet | Par défaut |
|---|---|---|
| « Projet privé » | Les agents des autres projets ne le voient pas par le serveur MCP d’Escouade : ni dans les listes, ni par son nom, ses tickets ou ses agents. Les siens ne voient que lui. Toi, Claude hors Escouade et les vues de l’app voyez tout. Un cadenas le marque, sur sa tuile, à côté de son nom dans la barre de titre et dans la rangée « Projet ». | Désactivé, ou « Nouveaux projets privés » pour un projet ajouté |

| « Comptes » (projet privé) | Les comptes de son harness par défaut où ses agents peuvent partir : réservés à lui seul, ou partagés avec d’autres projets (sessions, mémoire, historique en commun). « Ajouter un compte réservé… » et « Réserver Pro à ce projet » y remédient. | — |
| « Comptes partagés à la limite » (projet privé) | À la limite d’usage de ses comptes réservés, ses agents peuvent continuer sur un compte partagé (changement de compte, passage à un autre harness). Éteint, ils attendent la remise à zéro. | Désactivé |

Voir [les projets privés](mcp#les-projets-prives) et [un compte pour un projet privé](accounts#un-compte-pour-un-projet-prive).

### Ses fichiers

Pour un projet privé, les agents des autres projets ne lisent ni n’écrivent son dossier, ses worktrees, ses données dans le dossier d’Escouade (tickets, fichiers joints, conversations) ni ses sessions de harness. Ses propres agents ne lisent rien de tout cela pour les autres projets, ni le reste du dossier d’Escouade. Escouade donne ces règles à chaque harness au démarrage d’un agent, et l’onglet dit, sous « Projet privé », ce que chacun garantit :

| Harness | Protection | Ce qui n’est pas garanti |
|---|---|---|
| Claude Code | Garde-fou : ses règles refusent ces chemins à ses outils de fichiers et aux commandes qu’il reconnaît (`cat`, `sed`, `Get-Content`…), même en mode Bypass. | Un script qui ouvre un fichier lui-même. Avec son bac à sable activé (macOS, Linux, WSL2), toutes ses commandes sont tenues aussi. |
| Codex | Barrière : son bac à sable refuse ces chemins à toutes ses commandes. | Le mode Bypass, qui n’a pas de bac à sable. |
| OpenCode | Garde-fou : ses règles refusent ces chemins à ses outils et aux commandes qui les nomment. | Une commande qui les atteint sans les nommer : il n’a pas de bac à sable. |
| Copilot CLI | Garde-fou : Escouade refuse d’office ce qu’il demande sur ces chemins, sans te le proposer. | Le mode Bypass, où il ne demande rien, et la vue Copilot CLI, où ses demandes ne passent pas par Escouade. |

Un garde-fou est une règle du harness, qu’une commande peut contourner ; une barrière tient pour toutes ses commandes. Une vraie barrière pour tous les harness demanderait un compte système par projet : Escouade ne le propose pas, les harness et leurs comptes n’y suivraient pas. Quand tu changes « Projet privé », un agent déjà lancé prend ses nouvelles règles avant son prochain tour : son process repart, sa conversation reste. Un agent ouvert en vue terminal les prend à son prochain lancement.

## Workflow

Escouade lit le workflow du projet dans les fichiers de tes harness : leurs commandes, skills, sous-agents et prompts, `PROMPT.md`, les fichiers d’état et les consignes, avec ce qu’ils incluent par `@`. Sur ta machine, sans coût.

- **Lu par les règles à 10:12** : l’heure de la lecture, et les fichiers lus, dossier par dossier. **Relire** refait la lecture.
- **Affiner avec un agent** : un agent court relira ce que les règles ont trouvé. Le bouton arrive dans une prochaine version.
- **Relire quand les fichiers changent** : quand la fenêtre revient au premier plan, si un fichier lu a changé. Éteint, seul « Relire » relit.
- **Trouvés** : les workflows du projet (le nombre d’étapes, de portes et de sous-agents), la boucle des tickets du Kanban, et les commandes seules, qui ne sont pas des workflows. Un clic choisit celui dont tu corriges les étapes.
- **Étapes** : renomme une étape, fusionne-la avec la suivante, retire-la, déplace-la (glisse son numéro, ou Alt+↑ et Alt+↓ dans son nom), ajoute celle que la lecture a manquée avec ce qui la signale (une commande, un sous-agent, un fichier écrit). Une étape devinée, en pointillé, attend « Garder » ou « Retirer ». « Ce n’est pas un workflow » l’écarte, « Rétablir » le reprend. Tes corrections passent devant une nouvelle lecture ; celle qui ne correspond plus à rien est signalée.
- **Où garder la configuration des workflows** : « Dans le dépôt » (par défaut) ou « Sur cette machine », pour les workflows tirés de la config du dépôt. Chaque workflow dit où il est gardé : « dans le dépôt » ou « sur cette machine (hors du dépôt) » ; ceux de tes propres dossiers (`~/.claude`, GSD…) restent toujours sur cette machine. Voir [où c’est gardé](workflows#ou-c-est-garde).
- **Mis à jour par la relecture** : une relecture a trouvé le workflow changé et a mis son fichier à jour (l’infobulle dit ce qui a changé) ; tes corrections restent.
- **Du dépôt** : un workflow que cette machine ne lit pas elle-même, venu du fichier d’un coéquipier ; il est montré et suivi quand même. « Retirer du dépôt » supprime son fichier.
- Un fichier illisible est dit sous la lecture, avec son chemin et la raison.

## Git

| Réglage | Effet | Par défaut |
|---|---|---|
| « Un worktree par agent » | Isole les modifications de chaque nouvel agent dans sa propre branche. | Choisi à la création du projet (désactivé tant que tu ne le coches pas) |
| « Créer le worktree au premier message, au nom de l’agent » | Le premier message nomme l’agent, puis sa branche et son dossier prennent ce nom avant la préparation (voir [au premier message](worktrees#au-premier-message)). Éteint, le worktree est créé tout de suite, en agent-N. | Activé |
| « Branche de départ des nouveaux worktrees » | La branche dont part le worktree d’un nouvel agent, qui est aussi sa base (merge, « Intégrer », pull request) : « Branche actuelle du dossier », « Branche cible du Kanban », ou une branche choisie dans le sélecteur, locale ou distante (voir [la branche de départ](worktrees#la-branche-de-depart)). « Nouvel agent depuis une autre branche… » en choisit une autre pour un agent seul. | « Branche actuelle du dossier » |
| « Dossier des worktrees » | Où vont les worktrees des agents du projet. | Vide : `.claude/worktrees` |
| « Préfixe des branches des agents » | Début du nom de la branche d’un nouvel agent : le préfixe suivi du nom de l’agent. Vide, la branche prend le nom de l’agent seul. | `escouade/` |
| « Préfixe des branches des tickets » | Début du nom de la branche d’un ticket, celle de sa pull request : le préfixe suivi de la clé du ticket. Vide, la branche prend la clé seule. | `ticket/` |
| « Fichiers copiés dans les worktrees » | Les fichiers copiés du projet dans chaque nouveau worktree, un motif par ligne. | `.env*` |
| « Commit » | Qui rédige les commits demandés depuis le panneau des fichiers. | « Rédigé par l’agent » |

Les worktrees sont expliqués dans [les worktrees](worktrees).

### Dossier des worktrees

Tu peux écrire un chemin relatif au dossier du projet (`../app-worktrees`, `.worktrees`), un chemin relatif à ton dossier personnel (`~/worktrees/app`) ou un chemin complet. Les guillemets qu’ajoute « Copier en tant que chemin » sont retirés. Laissé vide, le champ vaut `.claude/worktrees`, comme `claude --worktree`.

Sous le champ, une ligne dit où ira le prochain worktree (« Le prochain : » suivi du chemin, avec `<nom>` pour le nom de l’agent). Les worktrees déjà créés restent où ils sont. Hors du projet, la vue Claude Code demande d’abord ta confiance pour chacun.

Un dossier refusé remplace cette ligne par un message rouge et bloque « Enregistrer » :

- le dossier du projet lui-même ;
- un dossier qui contient celui du projet : choisis-en un à côté (`../app-worktrees`) ou dedans (`.worktrees`) ;
- un dossier situé dans un dossier `.git` ;
- sous Windows, un chemin qui n’est ni relatif ni complet, comme `D:wt` ou `\wt`.

### Préfixes de branche

Les noms de branche des agents et des tickets sont ceux qui partent sur le dépôt distant avec une pull request : les préfixes servent à suivre la convention d’un dépôt. Les [branches](branches) sont décrites à part. Sous chaque champ, une ligne montre la forme obtenue.

Un préfixe qui ne donnerait pas un nom de branche valide est refusé pendant que tu tapes, et bloque « Enregistrer ». Sont refusés : un espace, `..`, `~`, `^`, `:`, `?`, `*`, `[`, `\` et `@{` ; un `-` ou un `/` au début ; `//` ; une partie du nom qui commence par `.` ou finit par `.lock`.

### Fichiers copiés dans les worktrees

Le champ est une zone de texte, un motif par ligne : `.env*` copie les `.env` de la racine, `**/.env*` ceux de tous les dossiers. Seuls sont copiés les fichiers que git ignore, et ils ne sont jamais commités. Les lignes vides sont ignorées.

### Commit

Ce choix concerne les boutons « Commit… » et « Commit tout… » du panneau des fichiers, décrits dans [Git](git).

- « Rédigé par l’agent » : l’agent est invité à commiter ses modifications.
- « Direct, avec un message proposé » : Escouade propose un message, tu le relis et tu commites toi-même.

## Worktrees

Ce groupe règle les commandes qu’Escouade lance quand un worktree du projet s’ouvre, et avant qu’il soit supprimé. Aucune commande n’est définie au départ.

| Réglage | Effet |
|---|---|
| « À l’ouverture d’un worktree » | La préparation. Les commandes tournent dans l’ordre, avant le premier message de l’agent : dépendances, code généré… Les messages attendent la fin. |
| « Avant sa suppression » | Le démontage. Ce que la préparation a créé hors du worktree (base de données, conteneurs…) ; souvent rien. |
| « Worktrees restés » | « Nettoyer » supprime les worktrees des agents archivés, après leurs commandes de démontage, sauf ceux qui gardent des changements non commités ou des commits ni mergés ni poussés, et dit ce qu’il a fait. Voir [les worktrees](worktrees#les-worktrees-qui-restent). |

Chaque commande est une ligne numérotée.

| Champ | Effet |
|---|---|
| « Shell » | Le shell qui la lance, parmi ceux que l’app détecte. Une nouvelle commande prend le premier trouvé. Un shell que la machine n’a pas s’affiche avec « (introuvable) ». |
| « Commande » | La ligne de commande, comme `npm ci`. |
| « Sous-dossier » | Le dossier du worktree où elle tourne. Vide : sa racine. |
| ↑ et ↓ | Montent ou descendent la commande. `Alt+↑` et `Alt+↓` (`⌥↑` et `⌥↓` sous macOS) font de même depuis n’importe quel champ de la ligne. |
| × | Supprime la commande. |

« + Ajouter une commande » ajoute une ligne à la liste. Une commande dont la ligne de commande est vide n’est pas enregistrée.

Ces variables sont disponibles : `ESCOUADE_PROJECT_DIR` (le projet), `ESCOUADE_WORKTREE_DIR`, `ESCOUADE_BRANCH`, et les ports réservés d’un ticket (`ESCOUADE_PORT_BASE`, `ESCOUADE_PORT_END`). Un échec est signalé dans la conversation de l’agent.

### Remplir automatiquement

Le bouton « ✦ Remplir automatiquement » (« Claude lit le projet… » pendant qu’il travaille) demande à Claude de proposer les commandes. Il lit le projet (manifestes, lockfiles, README…) avec des outils de lecture seule : il ne modifie rien et ne lance rien. Ses commandes sont écrites pour le premier shell détecté.

La proposition s’affiche en entier, avec le shell et le dossier de chaque commande, dans « Commandes de worktree proposées ». Relis chaque commande en entier : une fois enregistrées, elles tournent seules, la préparation à l’ouverture de chaque nouveau worktree et le démontage avant sa suppression.

- « Remplacer les commandes » met la proposition à la place des deux listes du brouillon.
- « Ignorer » la jette.

Rien n’est enregistré avant « Enregistrer ». Un message dit combien de commandes sont proposées, et combien Claude en a écarté (caractères invisibles ou ligne trop longue). Quand il n’en trouve aucune, il dit « Claude n’a trouvé aucune commande à lancer pour ce projet. ».

## Lancement

Les commandes de lancement démarrent les serveurs, les observateurs et les services du projet. Chacune tourne dans son propre terminal, en lecture seule, lancée depuis la section Lancement de la barre latérale. L’engrenage de cette section (« Commandes de lancement… ») ouvre cet onglet sur ce groupe. Quand la section est vide, ses liens « Configurer » et « ✦ Proposer des commandes » l’ouvrent aussi, le second lançant en plus le remplissage. Voir [le lancement](launch) et [les terminaux](terminals).

Sans commande, le groupe dit « Aucune commande pour l’instant. ». Chaque commande a quatre champs.

| Champ | Effet |
|---|---|
| « Nom » | Le nom affiché dans la section Lancement, comme « Front ». Obligatoire. |
| « Shell » | Le shell qui la lance. Une nouvelle commande prend le premier shell trouvé. |
| « Commande » | La ligne de commande, comme `npm run dev`. Obligatoire. |
| « Sous-dossier » (vide = dossier du projet) | Le dossier, sous celui du projet, où elle tourne, comme `apps/web`. |

La croix à droite supprime la commande, et « + Ajouter une commande » en ajoute une. Une commande sans nom ou sans ligne de commande bloque « Enregistrer ».

« ✦ Remplir automatiquement » fonctionne comme pour les worktrees. Claude lit le projet (manifestes, scripts, docker-compose, README…) et propose les commandes à lancer dans « Commandes de lancement proposées », que « Remplacer les commandes » met à la place de la liste.

## Assistant de configuration

« Ouvrir l’assistant… » remplace les réglages par l’assistant de la fenêtre « Nouveau projet », sur ce projet (« Configurer <projet> »), pour revoir ses réglages étape par étape : « Garder la configuration actuelle » (par défaut), « Copier la configuration d’un projet », « Configuration recommandée » ou « Partir de zéro », puis les mêmes étapes et les mêmes contrôles qu’à la création (voir [Ajouter un projet](quick-start#ajouter-un-projet)). « Enregistrer », au récapitulatif, enregistre ce que l’assistant a réglé ; le reste du brouillon des réglages, s’il y en avait un, n’est pas enregistré. Ses sources liées et ses tickets restent les siens.

## Zone sensible

| Réglage | Effet |
|---|---|
| « Fermer le projet » | Le bouton « Fermer le projet… » demande confirmation, puis retire le projet de l’app : ses agents, ses terminaux et ses lancements s’arrêtent. Son historique (conversations, tickets, réglages) est gardé : ajouter un projet dans le même dossier le retrouve (voir [Projets fermés](#projets-fermes)). Les fichiers et les worktrees sur le disque ne sont pas touchés. |

La fenêtre de confirmation a une case « Effacer aussi son historique (conversations, tickets, réglages) », décochée par défaut. Cochée, rien n’est gardé : les conversations, les tickets et les réglages du projet sont effacés pour de bon, et le même dossier, ajouté de nouveau, repart de zéro.

Fermer le projet agit tout de suite, sans attendre « Enregistrer », et ramène aux réglages avec le brouillon tel qu’il était.

Fermer est refusé tant que la validation d’un ticket du projet tourne : « La validation du ticket DEM-3 est en cours : attends sa fin pour fermer le projet. » Attends qu’elle se termine, puis ferme le projet.

## Tous les projets

| Réglage | Effet | Défaut |
| --- | --- | --- |
| « Index du code sur tous les projets » | Allume l’[index du code](code-index#sur-tous-les-projets) de chaque projet qui n’a pas fait son propre choix : ils sont lus un par un, en arrière-plan. La couper efface leur index, après une confirmation qui dit combien de projets le perdent. Enregistré tout de suite, sans **Enregistrer**. | Désactivé |

## Nouveaux projets

Ce groupe vaut pour tous les projets : il est là même sans projet ouvert.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Nouveaux projets privés » | Chaque projet ajouté est privé dès le départ. Ceux qui existent gardent leur réglage, et un projet rouvert avec son historique garde le sien. | Désactivé |

## Projets fermés

Tant qu’aucun projet n’a été fermé en gardant son historique, ce groupe n’existe pas. Ensuite, « Projets fermés » liste ces projets, du plus récemment fermé au plus ancien, sous la note « Leur historique est gardé : ajouter un projet dans leur dossier le retrouve. ». Chaque ligne donne la couleur du projet, son nom, son dossier, la place que son historique prend sur le disque (ses conversations, les images et les fichiers de ses tickets, et les fichiers joints à ses agents compris) et sa dernière activité (« dernière activité le … »), avec deux boutons : « Rouvrir » et « Effacer l’historique… ».

« Rouvrir » ouvre la fenêtre « Nouveau projet » déjà sur le dossier de ce projet, lu tout de suite, avec l’historique retrouvé. C’est celui de cette ligne qui revient, même quand plusieurs projets fermés se trouvaient dans le même dossier (sans « Rouvrir », c’est le plus récemment fermé). Si tu changes le chemin dans la fenêtre, cet historique est abandonné : le nouveau dossier a le sien, ou n’en a pas. « Annuler » ferme la fenêtre, sans rien rouvrir ni effacer.

Un fichier que l’app ne sait pas lire (abîmé, ou renommé à la main) apparaît quand même dans la liste, sous « Historique illisible », avec son nom de fichier (`….json`) et sa taille. Il n’a pas de « Rouvrir » : seul « Effacer » est proposé. Après confirmation (« Ce fichier, que l’application ne peut pas lire, est effacé pour de bon. Les autres historiques, les fichiers et les worktrees sur le disque ne sont pas touchés. »), il supprime ce fichier, lui seul.

Sans passer par la liste, ajoute simplement le projet de nouveau (le bouton `+` à droite des onglets), dans le même dossier. Peu importe comment tu écris le chemin : majuscules ou minuscules (sous Windows), `/` ou `\`, séparateur final, nom court, partage WSL… c’est reconnu comme le même dossier. La fenêtre « Nouveau projet » dit alors ce qu’elle a trouvé, par exemple « Historique retrouvé : 12 agents, 34 tickets, dernière activité le 14 octobre 2026 », reprend le nom, la couleur et « Un worktree git par agent » du projet fermé, et son bouton devient « Rouvrir le projet ».

Le projet revient tel qu’il était :

- ses réglages, son Kanban et ses commandes ;
- ses agents, avec leurs conversations. Aucun process ne tourne encore : ils repartent à ton prochain message, comme après un redémarrage d’Escouade. Un agent dont le worktree a disparu entre-temps est traité de la même façon ;
- ses tickets. Un ticket qui était « En cours » à la fermeture revient bloqué, avec la raison « Projet fermé pendant le ticket » : « Reprendre » le relance ;
- ce qui attendait dans la file de synchronisation avec Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards : la file de ses tickets est gardée pendant la fermeture et repart à la réouverture (elle est oubliée si l’historique est effacé) ;
- ses [statistiques](stats), qui retrouvent son nom.

Les comptes qui lui étaient réservés (« Réservé aux projets ») n’ont pas bougé pendant la fermeture : voir [les comptes Claude](accounts#reserve-aux-projets). Le « Compte préféré » du projet repasse sur « Automatique » si son compte n’existe plus.

Rouvrir n’est pas créer un projet : la fenêtre ne propose pas de premier agent, puisque ceux du projet reviennent.

L’interrupteur « Partir d’un projet vierge » (« L’historique reste dans Réglages › Projets, où l’effacer. ») crée à la place un projet neuf dans ce dossier, sans les agents ni les tickets d’avant. L’ancien historique reste dans « Projets fermés » : rien n’est effacé sans que tu le demandes.

« Effacer l’historique… » demande confirmation (« Ses conversations, ses tickets et ses réglages sont effacés pour de bon. Les fichiers et les worktrees sur le disque ne sont pas touchés. »), puis supprime les conversations, les tickets et les réglages gardés de ce projet. Le même dossier, ajouté ensuite, n’a plus d’historique à retrouver.

> [!NOTE]
> Un projet fermé avant cette version a déjà perdu son historique : il n’y a rien à retrouver pour lui.

## Index du code

Le groupe « Index du code » allume l’[index du code](code-index) du projet (**Indexer ce projet**) : il dit ce que l’index trouve, que tout reste sur la machine, et ce qu’il contient. Tant que le projet n’a pas fait son propre choix, il suit **Index du code sur tous les projets** (voir plus bas) et le dit ; un choix fait ici est gardé, et **Suivre le réglage de l’app** y revient. Pour un projet privé, il dit ce qui reste hors de l’index. **Recherche par le sens** télécharge le modèle des vecteurs, avec sa progression et **Annuler**.

## Mémoire partagée

Le groupe « Mémoire partagée » liste ce que les agents du projet ont noté dans sa [mémoire partagée](shared-memory) : chaque souvenir, sa sorte et sa description ; un clic montre son texte. « Oublier », puis « Confirmer l’oubli », supprime son fichier de `.escouade/memory/`.

## Les autres réglages d’un projet

Les liens d’un projet vers Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues et Azure Boards se règlent dans l’onglet [« Intégrations »](settings-integrations). Son Kanban (pilote auto, agents, validation) se règle dans l’onglet [« Kanban »](settings-board).
