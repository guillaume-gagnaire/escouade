# Installation

Les prérequis de chaque système, les harness à installer, l’installation sous Windows, macOS et Linux, l’utilisation d’un Claude Code installé dans WSL, le PATH sous macOS, puis ce que tu vois au premier lancement et d’où viennent les mises à jour.

## Prérequis

| Élément | Windows | macOS | Linux |
|---|---|---|---|
| Système | Windows 10 ou 11 (WebView2, présent par défaut sur Windows 11) | macOS 11 (Big Sur) ou plus récent, sur Mac Apple Silicon ou Intel | x86-64, glibc 2.35 ou plus récente (Ubuntu 22.04, Debian 12, Fedora 36 et suivantes), avec WebKitGTK 4.1 |
| Git | Git for Windows | Les outils en ligne de commande Xcode (`xcode-select --install`) ou celui de Homebrew | Celui de ta distribution |

En plus :

- **Un harness**, installé et connecté à ton compte, comme tu le ferais pour l’utiliser dans un terminal : Claude Code (voir la [documentation de Claude Code](https://code.claude.com/docs/fr/overview)), ou Codex, OpenCode ou GitHub Copilot CLI (voir [plus bas](#les-autres-harness)). Escouade lance celui de ta machine.
- **Git** : Escouade s’en sert pour les diffs, les commits, les branches et les worktrees. Si le dossier d’un projet n’est pas un dépôt, Escouade en initialise un quand tu crées le projet.
- **Facultatif sous Windows** : PowerShell 7 (à défaut, les terminaux intégrés utilisent Windows PowerShell) et WSL.

### Où Escouade cherche Claude Code

Escouade cherche `claude` dans cet ordre :

1. le chemin de Réglages › Harness › « Chemin de l’exécutable », s’il n’est pas vide (c’est alors le seul endroit essayé) ;
2. le `PATH` ;
3. les endroits où les installateurs de Claude Code le mettent (tableau ci-dessous) ;
4. sous Windows, quand il n’y en a pas, un Claude Code installé dans WSL (voir [plus bas](#claude-code-dans-wsl)).

| Système | Emplacements essayés |
|---|---|
| Windows | `%UserProfile%\.local\bin\claude.exe`, `%UserProfile%\.claude\local\claude.exe`, `%UserProfile%\AppData\Roaming\npm\claude.cmd` |
| macOS et Linux | `~/.local/bin/claude`, `~/.claude/local/claude`, `/opt/homebrew/bin/claude`, `/usr/local/bin/claude` |

Le chemin des réglages accepte `~` pour ton dossier personnel, et sous Windows il peut se passer de `.exe` ou de `.cmd`. Laisse le champ vide pour la détection automatique. La ligne sous le champ dit où Escouade a trouvé `claude` (« Trouvé : … ») ou, s’il est introuvable, les formes de chemin acceptées.

Sans aucun harness, l’écran d’accueil dit « Aucun harness trouvé » avec la commande d’installation de chacun, et le Kanban ne démarre aucun ticket (« Aucun harness trouvé — aucun ticket ne démarre ») ; un agent dont le harness a disparu le dit dans sa conversation. Installe-en un ou indique le chemin de Claude Code dans les réglages : quand tu enregistres un autre chemin, ces messages se mettent à jour sans redémarrer Escouade.

### Les autres harness

Claude Code n’est pas obligatoire : Escouade fait aussi tourner ses agents sur Codex, OpenCode et GitHub Copilot CLI, et un seul harness lui suffit. Installe ceux que tu veux et connecte-les ; ils apparaissent alors dans Réglages › Harness, avec leur version. Ajoute ensuite le compte de chacun dans Réglages › Comptes : seuls les comptes ajoutés lancent des agents (voir [les comptes](accounts)).

| Harness | Installation | Connexion | Testé avec |
|---|---|---|---|
| Codex | `npm install -g @openai/codex` | `codex login` (compte ChatGPT) | 0.163.0 |
| OpenCode | `npm install -g opencode-ai` | `opencode auth login` (un fournisseur, ou les modèles locaux d’Ollama) | 1.19.3 |
| Copilot CLI | `npm install -g @github/copilot` | `copilot login` (compte GitHub avec Copilot) | 1.0.98 |

Escouade lance le binaire natif que le paquet npm installe (sous Windows, pas son lanceur). Une version plus ancienne que celle testée est refusée, avec la commande qui la met à jour ; une plus récente est utilisée, sans avertissement. L’app de bureau de Codex installe souvent une version plus ancienne, que le `PATH` peut trouver d’abord : installe alors celle de npm. OpenCode 2 n’est pas encore pris en charge.

Ce que chacun fait, ses modes et ce qui le protège, son coût et son quota : voir [les harness](harness).

## Installer Escouade

Les installeurs sont sur la page des [releases GitHub](https://github.com/guillaume-gagnaire/escouade/releases/latest).

### Windows

1. Télécharge l’installeur `.exe` de la dernière release.
2. Lance-le. Il installe Escouade pour ton compte utilisateur, sans demander de droits d’administrateur, et il parle la langue du système (français ou anglais).
3. Ouvre Escouade depuis le menu Démarrer.

> [!NOTE]
> Escouade s’appelait « Claude Code Manager ». Installée sur une ancienne version, elle la remplace et garde tes réglages et tes conversations.

### macOS

1. Télécharge le `.dmg` de la dernière release. Il est universel : le même fichier sert aux Mac Apple Silicon et Intel.
2. Ouvre-le et glisse Escouade dans le dossier Applications.
3. Lance Escouade.

L’app est signée avec un certificat Apple Developer ID et notarisée par Apple : macOS l’ouvre sans demander d’autorisation.

### Linux

L’`AppImage` fonctionne sur toute distribution qui remplit les prérequis :

```sh
chmod +x Escouade_*.AppImage
./Escouade_*.AppImage
```

C’est elle que la mise à jour intégrée remplace. Sur Debian ou Ubuntu, tu peux aussi installer le paquet `.deb` :

```sh
sudo apt install ./Escouade_*_amd64.deb
```

Sous Linux, les jetons des intégrations (Jira, Trello, GitHub, Linear) sont gardés dans `~/.escouade/integrations.json`, faute de trousseau du système pris en charge.

## Claude Code dans WSL

Sous Windows, Escouade peut faire tourner un Claude Code installé dans WSL.

1. Installe Claude Code dans ta distribution WSL et connecte-le.
2. Ouvre Réglages › Harness. Dans « Chemin de l’exécutable », écris le chemin de `claude` sous l’une de ces formes :
   - `~/.local/bin/claude` ;
   - `/home/<utilisateur>/.local/bin/claude` ;
   - `\\wsl.localhost\<distribution>\home\<utilisateur>\.local\bin\claude`.
3. Laisse le champ vide si Windows n’a pas de `claude` : Escouade cherche alors le sien dans WSL, aux endroits habituels (`~/.local/bin/claude` en premier).
4. Lis la ligne sous le champ : « Trouvé dans WSL (Ubuntu) : … » confirme que `claude` est trouvé, et dans quelle distribution.

La distribution utilisée est celle du chemin quand il la nomme (`\\wsl.localhost\<distribution>\…`), sinon celle de Réglages › Terminaux › « Distribution WSL », sinon la distribution par défaut de WSL.

Escouade lance `claude` par `wsl.exe`, dans le dossier de l’agent (vu depuis WSL sous `/mnt/c/…`). Ses sessions, sa connexion et son quota sont lus dans le dossier personnel Linux. Un dossier de projet que WSL n’atteint pas (un partage réseau, le dossier d’une autre distribution) est refusé.

> [!WARNING]
> Les outils MCP d’Escouade et le suivi en direct de la [vue Claude Code](claude-code-view) demandent le réseau miroir de WSL. Ajoute `networkingMode=mirrored` sous `[wsl2]` dans `%UserProfile%\.wslconfig`, puis lance `wsl --shutdown`. Quand le réseau de WSL est en NAT, les réglages le disent sous le champ du chemin.

Claude Code est lancé dans WSL sans le profil du shell : une variable posée dans `~/.profile` ou `~/.bashrc` ne lui parvient pas.

## Le PATH sous macOS

Une app lancée depuis le Finder ou le Dock ne reçoit pas le `PATH` de ton terminal : sans précaution, `claude` ou `git` installés avec Homebrew resteraient introuvables. Au démarrage, Escouade demande donc son `PATH` à ton shell de connexion (`$SHELL -ilc`, avec 5 secondes pour répondre) et le réunit avec celui qu’elle a reçu : les dossiers du shell d’abord, puis ceux déjà présents, sans doublon. Elle y ajoute `/opt/homebrew/bin`, `/usr/local/bin` et `~/.local/bin` s’ils n’y sont pas. Ce `PATH` est celui de tous les processus qu’elle lance : Claude Code, git et les commandes des agents.

Sous Linux, le même mécanisme s’applique, ce qui sert quand Escouade est lancée depuis le lanceur du bureau plutôt que depuis un terminal.

Si `claude` reste introuvable, indique son chemin dans Réglages › Harness.

## Premier lancement

Tant qu’aucun projet n’existe, Escouade ouvre un écran d’accueil : son logo, la phrase « Ajoute un projet pour y lancer des agents, suivre leurs questions et ouvrir des terminaux. » et le bouton « + Ajouter un projet ».

- Si aucun harness n’est trouvé, un avertissement le dit : « Aucun harness trouvé », avec la commande d’installation de chacun. Ouvre alors les réglages avec le bouton ⚙ de la barre d’état ou `Ctrl+,`, onglet « Harness ». Sous macOS, `Ctrl` devient `⌘`.
- Tant qu’aucun compte n’est ajouté, l’écran dit « Aucun compte » : « Ajoute un compte : celui déjà connecté sur cette machine, ou un nouveau. Seuls les comptes ajoutés lancent des agents. » Le bouton « Ajouter un compte… » propose « Le compte de la machine », ton harness déjà connecté, ou « Un nouveau compte » (voir [les comptes](accounts#ajouter-un-compte)).
- Clique sur « + Ajouter un projet » pour ouvrir la fenêtre « Nouveau projet ». La [prise en main](quick-start) la décrit champ par champ et va jusqu’au premier commit.
- Escouade te demande aussi, dans une fenêtre « Aider à améliorer Escouade ? », si elle peut envoyer des statistiques d’usage : « Partager les statistiques » ou « Non merci ». Rien n’est envoyé avant ta réponse. Le détail est sur la page [Confidentialité](/privacy/), et le choix se change dans Réglages › À propos (voir [les réglages « À propos »](settings-about)).

La fenêtre s’ouvre en 1440 × 900 (1000 × 600 au minimum) ; Escouade retient ensuite sa taille et sa position. Tes projets, tes conversations et tes réglages sont écrits dans `~/.escouade/` : voir les [données locales](data).

## Mises à jour

Les mises à jour viennent des releases GitHub d’Escouade. L’app télécharge la nouvelle version en arrière-plan et l’installe sans fenêtre d’installeur. La barre d’état affiche alors « Mise à jour X prête · Redémarrer » : tu redémarres quand tu veux, ou Escouade redémarre d’elle-même quand aucun agent ne travaille et que tout est enregistré. Ce réglage, « Installer les mises à jour automatiquement », est activé par défaut ; il est dans Réglages › À propos, où « Rechercher une mise à jour » lance une vérification tout de suite.

Voir les [mises à jour](updates) et les [réglages « À propos »](settings-about).
