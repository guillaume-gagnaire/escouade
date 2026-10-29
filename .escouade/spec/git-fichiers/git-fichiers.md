---
title: "Git & fichiers"
subject: "Git & fichiers"
sources: ["docs/SPEC.md","src/lib/i18n/fr/git.ts","src-tauri/src/core.rs"]
tickets: ["ESC-204","ESC-160"]
decisions: []
compared: null
order: 22
---

## Ce que c'est

Le panneau Git montre ce que les agents ont changé : les fichiers non commités, l'historique du dépôt et les diffs. On y commite et on y merge le travail d'un agent. Chaque agent peut travailler dans un worktree à lui, sur sa propre branche : cette page dit comment ces worktrees naissent, se préparent et disparaissent, et comment le dossier du projet se synchronise avec son dépôt distant. Les branches ont leur page (voir *Branches*), comme les projets de plusieurs dépôts (voir *Projet à plusieurs dépôts*).

## Comportement

### Le panneau

- Il se ferme par son × au bout des onglets (« Fermer le panneau »). À côté de la disposition classique, il se ferme ; `Ctrl+Maj+B` ou « Fichiers » de l'en-tête le rouvrent. Côte à côte, il ramène à la disposition classique sans panneau, retenue comme la disposition ; `Ctrl+Maj+L` ou le sélecteur de disposition le rouvrent. L'infobulle dit le raccourci et comment le rouvrir.
- **« Non commités »** : portée « Cet agent » ou « Tout le projet ». Un fichier est attribué à un agent par son worktree ou, sans worktree, par ses outils d'édition (`Edit`, `Write`). Un clic ouvre le diff ; le bouton `</>` ou le clic droit ouvrent le fichier dans l'éditeur. La liste dessine les 500 premiers fichiers, puis « … et N autres fichiers » ; le compte de l'onglet, « Voir le diff » et les commits les prennent tous.
- **« Historique »** : le graphe des 300 derniers commits de tout le dépôt (branches locales et distantes, tags ; les branches des agents étiquetées de leur nom). La branche de l'agent (son worktree, sinon la branche courante) est mise en avant, le reste atténué. Un clic sur un commit ouvre son diff (contre le premier parent pour un merge). Le graphe se redessine à chaque commit, merge ou branche ; son menu contextuel agit sur les branches (voir *Branches*).
- Les compteurs git se rafraîchissent quand les fichiers changent (après un court délai), jamais par une lecture à intervalle fixe.

### Les diffs

- **« Voir le diff »** ouvre la vue diff plein écran, unifiée ou côte à côte. Ce choix est mémorisé, et partagé avec le volet de la disposition moitié / moitié et les diffs de la conversation.
- En portée « Tout le projet », la vue montre exactement les fichiers de la liste : ceux du dossier du projet et ceux du worktree de chaque agent, chacun étiqueté du nom de son agent (un même chemin peut y figurer pour deux dossiers). « Aucune différence. » seulement quand la liste est vide. Un worktree que git n'a pas pu lire fait échouer la vue avec sa raison (« Diff du worktree de « refacto-auth » non lu : … ») au lieu de taire ses fichiers. Un agent ou un worktree disparu entre-temps (validation, archivage) est laissé de côté.
- **Gros diffs** : le diff d'un fichier (suivi, non suivi ou d'un commit) de plus de 4 Mo n'est pas affiché. Le fichier reste listé avec son état et « Diff trop volumineux pour être affiché. », sans compteurs +N −M. Au-delà de 4 Mo de diff en tout, les fichiers non suivis suivants ne sont plus lus mais listés de même, les 500 premiers, puis « … et N autres fichiers ». Un fichier non suivi qui contient un octet nul est binaire, quelle que soit sa taille.
- Un nom de fichier qui contient un guillemet, une barre oblique inverse ou un caractère de contrôle est écrit dans l'en-tête du diff comme git l'écrit (entre guillemets, échappé) : il ne peut pas y ajouter de lignes à lui.
- Un diff de plus de 1 500 lignes est replié (« Diff volumineux (N lignes) », « Afficher »), puis se déplie par tranches de 500 (« Afficher 500 lignes de plus » ; la dernière tranche dit le vrai reste). Le focus reste sur le bouton, puis passe au diff. Déplié, il le reste pendant que l'agent édite le fichier.

### Commiter

« Commit… » (l'agent) et « Commit tout… » (le projet) suivent le réglage « Commit » du projet.

- **« Rédigé par l'agent »** : Escouade demande à l'agent de commiter ses changements ; il rédige le message.
- **« Direct, avec un message proposé »** : la fenêtre « Commit » s'ouvre, même sans agent.
  - Portée : « Modifications de <agent> » (son worktree, sinon ses fichiers dans le dossier du projet) ou « Modifications de <projet> » (tout le dossier du projet, sans les worktrees).
  - Les fichiers : 500 au plus, puis « … et N autres fichiers ». Les fichiers copiés laissés de côté sont nommés (« Jamais commité : .env (copié dans les worktrees). »).
  - « Message » est pré-rempli par le modèle des textes rédigés, en texte court (voir *Langues*), d'après le diff des 200 premiers fichiers (60 Ko au plus) et le style des 10 derniers sujets de commit du dépôt, en 90 s au plus. En attendant, « Haiku rédige le message… », et le champ reste éditable. « Régénérer » en redemande un ; un échec dit « Pas de proposition : <raison>. ». Le modèle ne lit le contenu d'aucun fichier qui correspond aux « Fichiers copiés dans les worktrees ».
  - « Commiter » reste désactivé tant que le message est vide : jamais de commit sans ce clic. Pendant le commit, le bouton dit « Commit… » et la fenêtre ne se ferme pas.
  - Échap, un clic à côté, × ou « Annuler » ferment tout de suite quand le message est vide ou est la proposition telle quelle. Sinon, « Abandonner le message ? » (« Le message que tu as écrit pour ce commit sera perdu. », « Abandonner ») ; son « Annuler » ramène la fenêtre avec le message, sans nouvelle proposition.
  - Seuls les chemins que la portée liste encore au moment du clic sont commités (sinon « Plus rien à commiter : ces fichiers n'ont plus de modification. » ; message vide : « Écris le message du commit. »). Les hooks de git tournent ; un refus de git (un hook) s'affiche dans la fenêtre et laisse l'index tel qu'il était. Ce que l'utilisateur avait indexé à part reste indexé et hors du commit (`git commit --only`). Réussi : toast « Commit <hash> créé ».
  - « Commit tout… » ne commite pas les worktrees. Quand seuls eux ont des changements : « Aucune modification dans le dossier du projet : le worktree d'un agent se commite depuis « Cet agent ». ».
  - Une mise à jour automatique d'Escouade attend la fin d'une proposition ou d'un commit.
- Les fichiers copiés dans les worktrees ne sont jamais commités : ceux qui correspondent au réglage et que git ignore (même forcés dans l'index) et, dans un worktree, ceux que la copie y a mis (même si l'agent ne les ignore plus).

### Les worktrees des agents

- Un agent en worktree travaille dans `<dossier des worktrees>/<agent>` (`<projet>/.claude/worktrees/<agent>` par défaut), sur la branche `<préfixe><agent>` (`escouade/<agent>` par défaut). Les fichiers du projet que git ignore et qui correspondent aux « Fichiers copiés dans les worktrees » y sont copiés. Un fichier seulement non suivi ne l'est jamais : il apparaîtrait comme une modification à commiter.
- Le dossier des worktrees reçoit les worktrees des nouveaux agents, des tickets, d'une copie d'agent et d'un agent sur une branche existante, et le worktree temporaire d'une validation (`.merge-<projet>-<clé>`, sinon à la racine du dépôt ; un reste d'une validation coupée, `.merge-…`, est reconnu où qu'il soit).
- Un nom de dossier déjà pris (par un autre dépôt dans un dossier partagé, ou par un agent dont le worktree a disparu pour l'instant) passe au suivant (`-2`…). La branche d'un agent suit ce nom ; celle d'un ticket garde le sien (celui de sa PR) tant qu'elle est libre.
- Les worktrees déjà créés restent où ils sont quand le réglage change. Un agent restauré retrouve le sien à son ancien chemin ; si un autre worktree a pris ce dossier, le sien est recréé à côté (`-2`…) et sa conversation le dit (« Le dossier <ancien> est pris par un autre worktree : celui de l'agent est recréé dans <nouveau>. »).
- Un dossier des worktrees dans le dépôt est ajouté à `.git/info/exclude` depuis la racine du dépôt (`/.worktrees/`, ou `packages/web/.claude/worktrees/` pour un projet dans un sous-dossier). Un dossier hors du dépôt n'y touche pas.
- Un dossier des worktrees hors de celui du projet est surveillé lui aussi : les compteurs git des agents s'y rafraîchissent de même. La vue Claude Code demande alors d'abord la confiance du dossier de chaque worktree (voir `docs/PROTOCOL.md`).
- Un dossier des worktrees qui contient des fichiers suivis par git (`src`) est refusé à la création du worktree : les nouveaux fichiers du projet y seraient laissés hors des commits (« Le dossier des worktrees src contient des fichiers suivis par git : choisis-en un autre dans les réglages du projet. »).

### La branche de départ

- Chaque nouvel agent qui reçoit un worktree part de la « Branche de départ des nouveaux worktrees » du projet, qu'il soit créé tout de suite ou au premier message : « Nouvel agent » dans un projet à un worktree par agent, « Nouvel agent dans un worktree », la branche nouvelle de « Nouvel agent sur une branche… », l'outil MCP `create_agent` avec un worktree. Cette branche est aussi sa base : « Merger… », « Intégrer <base> », le diff et sa PR, ouverte vers elle.
- Aucun fetch à la création : l'état distant est celui du dernier fetch. Si la branche suivie (`origin/<branche>`) est en avance et que la locale n'est qu'en retard, la locale avance d'abord en fast-forward, sans toucher à aucun dossier. Sinon l'agent part de la locale telle qu'elle est, et sa conversation l'avertit : la branche est extraite dans un dossier (« origin/main a 2 commits que « main » n’a pas, mais elle n’a pas pu avancer (« main » est extraite dans <dossier>) : l’agent part de la « main » locale, sans eux. ») ou a des commits non poussés (« « main » a des commits non poussés et origin/main 2 commits qu’elle n’a pas : … »).
- Une branche distante seule (`origin/x`, sans locale qui la suive) : la locale `x` qui la suit est créée, sans fetch, et l'agent part d'elle, qui devient sa base (« Merger… », « Intégrer » et la PR ont besoin d'une base locale).
- Une branche cible absente en local : l'agent part de la branche du dossier (« La branche cible « x » n’existe pas : l’agent part de la branche du dossier du projet. »). De même pour une branche nommée disparue, ni locale ni distante (« La branche de départ « x » n’existe plus : l’agent part de la branche du dossier du projet. »).
- Jamais un agent parti du distant avec une base locale en retard : un agent sans commit n'aurait rien à merger.
- Sans effet sur les agents sans worktree, les copies, les agents sur une branche existante et les tickets, qui partent de la branche cible (voir *Agent d'un ticket*).
- **Pour un seul agent** : « Nouvel agent depuis une autre branche… » (sous « Worktree », dans le menu « Harness » à côté de « Nouvel agent ») ouvre le sélecteur de branche en mode « Branche de départ » et crée l'agent dans un worktree qui part de la branche choisie, même dans un projet sans worktree par agent. Dans « Nouvel agent sur une branche… », un nom tapé qu'aucune branche n'a offre « Créer la branche « x » et y lancer un agent » (qui dit d'où elle part : la branche de départ du projet, ou celle du dossier quand la sienne manque), puis « Créer « x » depuis une autre branche… », qui ouvre le même sélecteur (« Branche de départ de « x » »). Fermer le sélecteur ne crée rien. Avec le worktree au premier message, la branche choisie est gardée avec l'agent, même après un redémarrage, jusqu'à la création de son worktree. Elle suit les mêmes règles que celle du projet et devient la base de l'agent ; le réglage du projet ne change pas.

### Un agent sur une branche existante

« Nouvel agent sur une branche… » (à côté de « Nouvel agent »), ou « Lancer un agent sur cette branche » au clic droit dans le sélecteur de branche : l'agent a un worktree sur la branche choisie. Sa base est la branche cible du Kanban si elle existe, sinon la branche du dossier. Une branche distante (`origin/x`) se prend par la locale qui la suit, créée au besoin. Refusé :

- pour la branche du dossier du projet, grisée dans le sélecteur (« C’est la branche du dossier du projet : un agent sans worktree y travaille déjà, ou change de branche d’abord. ») ;
- pour la branche cible du Kanban, où les tickets validés sont mergés : ni un agent ni un ticket ne la prend ;
- pour la branche d'un autre worktree (« La branche « X » est utilisée par l’agent N, dans son worktree. »).

La branche est celle de l'utilisateur : supprimer l'agent avec « Supprimer aussi le worktree (la branche X est conservée) » ne supprime que le dossier, et elle n'est jamais renommée d'après le nom de l'agent.

### Merger le travail d'un agent

« Merger <branche> → <base>… » demande « Merger <branche> dans <base> ? », avec l'option « Squash (un seul commit) ». Les commits comptés depuis la base de l'agent y sont mergés.

- Si le dossier du projet est sur une autre branche (ou en HEAD détachée), une seconde confirmation propose « Basculer sur « <base> » ? » (« Le projet est sur la branche « <courante> ». Escouade bascule sur « <base> » puis merge « <branche> ». »), bouton « Basculer et merger ». La bascule ne crée jamais une branche depuis un dépôt distant.
- Un refus de git (fichiers qui gêneraient, base prise par un autre worktree) s'affiche tel quel, et rien n'est mergé.
- Une base disparue est refusée (« La branche de base « X » n'existe plus : « Y » ne peut pas être mergée dedans. »).

### Intégrer la base dans la branche d'un agent

« Intégrer <base> » (sous « Merger… » dans la liste des fichiers, et dans le menu de l'agent) fait entrer la base de l'agent dans sa branche, dans son worktree : par merge avec les stratégies « Merge commit » et « Squash » (il n'y a rien à écraser), par rebase avec « Rebase ».

- Refusé pendant son tour (« Attends la fin du tour de <nom> pour intégrer <base>. »), avec des changements de fichiers suivis (« Commite ou mets de côté les changements de l’agent d’abord. »), avec un merge ou un rebase déjà en cours dans son worktree, pour un agent archivé ou sans worktree, pendant la validation de son ticket, et quand la base a disparu.
- Sans conflit : toast « <base> intégrée dans <branche>. », ou « <branche> a déjà tout ce qu’il y a dans <base>. ».
- Un conflit de merge reste dans le worktree, et l'agent reçoit la demande de le résoudre, avec les fichiers (hors ticket : « Le merge de <base> dans ta branche a des conflits sur : … Résous-les, puis commite le merge. »). Un conflit de rebase est annulé, et l'agent reçoit « Rebase ta branche sur <base> et résous ses conflits. ».

### Préparer et démonter un worktree

- **Préparation** : les commandes de préparation du projet tournent sans fenêtre, l'une après l'autre, dans chaque nouveau worktree (agent créé par l'utilisateur ou par un ticket, worktree recréé à la restauration), avec `ESCOUADE_PROJECT_DIR`, `ESCOUADE_WORKTREE_DIR`, `ESCOUADE_BRANCH` (et les ports réservés d'un ticket) ; 20 min au plus chacune.
- Pendant ce temps, l'agent montre « Préparation… » dans la barre latérale, « Préparation du worktree · 1/2 · npm ci » avec « Voir la sortie » dans la conversation (la sortie en direct de l'étape, voir *Conversation*), et « Prépare le worktree · … » sur la carte du Kanban. Son harness ne démarre qu'une fois la préparation finie (ses hooks et serveurs MCP la trouveraient à moitié faite) ; ses messages attendent.
- La sortie complète de chaque étape va dans `logs/<agent>-setup.log` du dossier de données : réécrit à chaque préparation, avec un en-tête « ── 1/2 · npm ci » puis « ── terminée en N s » ou « ── échec : … » par étape. Il est gardé à l'archivage et quand le projet se ferme avec son historique, et supprimé avec l'agent, ou avec le projet quand son historique est effacé.
- La première commande qui échoue arrête la préparation : « La préparation du worktree a échoué sur `<commande>` (code N). », ses dernières lignes dans la conversation, puis « Sortie complète : <chemin du log> » quand il a pu être écrit ; pour un ticket, à la suite de son premier message. Une sortie qui n'a pas pu être lue jusqu'au bout fait échouer l'étape (« la sortie de la commande n'a pas pu être lue : … »). Réussie : « Worktree préparé (n commandes, x s). ».
- Supprimer ou archiver l'agent, fermer le projet ou quitter l'app arrête la préparation et ce qu'elle a lancé. Un agent archivé ou supprimé pendant sa préparation ne prend jamais le ticket qui l'attendait.
- **Démontage** : les commandes de démontage tournent dans le worktree juste avant que l'app le supprime (agent supprimé avec son worktree, ticket terminé ou supprimé, « Nettoyer »), toutes, même après un échec, 5 min au plus chacune. Un échec est signalé (« Agent supprimé, mais le démontage du worktree a échoué sur … ») sans empêcher la suppression.
- **« ✦ Remplir automatiquement »** : le modèle des textes rédigés lit le projet, sans rien modifier ni exécuter (voir *Langues*), et propose les deux listes : installer les dépendances avec le gestionnaire du lockfile, générer le code ; le démontage est souvent vide. Une commande dont le dossier sort du projet est écartée, comme une commande qui ne tient pas sur une ligne simple : mêmes règles et mêmes messages que les commandes de lancement (voir *Barre latérale*), puisqu'elles tournent seules dans chaque nouveau worktree. La proposition s'affiche d'abord en entier sous la rangée, dans « Commandes de worktree proposées » : « Proposition de Claude : relis chaque commande en entier. Elles remplacent les deux listes et, une fois enregistrées, tournent seules : la préparation à l'ouverture de chaque nouveau worktree, le démontage avant sa suppression. », chaque commande lisible jusqu'au bout avec son shell et son dossier, « Aucune commande. » pour une liste vide. « Remplacer les commandes » met les deux listes à la place du brouillon ; « Ignorer » le laisse tel quel.

### Supprimer, nettoyer, restaurer

- Supprimer un agent supprime son worktree et sa branche ; la branche d'un agent sur une branche existante est conservée.
- « Nettoyer » (Réglages › Projets, « Worktrees restés ») supprime les worktrees des agents archivés du projet encore sur le disque, avec la même prudence que ceux des tickets finis (voir *Validation*), puis fait `git worktree prune` pour ceux dont le dossier a disparu. La rangée dit ensuite combien sont supprimés (« Aucun worktree à supprimer. ») et ceux gardés, avec pourquoi.
- Restaurer un agent archivé dont le worktree a disparu (supprimé à la fin de son ticket, ou par « Nettoyer ») le recrée depuis sa branche, avec les fichiers copiés et la préparation. Sans sa branche (mergée, puis supprimée avec lui), il reste tel quel et sa conversation le dit (« Son worktree a été supprimé avec sa branche … (mergée) : il n'est pas recréé. »). Il n'y a pas de délai de grâce : la branche gardée suffit à le recréer.

### Synchroniser avec le dépôt distant

- Le dossier du projet compte ses commits à tirer et à pousser par rapport à sa branche suivie ; la barre d'état les montre (voir *Barre de statut*).
- Un fetch tourne en arrière-plan peu après le démarrage, puis toutes les 5 min, un dépôt à la fois, sans jamais demander d'identifiants (ni fenêtre de Git Credential Manager). Ses échecs sont seulement journalisés.
- Depuis le sélecteur de branche (voir *Branches*) : « Récupérer » (pull, en avance rapide seulement ; des branches divergées donnent un message clair), « Pousser » (une branche sans branche suivie, ou dont la distante a été supprimée, est publiée sur `origin`, ou sur l'unique dépôt distant) et « Fetch » à la demande.

## Réglages

Réglages › Projets, groupe « Git ». Un dossier ou un préfixe refusé l'est à la frappe comme à l'enregistrement.

- **« Commit »** : « Rédigé par l'agent » (défaut) ou « Direct, avec un message proposé ».
- **Un worktree par agent**, et **« Créer le worktree au premier message »**.
- **« Branche de départ des nouveaux worktrees »** (sous « Créer le worktree au premier message » ; aussi à l'étape « Worktrees » de l'assistant « Nouveau projet », avec un worktree par agent ; partagée par l'équipe dans `.escouade.yml`, voir *.escouade.yml*) : trois segments, « Branche actuelle du dossier » (défaut : la HEAD du dossier au moment de la création), « Branche cible du Kanban » et « Une branche… ». Ce dernier ouvre le sélecteur de branche en mode « Branche de départ » : toutes les branches, locales et distantes, la branche du dossier et la cible comprises, celle choisie cochée ; dans l'assistant, pour un dossier pas encore ajouté, ses branches locales. Le segment montre ensuite « ⎇ <branche> ▾ », et le texte dessous dit ce que fait le choix. Un projet enregistré avec l'ancien interrupteur « Les agents partent de la branche cible » (onglet « Kanban ») allumé passe à « Branche cible du Kanban » à son chargement, ou à sa réouverture s'il était fermé ; l'interrupteur n'est plus écrit.
- **« Dossier des worktrees »** : vide par défaut, soit `.claude/worktrees`, comme `claude --worktree`. Il prend un chemin relatif au dossier du projet (`../app-worktrees`, un dossier voisin du dépôt ; `.worktrees`), au dossier personnel (`~/worktrees/app`) ou complet. `.` et `..` sont résolus tels qu'écrits ; des guillemets autour (« Copier en tant que chemin ») sont retirés. Sous le champ : « Le prochain : <chemin>\<nom> ». Refusés : le dossier du projet lui-même (« C’est le dossier du projet lui-même. »), un dossier qui le contient (`..`, `../..` : « Ce dossier contient celui du projet : choisis-en un à côté (../app-worktrees) ou dedans (.worktrees). »), un dossier dans un `.git` (« Ce dossier est dans un dossier .git. ») et, sous Windows, un chemin ni relatif ni complet (`D:wt`, `\wt` : « Écris un chemin relatif au dossier du projet, ou un chemin complet (C:\…). »).
- **« Préfixe des branches des agents »** (`escouade/` par défaut) et **« Préfixe des branches des tickets »** (`ticket/` par défaut), chacun avec un exemple dessous (« Branche d’un nouvel agent : escouade/<nom>. Vide : son nom seul. »). Vide, la branche est le nom de l'agent ou la clé du ticket seuls. Un préfixe dont aucun nom de branche ne peut partir est refusé : espace, `..`, `~`, `^`, `:`, `?`, `*`, `[`, `\`, `@{`, `-` ou `/` au début, `//`, une partie qui commence par `.` ou finit par `.lock`. Un agent renommé d'après son premier message garde le préfixe de son projet, une copie aussi. Les textes du Kanban (« push ticket/* », « Pousse ticket/<clé>… ») et le champ « Branche » d'un ticket (« Nouvelle branche ticket/<clé> ») suivent le préfixe des tickets. La barre latérale montre la branche d'un agent sans le préfixe de son projet.
- **« Fichiers copiés dans les worktrees »** : `.env*` par défaut.
- **Commandes de préparation et de démontage** : pour chacune, shell, ligne de commande et sous-dossier du worktree ; réordonnables par « Monter » / « Descendre », ou `Alt+↑` / `Alt+↓` depuis l'un de leurs champs ; « ✦ Remplir automatiquement ».
- **« Worktrees restés »** › « Nettoyer ».

## Limites

- Un commit direct n'est pas refusé pendant que l'agent travaille.
- Dans le dossier du projet, un `.env` qu'un agent sans worktree a sorti du `.gitignore` est listé, et peut être commité si on le laisse ; le modèle ne le lit pas.
- « Commit » direct demande git 2.26 au moins.
- Le pull se fait en avance rapide seulement : des branches divergées se rebasent ou se mergent à la main.
- Rien à synchroniser en HEAD détachée ni sans dépôt distant.
- La fenêtre « Commit » dit « Haiku rédige le message… » quel que soit le harness des textes rédigés ; la fenêtre d'une pull request (« Rédaction du titre et de la description… ») et le commit d'un projet à plusieurs dépôts (« Message en cours de rédaction… ») ne nomment pas de modèle.
