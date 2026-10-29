---
title: "Claude Code dans WSL"
subject: "Harness"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Sous Windows, les agents peuvent tourner avec un Claude Code installé seulement dans une distribution WSL, sans script intermédiaire. Escouade trouve le programme dans la distribution, l'y lance dans le dossier de l'agent, et lit son dossier de configuration par le partage `\\wsl.localhost`.

## Comportement

### Chemin

- Le chemin (Réglages › Harness, ou l'exécutable d'un compte) peut être :
  - `\\wsl.localhost\<distribution>\…` ou `\\wsl$\<distribution>\…` : la distribution du chemin, jamais lancé comme un programme de Windows ;
  - un chemin Linux `/home/<utilisateur>/…` ;
  - `~/…`.
- `~` désigne d'abord un programme de Windows sous `%UserProfile%` (`~/.local/bin/claude` y trouve le `claude.exe` de l'installeur natif), sinon le dossier personnel Linux de l'utilisateur par défaut de la distribution.
- Un chemin Linux va à la distribution de Réglages › Terminaux › « Distribution WSL », sinon à celle par défaut.
- Partout, `~` est le dossier personnel, et sous Windows `.exe` ou `.cmd` peut être omis.
- Champ vide : le `claude` de Windows (PATH, installeurs), sinon celui de la distribution, cherché dans `~/.local/bin`, `~/.claude/local`, `/usr/local/bin` puis `/usr/bin`.

### Recherche

- Les distributions et la distribution par défaut sont lues dans le registre de WSL.
- L'utilisateur sous lequel `wsl.exe` lance est celui que nomme `[user] default` de `/etc/wsl.conf` (cas d'une distribution importée, dont le registre dit root), sinon celui du registre (`DefaultUid`). Son dossier personnel vient de `/etc/passwd`, sinon il est demandé à la distribution (`$HOME`).
- La racine des lecteurs vient de `[automount] root` de `/etc/wsl.conf` (`/mnt/` par défaut).
- Ces fichiers sont lus par le partage `\\wsl.localhost`. Le programme y est présent s'il est un fichier ou un lien (Windows ne suit pas les liens de WSL, et le `~/.local/bin/claude` de l'installeur natif en est un). Sinon, la distribution le confirme elle-même (`[ -f <chemin> ]`, 20 s au plus), ce qui suit un dossier lien comme `/bin`.
- Regarder dans une distribution arrêtée la démarre : chaque compte actif cherche donc son `claude` au démarrage, hors du fil de la fenêtre.
- Un programme trouvé reste trouvé. Un programme introuvable est recherché de nouveau 5 min plus tard, ou tout de suite quand la ligne sous le champ le demande. Enregistrer un chemin fait tout rechercher de nouveau.

### Lancement

- Commande : `wsl.exe -d <distribution> --exec /bin/sh -c '<script>' <claude> <dossier Linux> <arguments>`. Sans shell entre les deux, aucun argument n'est lu comme du code.
- Le script entre dans le dossier de l'agent (`cd`) ou s'arrête (code 126) : `wsl.exe`, lui, tomberait sans un mot sur le dossier personnel ou sur `/`. Un dossier sans chemin dans WSL (un partage réseau, le dossier d'une autre distribution) est refusé : « Le dossier <dossier> n'est pas joignable depuis WSL (Ubuntu) : le Claude Code qui y est installé ne peut pas y travailler. »
- Le script met le dossier du programme en tête du `PATH` : une installation npm sous nvm y trouve son `node`.
- Une variable posée sur `wsl.exe` n'atteint Linux que si `WSLENV` la nomme : chacune y est donc nommée (réseau, liste de tâches, `CLAUDE_CONFIG_DIR`, `ESCOUADE_HOOK_TOKEN`, `TERM` et `COLORTERM` des terminaux). Aucune ne passe sur la ligne de commande, ni un jeton ni le mot de passe d'un proxy.
- `WSL_UTF8=1` fait écrire ses propres messages à `wsl.exe` en UTF-8 plutôt qu'en UTF-16.
- `--mcp-config`, `--settings` et `CLAUDE_CONFIG_DIR` passent par leur chemin Linux.
- `wsl.exe` refuse un argument vide (`--tools ""`, `--setting-sources ""`) : un marqueur le remplace, que le script remet à vide.
- Les cinq lancements y passent : agent, `claude -p` (réponse JSON cherchée aussi après une ligne d'avertissement), `claude mcp add|remove`, vue Claude Code et connexion d'un compte.
- Les lignes de sa sortie qui ne sont pas des trames (un message de `wsl.exe`) rejoignent son message de fin, avec sa sortie d'erreur.
- Arrêter l'agent tue `wsl.exe`, ce qui arrête le processus Linux et ce qu'il a lancé, sauf ce qui s'est détaché (`setsid`).

### Dossier de Claude Code et comptes

- Le dossier de Principal est le `~/.claude` (et `~/.claude.json`) du dossier personnel Linux. L'app le lit par `\\wsl.localhost\<distribution>\home\…` : sessions de la vue Claude Code, connexion et quota, e-mail, déclaration MCP.
- La commande MCP à lancer à la main est écrite pour le shell de la distribution.
- Un compte ajouté garde son dossier sous `~/.escouade/claude/<compte>` de Windows, donné à Claude par son chemin `/mnt/c/…`. Partager les éléments de Principal depuis WSL se fait par copie : une jonction ne mène pas dans WSL.
- Une session ne passe pas d'un compte qui fait tourner Claude Code sous Windows à un compte où il tourne dans WSL, ni l'inverse : Claude Code range une session sous le dossier où il tourne, que WSL écrit autrement. « Reprendre sur… » le refuse : « Le compte « Pro » fait tourner Claude Code dans WSL, celui de l'agent sous Windows : sa session ne passe pas de l'un à l'autre. » La reprise automatique d'un agent de ticket, et le changement de compte au seuil, ne choisissent qu'un compte du même côté (voir *Comptes*).

### Chemins que donne Claude

- `/mnt/<lecteur>/…` se lit `<lecteur>:/…` : fichiers touchés, plans, autorisations.
- Le `transcript_path` des hooks de la vue Claude Code se lit par le partage.
- Le texte d'une copie de conversation (« … travaillait dans <dossier> ») donne les dossiers en `/mnt/c/…`.

### Worktrees

git de Windows écrit `gitdir: C:/…` dans le `.git` d'un worktree, chemin que git de Linux ne suit pas. Avant chaque démarrage d'un `claude` de WSL, le `.git` du worktree de l'agent est réécrit en chemin relatif (`gitdir: ../../../.git/worktrees/<nom>`), que les deux lisent.

### Réseau

Le serveur MCP et les hooks de la vue Claude Code écoutent sur 127.0.0.1 de Windows, que WSL ne joint qu'en réseau miroir (`networkingMode=mirrored` sous `[wsl2]` dans `%UserProfile%\.wslconfig`). En NAT (le défaut de WSL), les conversations marchent, mais pas les outils MCP d'Escouade ni le suivi en direct de la vue Claude Code (son transcript reste lu).

## Réglages

- Réglages › Harness, chemin de Claude Code : une ligne dessous dit où est son `claude`, redemandée 250 ms après chaque frappe (une réponse à un chemin d'avant est écartée) :
  - « Trouvé : <chemin> » ;
  - « Trouvé dans WSL (<distribution>) : <chemin> » ;
  - « Introuvable. Indique le chemin de claude.exe, ou de Claude Code installé dans WSL : ~/.local/bin/claude, /home/<utilisateur>/…, ou \\wsl.localhost\<distribution>\…. » (sous macOS et Linux : « Introuvable. Indique le chemin de claude. »).
- Pour un `claude` de WSL en NAT, une note sous le groupe : « Le réseau de WSL est en NAT : le serveur MCP d'Escouade et le suivi de la vue Claude Code ne joignent pas l'app. Passe-le en mode miroir (networkingMode=mirrored sous [wsl2] dans %UserProfile%\.wslconfig), puis lance wsl --shutdown. »
- Enregistrer un autre chemin ou une autre distribution met à jour le bandeau « Claude Code introuvable » du Kanban et l'indice du champ, sans redémarrer.
- Réglages › Terminaux › « Distribution WSL » : la distribution d'un chemin Linux.
- L'exécutable d'un compte, dans sa fiche de Réglages › Comptes.

## Limites

- Claude Code est lancé sans le profil du shell (`~/.profile`, `~/.bashrc` ne sont pas lus) : une variable ou un `PATH` qui n'est posé que là (clé d'API, nvm d'un serveur MCP) lui manque.
- git de WSL lit sa propre configuration (`user.name`, `core.autocrlf` : un dépôt extrait en CRLF par Windows peut y paraître modifié).
- Le lien du dépôt vers un worktree (`.git/worktrees/<nom>/gitdir`) reste en `C:/…` : git de WSL tient ce worktree pour absent dans `git worktree list`.
- Les fichiers sous `/mnt/c` sont plus lents qu'en Linux.
- Une racine des lecteurs autre que `/mnt/` n'est pas reconnue dans les chemins que donne Claude.
- Les éléments de Principal qui sont des liens dans WSL ne sont ni proposés ni copiés à la création d'un compte.
- Un projet rangé dans la distribution (`\\wsl.localhost\…`) n'est pas pris en charge.
- En NAT, ni les outils MCP d'Escouade ni le suivi en direct de la vue Claude Code.
