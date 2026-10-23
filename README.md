# Escouade

Un environnement de développement agentique (ADE) : une application Windows, macOS et Linux où tu développes avec des agents de bout en bout. Tu planifies dans un Kanban de tickets, des agents codent en parallèle, chacun dans son worktree, tu relis et testes leur travail, tu le livres (commit, merge, pull request) et tu pilotes l'ensemble : notifications, quotas, coût, statistiques, serveur MCP. Les agents tournent sur le harness de ton choix : [Claude Code](https://claude.com/claude-code), [Codex](https://github.com/openai/codex), [OpenCode](https://opencode.ai) ou [GitHub Copilot CLI](https://github.com/github/copilot-cli) (Cursor CLI viendra plus tard). Gratuit et open source, en français et en anglais.

Site : [escouade.dev](https://escouade.dev/) · Documentation : [escouade.dev/docs](https://escouade.dev/docs/) · Discord : [discord.gg/m3KYtN8gxP](https://discord.gg/m3KYtN8gxP)

Escouade s'appelait auparavant « Claude Code Manager » : la mise à jour remplace l'ancienne installation et garde tes réglages et tes conversations. Projet indépendant, non affilié à Anthropic, OpenAI ni GitHub ; Claude et Claude Code sont des marques d'Anthropic, Codex d'OpenAI, GitHub Copilot de GitHub.

## Fonctionnalités

Ce qui change d'une version à l'autre est dans le [journal des versions](CHANGELOG.md).

### Planifier

- **Kanban** : des tickets avec leurs critères d'acceptation, que des agents prennent seuls (pilote auto, de 1 à 6 en parallèle), chacun dans son worktree, en bouclant jusqu'à les atteindre ; un ticket peut en attendre d'autres, reprendre une branche existante, s'écrire dans une grande fenêtre (`Ctrl+Entrée` pour l'ajouter), et le pilote auto se met en pause près de la limite de tes quotas. « ▶ Tester » lance la fonctionnalité sur des ports à part, après t'avoir montré la recette écrite par l'agent ; la validation commite, merge, ouvre une pull request ou pousse.
- **Intégrations** : importe tes tickets Jira, Trello, GitHub Issues ou Linear dans le Kanban, avec leurs critères ; leur statut et des commentaires suivent, et une synchro ratée est retentée. Les jetons restent dans le trousseau du système.
- **Images et fichiers des tickets** : une capture ou une maquette se colle (`Ctrl+V`) ou se glisse dans la fenêtre d'un ticket, et « Joindre un fichier » y ajoute n'importe quel fichier (20 au plus) ; l'agent du ticket les reçoit avec son premier message, et la carte du Kanban montre un trombone et leur nombre.

### Coder

- **Projets en onglets** : compteur de modifications git non commitées, pastille quand un agent attend une réponse, couleur par projet (qui teinte toute l'interface), réordonnables par glisser-déposer.
- **Quatre harness** : chaque agent tourne sur Claude Code, Codex, OpenCode ou Copilot CLI, dans la même conversation, avec les mêmes cartes, le même Kanban et les mêmes worktrees. Un « Harness par défaut » par projet, un autre pour un agent (« Nouvel agent ▾ ») ou un ticket, et le pilote auto qui suit, jusqu'à continuer sur un autre harness quand les quotas du sien sont épuisés. Ce que chacun sait faire et ce qui le protège : [Installer un harness](#installer-un-harness) et la page [Harness](https://escouade.dev/docs/harness/) du site.
- **Agents** : une conversation par agent, nommée automatiquement d'après la première demande. Modèle (pour Claude Code : Fable, Opus, Sonnet, Haiku, toujours le plus récent qu'il connaît, affiché avec sa version : « Sonnet 5.5 » ; pour les autres, ceux que le harness liste), effort (jusqu'à `max`) et mode de permission (Auto, Demander, Plan, Édits auto, Bypass) modifiables en cours de conversation. Archivage, suppression, renommage. « Dupliquer la conversation » crée une copie de l'agent qui reprend tout son contexte dans une nouvelle session (et son propre worktree), sans toucher à l'original.
- **Chat natif** : markdown avec coloration syntaxique, réflexion repliable, appels d'outils compacts et dépliables (diffs, sorties de commandes, sous-agents). Questions de Claude et demandes d'autorisation sous forme de cartes cliquables (ou réponse libre), auxquelles tu réponds aussi au clavier (`Ctrl+Entrée`, `Alt+1` … `Alt+9`). Images (coller/glisser), autocomplétion `@fichier` et `/commande`, messages envoyés pendant que Claude travaille (il en tient compte dès sa prochaine étape, comme dans Claude Code), interruption par `Échap`. Le texte que tu commences est gardé d'un redémarrage à l'autre, chaque agent se rouvre là où tu l'avais laissé, et une longue conversation n'affiche que ses 80 derniers éléments (« Afficher les N précédents » pour remonter).
- **Vue Claude Code** : un bouton de l'en-tête fait tourner l'agent dans l'interface de Claude Code elle-même, dans un terminal, avec ses commandes, ses raccourcis et ses dialogues, et y revient dans la conversation d'Escouade sans perdre la session. Escouade le suit toujours (notifications quand il attend une autorisation ou termine, coût estimé, statistiques, bandeau « Plan », recherche) par ses hooks et son transcript. Un réglage y ouvre les nouveaux agents ; les agents de ticket restent dans la vue d'Escouade. Les autres harness ont la leur : « Vue Codex », « Vue Copilot CLI » (même principe) et « Vue OpenCode », qui branche l'interface d'OpenCode sur la session de l'agent sans arrêter le suivi d'Escouade.
- **Avancée d'un plan** : quand un agent exécute un plan, un bandeau « Plan » entre son en-tête et ses messages dit où il en est sans lire la conversation : tâches faites, sous-agents qui travaillent et ce que fait chacun, tâche bloquée en attente de ta réponse. La liste vient des outils de tâches de Claude Code ou d'un plan superpowers, lu en lecture seule ; une mini-barre « Plan 2/7 » le résume sur la carte de l'agent.
- **Sous-agents et tâches de fond** : sous la carte d'un agent, ses sous-agents et ses tâches de fond (commandes en arrière-plan, surveillances, workflows). Un clic ouvre la conversation d'un sous-agent ou la sortie d'une commande, suivie en direct ; « Arrêter » en arrête un seul, et « Lui écrire » fait passer un message à un sous-agent en arrière-plan.
- **Fichiers joints** : n'importe quel fichier se joint à un message (trombone, collage, glisser-déposer). Ce que le harness lit (image, texte, PDF pour Claude Code) part dans le message, le reste par son chemin ; sous Windows, un fichier déposé est lu là où il est, sans copie.
- **Rechercher dans les conversations** (`Ctrl+K`) : dans ce que toi et tes agents avez écrit, les commandes, les fichiers et les sorties d'outils, sans tenir compte de la casse ni des accents ; un résultat ouvre l'agent sur le message.
- **Éditeur intégré** : parcourir et modifier les fichiers du projet ou du worktree d'un agent sans quitter l'app (arborescence, onglets, coloration syntaxique, recherche, lignes modifiées marquées dans la marge, `Ctrl+S` pour enregistrer). Il s'ouvre depuis l'en-tête d'un agent, les fichiers non commités ou les fichiers édités dans la conversation (une modification s'ouvre à sa première ligne modifiée), et prévient quand un fichier ouvert change sur le disque pendant que l'agent travaille, avec « Comparer » pour voir ce que le disque a de différent avant de choisir. Et aussi :
  - `Ctrl+clic` ou `F12` suit un import, un lien ou un chemin, et va à la définition d'un nom, sans serveur de langage ; `Alt+←` revient ;
  - `Ctrl+P` ouvre un fichier par son nom, `Ctrl+Shift+F` cherche dans tous les fichiers, `Shift+F12` trouve les références d'un nom ;
  - « Voir les changements » montre dans le texte ce qui a changé depuis le dernier commit, et « Annuler ce bloc » le remet ;
  - au clic droit dans l'arborescence : renommer (`F2`), supprimer vers la corbeille (`Suppr`), créer un dossier, ouvrir un terminal ici ; les fichiers `.env*` copiés dans les worktrees y apparaissent en grisé.
- **Terminaux** : vrais terminaux (ConPTY ou pty + xterm.js) PowerShell 7, Git Bash et WSL sous Windows, zsh, bash et fish sous macOS, avec l'autocomplétion native du shell. Un terminal s'ouvre aussi dans le worktree d'un agent ou dans un dossier de l'éditeur.

### Relire et tester

- **Git** : panneau des fichiers non commités (par agent ou pour tout le projet, worktrees compris), visionneuse de diff (unifié / côte à côte, un choix qui s'applique aussi aux diffs de la conversation ; un long diff se déplie par tranches), commit rédigé par l'agent lui-même ou commit direct avec un message proposé par Haiku, que tu relis avant de cliquer, **git graph** de tout le dépôt avec la branche de l'agent mise en avant (onglet « Historique », clic sur un commit pour son diff, clic droit pour créer une branche, y passer ou la comparer). Disposition **moitié / moitié** au choix : la conversation à gauche, les fichiers modifiés et leur diff à droite, rafraîchis pendant que l'agent travaille. Option **un worktree par agent**, avec les fichiers `.env*` du projet copiés et des commandes de préparation (installer les dépendances…) dont tu suis la sortie en direct, puis merge (ou squash) dans la branche de base de l'agent et nettoyage.
- **Branches** : le bouton de branche de la barre d'état ouvre un sélecteur (recherche, locales et distantes, avance et retard « ↑2 ↓5 », récupérer, pousser) pour changer de branche sans perdre de travail (stash proposé), en créer, supprimer celles qui sont mergées ou, après confirmation, les autres, et nettoyer d'un coup les « Branches mergées ». Un agent ou un ticket peut travailler sur une branche qui existe déjà, et « Intégrer <base> » fait entrer la base dans la branche d'un agent en lui rendant les conflits à résoudre.
- **Lancement du projet** : par projet, une liste de commandes (nom, ligne de commande, shell, sous-dossier) à lancer une par une ou toutes ensemble. Chacune tourne dans son propre terminal, en lecture seule, et garde son log d'un lancement à l'autre. Statut en direct (en cours, arrêté, terminé, planté avec le code de sortie et une notification) ; lancer, relancer, stopper. « ✦ Remplir automatiquement » laisse Claude lire le projet (sans rien modifier) et proposer ces commandes, que tu lis en entier avant de les prendre.

### Livrer

- **Livrer** : un ticket validé passe d'abord les tests du Kanban s'ils sont réglés, puis il est commité et part comme le règle le projet : merge dans la branche cible (merge commit, squash ou rebase), push et pull request (par `gh`, avec la description, l'avancement et les critères ; sinon la page GitHub de la pull request s'ouvre pré-remplie), push de la branche, ou branche gardée. Un conflit de fusion se confie à l'agent, le worktree est supprimé une fois validé si tu le règles, et les fichiers que git ignore (`.env*`) ne sont jamais commités, poussés ni mergés.

### Piloter

- **Notifications** : l'onglet du projet et la carte de l'agent clignotent (ambre : question, vert : terminé, rouge : erreur) jusqu'à ce que tu affiches l'agent ; carillon, notification système (Windows : cliquable), clignotement de la barre des tâches (macOS : rebond de l'icône du Dock) et badge dans la zone de notification / la barre des menus quand un agent pose une question ou termine. La notification dit de quoi il s'agit (« Autoriser Bash : npm test ? », la question, la première ligne de la réponse), et « Me prévenir pour » choisit les types qui sonnent.
- **Vue d'ensemble** (`Ctrl+Shift+A`) : tous les agents de tous les projets sur une page, ceux qui attendent ta réponse en tête ; une autorisation qui s'y lit en entier s'accorde ou se refuse sur place.
- **Barre de statut** : agents actifs, en attente et terminés, quotas du compte en cours en compact (`5h … reset 3h01 | 7j … reset 4j 12h`, le pourcentage au survol de la barre ; avec plusieurs comptes, un clic ouvre le détail de chacun), coût du jour. Tokens et coût montent en direct pendant que Claude travaille (estimation « ≈ » d'après les tarifs publics), puis prennent le chiffre exact du harness à la fin du tour (Codex ne donnant pas le sien, son coût reste « ≈ … estimé »).
- **Statistiques** : tokens (entrée, cache, sortie) par jour, semaine ou mois, coût global, coût moyen par prompt, répartition par projet, par modèle, par agent, par ticket et par compte.
- **Plusieurs comptes** : pour Claude Code, Codex et Copilot CLI, un compte par dossier de configuration (perso, pro…), chacun connecté depuis Escouade (« Ajouter un compte… » partage, par lien ou par copie, tes réglages, skills, agents et plugins, jamais la connexion). Chaque agent reste sur son compte ; un nouvel agent ou un ticket part sur le premier compte sous le seuil de « Pause au-delà du quota » (ou le « Compte préféré » du projet), et le pilote auto ne se met en pause que quand tous sont au seuil. Un agent arrêté par la limite d'usage peut reprendre, avec sa conversation, sur un autre compte (« Reprendre sur Pro », ou tout seul pour un agent de ticket si tu l'actives). Les statistiques se filtrent par compte. Un compte, Principal compris, peut être « Réservé aux projets » que tu coches (ceux d'un client, par exemple) : les autres projets n'y partent jamais, ni en automatique, ni au choix, ni en reprise ; un projet que plus aucun compte actif ne sert ne démarre rien, et les réglages le disent. Ton compte habituel devient « Principal » et se lance comme avant. Chaque harness a ses comptes et son quota : fenêtres de 5 h et 7 jours pour Claude Code et Codex, crédits du mois pour Copilot CLI, et pour OpenCode, qui n'a pas de quota, le coût du jour avec un « Budget par jour ($) ».
- **Serveur MCP** : Claude Code, Codex, OpenCode ou Copilot CLI (dans un terminal, leur app, ou un agent d'Escouade) lit tes projets, agents et tickets, crée, modifie, déplace et lance des tickets (avec leurs images et leurs fichiers, qu'il joint, liste et lit), mène leur flux jusqu'au bout (valider, renvoyer, reprendre, supprimer, attendre un changement), règle le Kanban d'un projet, crée un agent, lui écrit et l'arrête, lit ses sous-agents et ses tâches de fond (état, sortie, rapport), les arrête et écrit à un sous-agent, voit ses questions et ses demandes de permission et y répond, sous les mêmes garde-fous que le pilote auto. « Les agents peuvent piloter Escouade » (Réglages › Harness) le déclare dans chaque compte de chaque harness installé (le jeton écrit en clair dans sa config) et le donne à chaque agent d'Escouade, avec un jeton à lui ; il n'écoute que sur ta machine, derrière un jeton ; un journal « Activité MCP » garde les appels. Pour un ticket, il fait ce que font les boutons du Kanban, valider (les tests, puis la fusion) et supprimer compris ; aucun outil n'accorde un « Toujours autoriser » (une règle durable reste ta décision, dans la fenêtre), ne lance une commande de son choix ni ne supprime un projet ou un agent.
- **Remote control** : clic droit sur un agent → « Activer le remote control » (Claude Code). Sa session devient accessible depuis claude.ai et l'app Claude sur mobile ; ce que tu y envoies s'affiche aussi dans l'app, et l'agent reste joignable tant que l'app tourne (même session après un redémarrage).

### Et aussi

- **Français et anglais** : l'interface existe dans les deux langues, celle du système par défaut, au choix dans Réglages › Application, sans redémarrer ; nombres, montants, dates et pluriels suivent la langue. Une langue à part pour les textes que Claude rédige (messages de commit, descriptions de pull request, commentaires publiés dans Jira, Trello, GitHub et Linear, consignes aux agents), car beaucoup d'équipes commitent en anglais avec une interface en français. Le site existe aussi en deux langues.
- **Affichage** : « Zoom de l'interface » de 80 à 200 % (`Ctrl+=`, `Ctrl+-`, `Ctrl+0`), sans redémarrer, et la taille du texte des terminaux et de l'éditeur à part.
- **Fermer un projet garde son historique** : l'ajouter de nouveau dans le même dossier retrouve ses agents, leurs conversations, ses tickets et ses réglages ; Réglages › Projets › « Projets fermés » les liste, pour les rouvrir ou les effacer.
- **Fermer la fenêtre ne coupe pas les agents** : l'app reste dans la zone de notification. Au redémarrage, chaque agent reprend sa session. Les processus inactifs sont arrêtés après un délai réglable et reprennent automatiquement à la prochaine action.
- **Proxy réseau** configurable (processus des harness, quotas, intégrations, mises à jour, et optionnellement terminaux).
- **Mises à jour silencieuses** via les releases GitHub : téléchargées en arrière-plan, installées sans fenêtre d'installeur, au redémarrage que tu choisis, ou d'elles-mêmes quand aucun agent ne travaille et que tout est enregistré.

## Prérequis

- Windows 10 ou 11 (WebView2, présent par défaut sur Windows 11), macOS 11 (Big Sur) ou plus récent, sur Mac Apple Silicon ou Intel, ou Linux x86-64 avec une glibc 2.35 ou plus récente (Ubuntu 22.04, Debian 12, Fedora 36 et suivantes), WebKitGTK 4.1 compris.
- Au moins un harness installé et connecté : Claude Code, Codex, OpenCode ou Copilot CLI (voir [Installer un harness](#installer-un-harness)).
- Git : Git for Windows, sous macOS celui des outils en ligne de commande Xcode (`xcode-select --install`) ou de Homebrew, sous Linux celui de la distribution.
- Optionnel sous Windows : PowerShell 7 (à défaut, les terminaux utilisent Windows PowerShell), WSL.

Sous Windows, Claude Code peut aussi être installé dans WSL. Donne son chemin dans Réglages › Harness, sous l'une de ces formes : `~/.local/bin/claude`, `/home/<utilisateur>/.local/bin/claude` ou `\\wsl.localhost\<distribution>\home\<utilisateur>\.local\bin\claude`. Champ vide, il est trouvé seul quand Windows n'a pas de `claude`. La ligne sous le champ dit où Escouade l'a trouvé. Escouade le lance par `wsl.exe` dans le dossier de l'agent (`/mnt/c/…`) et lit ses sessions et sa connexion dans le dossier personnel Linux. Les outils MCP d'Escouade et le suivi en direct de la vue Claude Code demandent le réseau miroir de WSL : `networkingMode=mirrored` sous `[wsl2]` dans `%UserProfile%\.wslconfig`, puis `wsl --shutdown`. Claude Code y est lancé sans le profil du shell : une variable posée dans `~/.profile` ou `~/.bashrc` ne lui parvient pas.

Sous macOS, une app lancée depuis le Finder ou le Dock ne reçoit pas le `PATH` du terminal : Escouade reprend celui de ton shell de connexion (`$SHELL -ilc`) au démarrage, et cherche aussi `claude` dans `~/.local/bin`, `/opt/homebrew/bin` et `/usr/local/bin`.

## Installer un harness

Escouade lance le harness de ta machine, avec ton propre compte : installe-en au moins un et connecte-le une fois, comme pour t'en servir dans un terminal. Réglages › Harness dit ensuite pour chacun son exécutable, sa version et son état, et « Se connecter… » ouvre sa connexion dans un terminal intégré.

| Harness | Installation | Connexion | Testé avec |
|---|---|---|---|
| [Claude Code](https://code.claude.com/docs/fr/overview) | `irm https://claude.ai/install.ps1 \| iex` (Windows), `curl -fsSL https://claude.ai/install.sh \| bash` (macOS, Linux) | `claude`, puis `/login` | 2.1.283 à 2.1.301 |
| [Codex](https://github.com/openai/codex) | `npm install -g @openai/codex` | `codex login` (compte ChatGPT) | 0.163.0 |
| [OpenCode](https://opencode.ai) | `npm install -g opencode-ai` (OpenCode 1 ; OpenCode 2 n'est pas encore pris en charge) | `opencode auth login` (un fournisseur ; les modèles locaux d'Ollama marchent aussi) | 1.19.3 |
| [GitHub Copilot CLI](https://github.com/github/copilot-cli) | `npm install -g @github/copilot` | `copilot login` (compte GitHub avec Copilot, Free compris) | 1.0.98 |

Une version plus ancienne que celle testée est refusée, avec la commande qui la met à jour ; une plus récente est utilisée, sans avertissement. Sous Windows, Escouade lance le binaire natif que le paquet npm installe, pas son lanceur. Les différences entre les quatre (modes et ce qui protège, coût et quota, vue terminal, ce qui n'est pas pris en charge) sont décrites dans docs/SPEC.md, « Harness », et sur la page [Harness](https://escouade.dev/docs/harness/) du site ; Cursor CLI est reporté.

## Installation

**Windows** : télécharge l'installeur `.exe` de la [dernière release](https://github.com/guillaume-gagnaire/escouade/releases/latest).

**macOS** : télécharge le `.dmg` (universel, Apple Silicon et Intel) de la [dernière release](https://github.com/guillaume-gagnaire/escouade/releases/latest), ouvre-le et glisse Escouade dans Applications. Depuis la 1.5.4, l'app est signée avec un certificat Apple Developer ID et notarisée par Apple : macOS l'ouvre sans demander d'autorisation.

**Linux** : télécharge l'`AppImage` de la [dernière release](https://github.com/guillaume-gagnaire/escouade/releases/latest), rends-la exécutable (`chmod +x Escouade_*.AppImage`) et lance-la ; c'est elle que la mise à jour intégrée remplace. Le `.deb` (Debian, Ubuntu) s'installe avec `sudo apt install ./Escouade_*_amd64.deb`. Sous Linux, les jetons des intégrations (Jira, Trello, GitHub, Linear) restent dans `~/.escouade/integrations.json` plutôt que dans un trousseau.

Les versions suivantes s'installent depuis l'application. Jusqu'à la 1.5.4, la barre de statut propose « Mise à jour disponible » ; à partir de la 1.6, la mise à jour se télécharge seule, la barre de statut affiche « Mise à jour X prête · Redémarrer », et l'app redémarre d'elle-même quand elle est au repos (réglage « Installer les mises à jour automatiquement », dans « À propos »).

## Données locales

Tout est stocké dans `~/.escouade/` (anciennement `~/.claude-code-manager/`, déplacé au premier lancement de la 0.1.4) :

| Fichier | Contenu |
|---|---|
| `settings.json` | réglages |
| `state.json` | projets, agents, état de l'interface |
| `integrations.json` | comptes Jira, Trello, GitHub et Linear, sans leurs jetons (dans le trousseau du système) |
| `sync-queue.json` | synchros avec Jira, Trello, GitHub ou Linear qui attendent d'être retentées |
| `conversations/<agent>.jsonl` | journal de chaque conversation |
| `tickets/<ticket>/` | images collées dans la fenêtre d'un ticket ou jointes par le serveur MCP, effacées avec lui |
| `claude/<compte>/` | dossier de configuration de Claude Code de chaque compte ajouté (sa connexion, ses conversations) |
| `codex/<compte>/`, `copilot/<compte>/` | dossier de Codex (`CODEX_HOME`) ou de Copilot CLI (`COPILOT_HOME`) de chaque compte ajouté |
| `mcp-token.json` | jeton du serveur MCP pour les harness hors Escouade, seulement quand le trousseau du système le refuse |
| `mcp/<agent>.json` | config MCP d'un agent d'Escouade, avec son jeton, effacée à l'arrêt de l'app |
| `logs/<agent>-setup.log` | sortie complète de la dernière préparation du worktree d'un agent |
| `update.json` | mise à jour qui s'installe, notée pour le démarrage suivant (puis effacée) |
| `stats.db` | statistiques (SQLite) |
| `app.log` | journal de l'application |

La variable d'environnement `ESCOUADE_DATA_DIR` (l'ancienne `CCM_DATA_DIR` marche encore) permet d'utiliser un autre dossier (démonstrations, tests).

## Raccourcis

Sous macOS, `Ctrl` devient `⌘` (sauf `Ctrl+Tab`, `⌘Tab` changeant d'application) ; dans les terminaux, `⌘C` / `⌘V` copient et collent, et les touches `Ctrl` vont au shell.

| Raccourci | Action |
|---|---|
| `Ctrl+1` … `Ctrl+9` | aller au projet n |
| `Ctrl+N` | nouvel agent |
| `Ctrl+Tab` / `Ctrl+Shift+Tab` | agent suivant / précédent |
| `Ctrl+J` | prochain agent en attente de réponse ou à voir (question, fin ou erreur pas encore vue), le curseur dans son champ de saisie |
| `Ctrl+Shift+A` | vue d'ensemble de tous les agents (aussi depuis un terminal) |
| `Ctrl+K` | rechercher dans les conversations (dans un terminal sous Windows, `Ctrl+K` reste au shell) |
| `Ctrl+P` | ouvrir un fichier par son nom (`nom:42` pour la ligne 42 ; dans un terminal sous Windows, `Ctrl+P` reste au shell) |
| `Ctrl+Entrée` / `Ctrl+Shift+Entrée` (dans la conversation) | autoriser / toujours autoriser la demande d'autorisation en attente (le texte tapé puis `Entrée` la refuse avec ce message) ; une demande qui vient d'arriver ignore ces touches une demi-seconde, et celle que Claude Code refuserait par défaut ne s'autorise qu'avec son bouton |
| `Ctrl+Entrée` (dans la fenêtre d'un ticket) | ajouter ou enregistrer le ticket depuis n'importe quel champ |
| `Alt+1` … `Alt+9` (dans la conversation) | choisir l'option n de la question en attente (coche ou décoche pour un choix multiple) ; `Ctrl+Entrée` valide quand la réponse est complète |
| `Ctrl+T` | nouveau terminal |
| `Ctrl+S` (dans l'éditeur) | enregistrer le fichier |
| `Ctrl+Shift+F` (dans l'éditeur) | rechercher dans tous les fichiers de la source |
| `Ctrl+clic` / `F12` (dans l'éditeur) | aller à ce que désigne le mot : fichier importé, lien, chemin, définition |
| `Shift+F12` (dans l'éditeur) | trouver les références du nom sous le curseur |
| `Alt+←` / `Alt+→` (dans l'éditeur ; sous macOS `⌃-` / `⌃⇧-`, avec la touche Contrôle et non `⌘`) | revenir là où tu étais avant un saut / y retourner (aussi les boutons « précédent » et « suivant » de la souris) |
| `Alt+clic` (dans l'éditeur) | ajouter un curseur |
| `F2` / `Suppr` (dans l'arborescence ; sous macOS `⌘⌫` pour supprimer) | renommer / supprimer vers la corbeille |
| `Ctrl+Shift+B` | panneau des fichiers non commités (disposition classique ; toujours affiché dans l'autre) |
| `Ctrl+Shift+L` | disposition classique / conversation et fichiers côte à côte |
| `Ctrl+,` | réglages |
| `Échap` (dans le champ de saisie) | interrompre Claude |
| `↑` (champ vide) | rappeler le dernier message envoyé, pour le renvoyer tel quel ou modifié (l'original reste dans la conversation) |
| `Alt+↑` / `Alt+↓` (réglages des worktrees) | monter / descendre une commande de préparation ou de démontage |

## Développement

Prérequis : Node.js 20.18+ et rustup (MSVC sous Windows ; sous macOS, les outils en ligne de commande Xcode). La version de Rust est épinglée dans `src-tauri/rust-toolchain.toml` (la même qu'en CI) et rustup l'installe automatiquement.

```powershell
npm ci
npm run tauri dev
```

Pour travailler sans toucher à tes vraies données (une version de développement lancée sans dossier à part déplacerait aussi `~/.claude-code-manager/`) :

```powershell
$env:ESCOUADE_DATA_DIR = "$env:TEMP\escouade-sandbox"; npm run tauri dev
```

Sous macOS :

```sh
ESCOUADE_DATA_DIR="$TMPDIR/escouade-sandbox" npm run tauri dev
```

### Tests

| Commande | Contenu |
|---|---|
| `npm run check` | typage (svelte-check) |
| `npm test` | Vitest + Testing Library : logique, stores et composants |
| `cd src-tauri; cargo test` | tests unitaires Rust et tests d'intégration du cœur contre un faux CLI `claude` (`tests/fixtures/fake-claude.mjs`), de vrais dépôts git temporaires et le runtime de test de Tauri |
| `npx tauri build --debug --no-bundle; npm run test:e2e` | tests de bout en bout : Playwright pilote l'application réelle via le protocole DevTools de WebView2 (Windows uniquement) |
| `$env:ESCOUADE_TERM_BENCH = 1; npx playwright test tests/e2e/terminal-latency.spec.ts` | banc de latence des terminaux, à lancer à la main (sauté sinon) : chaque étape d'une frappe et d'un collage, au calme puis avec des agents et des terminaux qui débitent (`docs/PROTOCOL.md`, *Terminaux intégrés*) ; `ESCOUADE_TERM_TRACE=1` donné à l'app écrit les temps du backend dans son journal |

Les tests de bout en bout ne gênent pas qui se sert du poste pendant qu'ils tournent : la fixture (`tests/e2e/fixture.ts`) donne `ESCOUADE_HIDDEN_WINDOW=1` à l'app, qui ouvre alors sa fenêtre hors de tous les écrans, à sa taille habituelle, sans jamais prendre le focus ni montrer de bouton dans la barre des tâches, et sans icône dans la zone de notification (la variable n'est lue qu'avec `ESCOUADE_DATA_DIR`, jamais par l'app de l'utilisateur). WebView2 y dessine comme dans une fenêtre visible (`--disable-backgrounding-occluded-windows` et deux autres options, `RENDER_UNSEEN` dans la fixture, aussi dans `ci.yml`), et la page se croit au premier plan, comme toute page que pilote Playwright (`Emulation.setFocusEmulationEnabled`) : `document.hasFocus()`, le focus des champs et le clavier des tests marchent comme devant l'utilisateur (`display.spec.ts`, « the window kept out of sight… », le vérifie). Pour regarder un test, fenêtre visible et au premier plan : `$env:ESCOUADE_E2E_SHOW = 1; npx playwright test tests/e2e/app.spec.ts -g "…"`. Un test qui ouvre un lien intercepte l'ouverture (`openUrl` lancerait le vrai navigateur) ; tout programme que l'app lance sans terminal l'est avec `CREATE_NO_WINDOW` (aucune console ne clignote).

La CI (`.github/workflows/ci.yml`) exécute l'ensemble sur chaque push et pull request sous Windows, et sous macOS tout sauf les tests de bout en bout, plus la construction du `.dmg` (téléchargeable dans les artefacts du run pour tester une branche).

### Publier une version

1. Une seule fois, ajoute les secrets du dépôt GitHub :
   - `TAURI_SIGNING_PRIVATE_KEY` : le contenu de la clé privée de signature des mises à jour (générée avec `npx tauri signer generate`, la clé publique correspondante est dans `src-tauri/tauri.conf.json`) ;
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` : son mot de passe (vide s'il n'y en a pas) ;
   - optionnel, pour signer et notariser l'app macOS (sans eux, elle est signée ad hoc et macOS demande une confirmation au premier lancement) : `APPLE_CERTIFICATE` (certificat Developer ID Application exporté en `.p12`, encodé en base64), `APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY` (ex. `Developer ID Application: Nom (TEAMID)`), `APPLE_ID`, `APPLE_PASSWORD` (mot de passe d'application) et `APPLE_TEAM_ID`.
2. Mets à jour la version partout, committe, tague et pousse :

   ```powershell
   npm run version:set -- 0.2.0
   git commit -am "chore: release 0.2.0"
   git tag v0.2.0
   git push origin main v0.2.0
   ```

   Ou crée la release depuis la page GitHub (« Draft a new release », nouveau tag `v0.2.0` sur `main`, « Publish ») : l'app prend la version du tag, même si celle de `main` n'a pas été montée.

Le workflow `release.yml` vérifie que le tag est une version et crée la release **en brouillon** (une release déjà créée sur la page GitHub y repasse). Puis, sous Windows, macOS et Linux en parallèle, il donne à l'app la version du tag, construit les installeurs (`.exe` NSIS ; `.dmg` et `.app.tar.gz` universels ; `.AppImage` et `.deb`), les signe et les dépose dans ce brouillon, retrouvé par son identifiant, avec le `latest.json` utilisé par la mise à jour automatique, commun aux trois systèmes. Les tests ne sont pas rejoués ici : ils tournent à chaque push sur `main` (`ci.yml`).

La mise à jour lit le `latest.json` de la dernière release : la release n'est donc publiée qu'à la fin, par le dernier job, une fois les trois constructions réussies. Il vérifie (`scripts/release-check.mjs`) que `latest.json` est celui de la version et contient chaque système, et que chaque fichier qu'il désigne est bien dans le brouillon ; puis il publie la release comme « latest » (une pré-version, tag `vX.Y.Z-…`, ne l'est jamais) et contrôle ce que lit la mise à jour. Enfin, le job `announce` poste les notes de la version sur le Discord d'Escouade (`discord.yml`, `scripts/discord.mjs`, secret `DISCORD_WEBHOOK_URL`), sauf pour une pré-version.

Si quelque chose casse :

- **une construction échoue** : la release reste en brouillon et la précédente reste la dernière pour la mise à jour. Relance les jobs ratés avec `gh run rerun <id> --failed` : leurs fichiers sont redéposés dans le même brouillon, sans danger ;
- **le contrôle de la publication échoue faute d'une plateforme** (deux constructions ont réécrit `latest.json` au même instant) : relance tout le run avec `gh run rerun <id>` ;
- **la publication elle-même est refusée** (« Resource not accessible by integration ») : le brouillon est complet, publie-le à la main avec `gh release edit vX.Y.Z --draft=false --latest` (sans `--latest` pour une pré-version) ;
- **l'annonce sur Discord échoue ou manque** : `gh run rerun <id> --failed` ne relance qu'elle, ou `gh workflow run discord.yml -f tag=vX.Y.Z` poste les notes d'une version déjà publiée.

Tague toujours la tête de `main`, et attends que la release soit créée (première minute du workflow) avant de pousser d'autres commits : une fois `main` plus loin que le tag, le `GITHUB_TOKEN` des Actions n'a plus le droit de créer la release (« Resource not accessible by integration »).

### Vidéo de présentation

Le dossier `video/` contient la vidéo de présentation (Remotion). Sa voix off mène tout : chaque scène dure le temps de ses phrases (`src/script.ts`), et l'animation se cale sur les mots prononcés. La voix et la musique viennent d'ElevenLabs (clé dans `ELEVENLABS_API_KEY`, jamais dans un fichier) ; elles sont versionnées, et seules les phrases modifiées sont redemandées :

```powershell
cd video
npm install
$env:ELEVENLABS_API_KEY = "…"
npm run voice               # public/voice/*.mp3 et src/voice.json
npm run voice -- --check    # retranscrit chaque phrase pour vérifier qu'elle est bien prononcée
npm run music               # public/music.mp3, recomposée quand la durée des scènes change
npm run studio              # aperçu
npm run stills -- board     # images de contrôle d'une scène dans out/stills
npm run render              # bruitages + out/presentation.mp4
npm run web-video           # out/escouade-web.mp4 : son à -16 LUFS, plus léger
npm test                    # chaque fonctionnalité montrée, cues, mixage, sous-titres
```

### Site

Le dossier `website/` contient le site de présentation (Nuxt, généré en statique), publié sur [escouade.dev](https://escouade.dev/) par Coolify à chaque push sur main (`website/Dockerfile` : génération, tests, puis nginx, qui renvoie aussi l’ancienne adresse escouade.gagnaire.dev vers la nouvelle). Il demande Node 22.19 ou plus récent :

```powershell
cd website
npm install
npm run dev        # aperçu
npm run generate   # site statique dans .output/public
npm test           # vérifie le site généré
```

Il existe en français (à la racine) et en anglais (sous `/en/`) ; ses textes sont dans deux catalogues de mêmes clés. Sa documentation (`/docs/`, `/en/docs/`) est faite de pages Markdown, une par langue, dans `website/content/docs/` : leur forme et leurs règles sont dans le `README.md` de ce dossier, leur ordre dans `website/app/data/docs.ts`. Ses images, sa vidéo et ses sous-titres viennent de `video/` : `npm run render`, `npm run web-video`, puis `npm run site-images` (captures, affiche et sous-titres dans les deux langues ; `-- --lang=en` pour une seule). La voix de la vidéo reste française, la page anglaise propose ses sous-titres anglais.

## Architecture

- `src-tauri/` : backend Rust (Tauri 2).
  - `engine/` : l'interface « moteur » (un harness : Claude Code, Codex, OpenCode, Copilot CLI ; Cursor CLI reporté), voir [docs/ENGINES.md](docs/ENGINES.md). `engine/claude/` en est l'implémentation pour Claude Code : pilotage d'un processus `claude` en mode `stream-json`, avec son protocole de contrôle (voir [docs/PROTOCOL.md](docs/PROTOCOL.md)).
  - `agent.rs` : conversation normalisée, statuts, questions et permissions, usage par tour ; `agent/claude.rs` y traduit les frames de Claude Code.
  - `core.rs` : orchestration.
  - Modules annexes : `git.rs`, `pty.rs`, `stats.rs`, `usage.rs` (quotas par compte), `accounts.rs` (comptes Claude), `i18n.rs` (langues du backend), `mcp/` (serveur MCP), `notify.rs`, `job.rs` (arbres de processus : Job Objects sous Windows, groupes de processus sous macOS), `shellenv.rs` (`PATH` du shell de connexion sous macOS).
  - `tauri.macos.conf.json` : ce qui change sous macOS (barre de titre native avec les boutons de fenêtre, bundle `.app` / `.dmg`).
- `src/` : interface Svelte 5 ; ses textes sont dans `src/lib/i18n/` (un catalogue par langue).
- `design/` : maquette de référence.
- `docs/SPEC.md` : spécification.

## Licence

GPL-3.0 ou ultérieure (`GPL-3.0-or-later`), voir [LICENSE](LICENSE).

Escouade est un logiciel libre : tu peux le redistribuer et le modifier selon les termes de la licence publique générale GNU, publiée par la Free Software Foundation, version 3 ou (à ton choix) toute version ultérieure. Il est distribué dans l'espoir qu'il sera utile, mais sans aucune garantie. Les versions jusqu'à la 1.7.7 comprise ont été publiées sous licence MIT.
