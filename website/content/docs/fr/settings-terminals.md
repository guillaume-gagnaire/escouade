# Réglages : les terminaux (Avancé)

Le groupe « Terminaux » de la page « Avancé » dit à Escouade où trouver les shells que les terminaux intégrés, les commandes de lancement, les commandes de worktree et les tests utilisent.

## Ouvrir la page

Ouvre les réglages avec `Ctrl+,` (`⌘,` sous macOS), puis choisis « Avancé », groupe « Terminaux ». Ces réglages sont ceux de l’app, communs à tous les projets, et s’appliquent au geste. Les terminaux eux-mêmes sont décrits dans [les terminaux](terminals).

Le groupe tient en quelques chemins. Escouade détecte les shells seule ; tu ne remplis un champ que pour désigner un shell qu’elle ne trouve pas, ou en préférer un autre.

## Chemins

Sous le titre du groupe, la ligne « Détectés : » liste les shells trouvés, par exemple « Détectés : PowerShell, Git Bash, WSL », ou « Détectés : aucun ». Elle se met à jour une fois les réglages enregistrés.

Sous Windows, le groupe a trois champs :

| Réglage | Effet | Par défaut |
|---|---|---|
| « PowerShell 7 » | Le chemin de `pwsh.exe`. | Vide (« vide = auto ») |
| « Git Bash » | Le chemin de `bash.exe`, l’exécutable bash de Git for Windows. | Vide (« vide = auto ») |
| « Distribution WSL » | Le nom de la distribution WSL à utiliser, par exemple `Ubuntu`. | Vide (« vide = distribution par défaut ») |

Sous macOS, le groupe n’a qu’un champ, « bash » (« vide = auto »), pour le chemin de bash.

> [!NOTE]
> Sous Linux, le groupe affiche les trois mêmes champs que sous Windows, mais seul le champ « Git Bash » compte : il désigne alors `bash`. Les champs « PowerShell 7 » et « Distribution WSL » n’y ont aucun effet.

Un champ laissé vide laisse Escouade chercher. Un chemin qui ne désigne aucun fichier n’est pas remplacé par la détection : le shell correspondant n’est pas proposé. Pour PowerShell, Escouade se rabat alors sur Windows PowerShell s’il est installé.

## Comment les shells sont détectés

### Sous Windows

Les shells sont listés dans cet ordre :

1. **PowerShell** : le chemin du champ « PowerShell 7 » s’il est rempli. Sinon `pwsh.exe` dans le `PATH`, puis les dossiers `PowerShell\7` et `PowerShell\7-preview` de Program Files, puis l’alias du Microsoft Store (`WindowsApps\pwsh.exe`). Sans PowerShell 7, Windows PowerShell (`powershell.exe`, fourni avec Windows) est proposé à la place, sous le nom « Windows PowerShell ».
2. **Git Bash** : le chemin du champ « Git Bash » s’il est rempli. Sinon le `bash.exe` du dossier de Git, retrouvé à partir de `git.exe` dans le `PATH` (`bin\bash.exe` ou `usr\bin\bash.exe`), puis `Git\bin\bash.exe` dans Program Files.
3. **WSL** : proposé dès que `wsl.exe` existe. Il s’appelle « WSL », ou « WSL (Ubuntu) » avec la distribution du champ « Distribution WSL ». Le terminal s’ouvre dans le dossier voulu, dans cette distribution ; champ vide, c’est la distribution par défaut de WSL.

### Sous macOS et Linux

Le shell de connexion (la variable `SHELL`) vient en premier s’il s’agit de zsh, bash, fish ou sh, puis zsh, bash et fish, cherchés dans `/bin`, `/usr/bin` et le `PATH`. `sh` n’est proposé que si aucun autre shell n’est trouvé. Le champ « bash » remplace l’emplacement de bash, sauf quand `SHELL` désigne déjà bash : c’est alors ce bash-là qui sert.

## Quel shell sert à quoi

L’ordre de la liste compte : il n’y a pas de réglage « shell par défaut », c’est le premier shell détecté.

- Dans la barre latérale, le bouton **+** de la section « Terminaux » (« Nouveau terminal ») ouvre un menu avec tous les shells détectés : tu choisis lequel.
- « Ouvrir un terminal » (menu d’un agent) et « Ouvrir un terminal ici » (menu de l’arborescence de l’éditeur) utilisent le premier shell.
- Une commande de lancement ou une commande de worktree a son propre choix « Shell », dans les réglages du projet (voir [ses pages](settings-projects)). Une nouvelle commande part avec le premier shell. Voir [le lancement](launch) et [les worktrees](worktrees).
- La « Commande de tests » du Kanban ([sa page « Kanban »](settings-board)) et les lancements de test ([les tests](testing)) utilisent aussi le premier shell.

Sans shell détecté, le bouton **+** affiche « Aucun shell détecté (PowerShell 7, Git Bash, WSL). Vérifie les réglages. »

## Ce qui est réglé ailleurs

- **Le proxy dans les terminaux** : « Appliquer aussi le proxy aux terminaux intégrés » est dans le groupe [« Réseau »](settings-network), juste au-dessus. Il vaut aussi pour les commandes de lancement et de worktree.
- **Claude Code dans WSL** : sous Windows, quand le chemin de l’exécutable ([« Comptes et harness »](settings-accounts) › Avancé) est vide et qu’aucun `claude` n’est trouvé sous Windows, Escouade en cherche un installé dans WSL, dans la distribution du champ « Distribution WSL » (la distribution par défaut s’il est vide).
