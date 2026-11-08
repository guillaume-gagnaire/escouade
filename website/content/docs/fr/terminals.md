# Les terminaux

Escouade a de vrais terminaux intégrés, un par shell que tu ouvres, rangés par projet dans la barre latérale. Cette page explique comment les ouvrir, quels shells sont proposés, comment copier, coller et chercher, et quels raccourcis restent au shell.

## Ouvrir un terminal

Un terminal est un vrai shell, dans un pseudo-terminal (ConPTY sous Windows, pty sous macOS et Linux) affiché par xterm.js. Son historique, son invite et sa complétion sont donc ceux du shell lui-même : la touche `Tab` complète comme dans n’importe quel terminal. Les adresses `http://` et `https://` qui s’affichent dans la sortie s’ouvrent dans ton navigateur d’un simple clic, sans touche à tenir. Le terminal garde 10 000 lignes d’historique.

| Pour ouvrir… | Fais ceci |
|---|---|
| Un terminal dans le dossier du projet | `Ctrl+T` (`⌘T` sous macOS), ou le bouton + de la section « Terminaux » de la barre latérale. Le raccourci utilise le premier shell de la liste ; le bouton + propose chaque shell. |
| Un terminal dans le worktree d’un agent | « Ouvrir un terminal » dans le menu « … » de l’en-tête de sa conversation ; ou le bouton + de la section « Terminaux », puis « Dans le worktree de… » et le nom de l’agent ; ou un clic droit sur la carte de l’agent, puis « Ouvrir un terminal ». Le bouton et le menu + ne concernent que les agents qui ont un worktree et ne sont pas archivés (« Dans le worktree de… » n’apparaît pas quand il n’y en a aucun) ; sans worktree, le clic droit ouvre le terminal dans le dossier du projet. Voir [les worktrees](worktrees). |
| Un terminal dans un dossier de l’éditeur | Clic droit sur un dossier de l’arborescence de [l’éditeur](editor), puis « Ouvrir un terminal ici ». Sur l’espace libre, c’est la racine de la source (le worktree de l’agent ou le projet). |

Un terminal est nommé d’après son shell et un numéro (`pwsh-1`), ou d’après son shell et l’agent ou le dossier d’où il vient (`pwsh · refacto-auth`, `pwsh · src`). Il s’ajoute à la section « Terminaux » du projet, avec le nombre de terminaux ouverts : un clic sur sa ligne l’affiche, la croix de la ligne le ferme. Quand le shell s’arrête, la ligne porte « terminé ».

Sans shell trouvé, un message le dit : « Aucun shell détecté (PowerShell 7, Git Bash, WSL). Vérifie les réglages. »

## Les shells disponibles

Escouade détecte les shells installés sur ta machine.

| Système | Shells proposés |
|---|---|
| Windows | « PowerShell » (PowerShell 7, cherché dans le `PATH` puis dans `Program Files`) ; « Windows PowerShell » à la place quand PowerShell 7 est absent ; « Git Bash », celui de Git for Windows ; « WSL » (« WSL (Ubuntu) » quand une distribution est réglée). |
| macOS et Linux | Ton shell de connexion (`$SHELL`) en premier, puis zsh, bash et fish quand ils sont installés. `sh` sert de dernier recours quand aucun autre n’est trouvé. |

Le premier de la liste est celui que `Ctrl+T` ouvre. Sous macOS et Linux, les shells démarrent en shells de connexion (`-l`), comme dans un terminal ordinaire. WSL s’ouvre dans le dossier du projet (ou celui de l’agent) et prend la distribution par défaut, ou celle de Réglages › Avancé › Terminaux.

Les chemins de PowerShell 7 et de Git Bash, la distribution de WSL et, sous macOS, le chemin de bash se règlent dans Réglages › Avancé › Terminaux, qui montre aussi les shells détectés (sous Linux, seul le champ « Git Bash » compte, pour bash) : voir [les réglages des terminaux](settings-terminals).

## Dans le terminal

Le terminal prend toute la zone principale. Son en-tête donne son nom, son shell et le dossier du projet, et il a trois boutons : la loupe (voir [plus bas](#rechercher-effacer-fermer)), « Effacer », qui vide l’écran, et « Fermer le terminal ». Fermer le terminal arrête le shell. Sous Windows, il arrête aussi les programmes que le shell a lancés, sauf ceux qui ont ouvert une fenêtre (un navigateur, un éditeur), qui restent ouverts.

Le terminal garde son contenu quand tu affiches autre chose, par exemple une conversation, puis que tu reviens.

Quand le shell s’arrête (un `exit`, par exemple), une barre dit « Processus terminé. » ou « Processus terminé (code 1). », avec « Relancer », qui ferme celui-ci et en ouvre un nouveau avec le même shell, dans le dossier du projet, et « Fermer ».

### Copier et coller

| Système | Copier | Coller |
|---|---|---|
| Windows et Linux | Sélectionne avec la souris, puis `Ctrl+C` (ou `Ctrl+Maj+C`). Sans sélection, `Ctrl+C` interrompt le programme comme d’habitude. | `Ctrl+V` |
| macOS | `⌘C` sur la sélection | `⌘V` |

Sous macOS, les touches `Ctrl` vont toutes au shell (sauf `Ctrl+Tab`).

### Les raccourcis d’Escouade

Un terminal garde presque toutes les touches pour le shell. Le comportement dépend du système.

- Sous Windows et Linux, ces raccourcis passent à Escouade : `Ctrl+Tab` et `Ctrl+Maj+Tab` (agent suivant et précédent), `Ctrl+Maj+A` (la [vue d’ensemble](overview)), `Ctrl+1` à `Ctrl+9` (aller au projet n), `Ctrl+,` (les réglages) et `Ctrl+J` (le prochain agent en attente). `Ctrl+Maj+B` (le panneau des fichiers) et `Ctrl+Maj+L` (la disposition) marchent aussi : le terminal n’envoie rien au shell pour `Ctrl+Maj` suivi d’une lettre. Les autres touches `Ctrl` suivies d’une lettre, dont `Ctrl+K`, `Ctrl+P`, `Ctrl+N` et `Ctrl+T`, vont au shell : un `Ctrl+T` tapé dans un terminal n’ouvre donc pas un nouveau terminal.
- Sous macOS, tous les raccourcis `⌘` d’Escouade fonctionnent dans un terminal, et `⌘C`, `⌘V` et `⌘A` (tout sélectionner) sont ceux du terminal. `Ctrl+Tab` passe à l’agent suivant.

### Rechercher, effacer, fermer

- La loupe de l’en-tête, ou `Ctrl+Maj+F` (`⇧⌘F` sous macOS) quand le terminal a le focus, ouvre un champ « Rechercher… » : `Entrée` va à l’occurrence suivante, `Maj+Entrée` à la précédente et `Échap` ferme le champ et rend le focus au terminal.
- « Effacer » vide l’écran du terminal.
- « Fermer le terminal » le ferme.

## Le proxy

Dans Réglages › Avancé › Réseau, l’interrupteur « Appliquer aussi le proxy aux terminaux intégrés » (désactivé par défaut) donne aux terminaux les variables `HTTPS_PROXY`, `HTTP_PROXY` (et leurs formes en minuscules) avec l’adresse du proxy, et `NO_PROXY` avec les exclusions. Il vaut pour les terminaux ouverts après le changement, et aussi pour les commandes de lancement et les commandes de préparation et de démontage des worktrees. Voir [les réglages du réseau](settings-network).

## Les autres terminaux d’Escouade

Escouade se sert du même terminal ailleurs : chaque commande de lancement tourne dans le sien, en lecture seule (voir [lancer le projet](launch)), et la vue Claude Code d’un agent est Claude Code lui-même dans un terminal (voir [la vue Claude Code](claude-code-view)), et la fenêtre qui connecte un compte Claude affiche le `claude` du compte dans un terminal (voir [les comptes Claude](accounts)).
