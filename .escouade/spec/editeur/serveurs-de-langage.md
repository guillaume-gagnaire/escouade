---
title: "Serveurs de langage"
subject: "editeur"
sources: ["docs/EDITOR-STUDY.md","docs/TODO-2.3.md","docs/REMOTE.md"]
tickets: ["ESC-446"]
decisions: [165]
compared: null
---

## Ce que c'est

Les serveurs de langage que le cœur d'Escouade lance pour l'éditeur (voir *Éditeur*) : un programme du poste par langage, qui comprend le code (erreurs, complétion, définitions). Le cœur les trouve, les lance, les partage entre les vues, les relance et garde leurs journaux. La page ne lance jamais un programme : elle nomme un serveur ou un langage.

## Comportement

### Le catalogue

| Id | Langages | Programme (dans l'ordre) | Arguments | Où d'autre | Exécute du code du projet | Pour l'installer |
|---|---|---|---|---|---|---|
| `typescript` | TypeScript, JavaScript (et leurs variantes React) | `typescript-language-server` | `--stdio` | `node_modules/.bin` | oui : le `tsserver` du `node_modules/typescript` du projet | `npm install -g typescript-language-server typescript` |
| `rust` | Rust | `rust-analyzer` | — | `rustup which rust-analyzer`, `~/.cargo/bin` | oui : les `build.rs` et les macros procédurales (`cargo check`) | `rustup component add rust-analyzer` |
| `python` | Python | `basedpyright-langserver`, puis `pyright-langserver` | `--stdio` | `node_modules/.bin`, `~/.local/bin`, `%APPDATA%\Python\*\Scripts` | non | `pip install basedpyright` |
| `go` | Go | `gopls` | — | `$(go env GOPATH)/bin`, sinon `~/go/bin` | oui, par précaution : la toolchain que nomme son `go.mod`, cgo | `go install golang.org/x/tools/gopls@latest` |
| `json` | JSON, JSONC | `vscode-json-language-server` | `--stdio` | `node_modules/.bin` | non | `npm install -g vscode-langservers-extracted` |
| `css` | CSS, SCSS, Less | `vscode-css-language-server` | `--stdio` | `node_modules/.bin` | non | `npm install -g vscode-langservers-extracted` |
| `html` | HTML | `vscode-html-language-server` | `--stdio` | `node_modules/.bin` | non | `npm install -g vscode-langservers-extracted` |

### Les trouver

- Dans l'ordre : le chemin réglé à la main dans l'app ; le `path` du `.escouade.yml` du projet ; puis, programme par programme (chacun cherché partout avant le suivant), le `node_modules/.bin` du worktree puis celui du projet (serveurs npm seulement), le `PATH`, et l'endroit où son outil l'installe.
- Un chemin réglé à la main l'emporte : un chemin absolu, ou le nom d'un programme du `PATH`. S'il ne mène à rien, c'est dit (« le chemin réglé à la main ne mène à aucun programme »), et aucun autre n'est pris à sa place.
- Un serveur absent est dit avec la commande qui l'installe. Rien n'est téléchargé.

### La confiance du projet

- Un serveur de langage est un programme du poste, choisi par l'utilisateur ou trouvé dans le projet.
- Un dépôt cloné peut contenir un faux serveur : ce que le dépôt choisit (un programme de son `node_modules/.bin`, le `path` ou les `args` de son `.escouade.yml`) ne se lance qu'une fois que l'utilisateur fait confiance au projet pour ses serveurs, une fois par projet, sur sa machine. Avant, le lancement est refusé et dit : « ce serveur de langage vient du projet (…) : confirme d'abord que tu fais confiance au projet ».
- Un serveur qui exécute du code du projet (rust-analyzer, typescript-language-server, gopls ; voir le catalogue) attend la même confiance pour se lancer dans un projet, même installé sur le poste ou réglé à la main : le programme est celui de l'utilisateur, le code qu'il fait tourner est celui du dépôt. Comme VS Code, qui les coupe hors de sa Workspace Trust. Le refus le dit : « ce serveur exécute du code du projet (build.rs, macros) : confirme d'abord que tu fais confiance au projet ».
- Les autres (pyright et basedpyright, JSON, CSS, HTML), pris sur le poste avec les arguments du catalogue ou de l'app, ou réglés à la main même dans le dossier du projet, se lancent sans rien demander.
- La confiance est gardée dans les réglages de l'app, par le cœur seul : une sauvegarde des réglages par la fenêtre ne l'accorde ni ne la retire. Retirée, les serveurs du projet qui en dépendaient s'arrêtent.

### Les lancer et les partager

- Un serveur tourne dans le dossier du projet ou du worktree, dans un Job du cœur (un Job object sous Windows, un groupe de processus sous Linux et macOS) : ce qu'il lance meurt avec lui, et avec l'app, même tuée.
- Sous Windows, il est lancé avec `NoDefaultCurrentDirectoryInExePath=1` : un serveur du `PATH` est souvent un petit script npm (`.cmd`) qui appelle `node` par son nom, et `cmd` le chercherait d'abord dans son dossier courant, la racine du worktree. Un `node.exe`, `node.cmd` ou `node.bat` posé à la racine d'un dépôt cloné ne s'exécute donc jamais à sa place.
- Une instance par dossier et par serveur, partagée par les vues qui en ont besoin. Quand plus aucune vue ne s'en sert, il s'arrête après un délai (60 s) : une vue fermée puis rouverte entre-temps le retrouve. Une page rechargée lâche les serveurs qu'elle tenait.
- Tout s'arrête quand l'app se ferme.

### Les relancer

- Un serveur qui s'arrête sans qu'on le lui demande est relancé après un délai qui double (1 s, 2 s, 4 s… jusqu'à 30 s), au plus 5 fois de suite. Une marche de plus de 2 minutes remet le compte à zéro.
- Au-delà, il est « Arrêté après trop de plantages », avec la raison de son dernier arrêt, et ne repart que si on le relance (« Relancer »), le compte remis à zéro.
- Chaque relance est dite à la vue (son état et le numéro de sa marche), qui réinitialise le serveur.

### Le transport

- JSON-RPC sur l'entrée et la sortie standard du serveur, cadré par `Content-Length` (le protocole de base de LSP), relayé par des commandes et des événements du cœur (`lsp-message`, `lsp-state`). Le gestionnaire écrit dans un « puits » : la fenêtre aujourd'hui, l'API distante du cœur (voir *Mode serveur*) demain.
- Les messages sont bornés à 32 Mo dans les deux sens. Une longueur annoncée trop grande est refusée avant toute lecture, comme une ligne d'en-tête trop longue (1 Ko), trop d'en-têtes (16), un en-tête illisible ou un message qui n'est pas de l'UTF-8 : le flux n'est plus lu, le serveur est arrêté, c'est écrit dans son journal, et il est relancé comme après un plantage.

### Les journaux

- Chaque serveur a son journal, consultable : ce qu'il écrit sur sa sortie d'erreur, et ce que le cœur a fait (lancé, arrêté et pourquoi, relance dans N ms, échec, flux refusé). Les 2000 dernières lignes, chacune coupée à 4 Ko.

### Les réglages

- Réglages › Général › Avancé › « Serveurs de langage » : pour chaque serveur du catalogue, où il a été trouvé (« Trouvé : … (PATH) », « (node_modules du projet) »…) ou la commande qui l'installe ; un interrupteur « Allumé », son chemin, ses arguments (séparés par des espaces ; vides, ceux du catalogue).
- Un serveur qui exécute du code du projet le dit : « Ce serveur exécute du code du projet (build.rs, macros) : il ne se lance dans un projet qu'une fois que tu fais confiance au projet. ». Ce qui vient du projet le dit aussi. Pour un serveur qui attend la confiance, avec un projet choisi : « Faire confiance au projet », ou « Ne plus faire confiance ».
- Dessous, les serveurs qui tournent, leur état, leur journal et, pour un serveur arrêté après trop de plantages, « Relancer ».
- Un projet peut en imposer par `.escouade.yml` (`language_servers.<id>`, voir *.escouade.yml*) : `enabled` passe devant les réglages de l'app ; `path` et `args` attendent la confiance ; un chemin réglé à la main dans l'app passe devant `path`.
- Un réglage changé vaut pour le prochain lancement du serveur.
