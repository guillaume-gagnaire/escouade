# .escouade.yml

Un fichier `.escouade.yml` à la racine du dépôt porte les réglages du projet que toute l’équipe partage. Cette page est la référence de son format : chaque clé, ses valeurs, ses règles de lecture, et ce qu’Escouade en fait.

## À quoi il sert

Le fichier se commite avec le code. Chaque personne qui ajoute le dépôt dans Escouade retrouve la même façon de travailler : un worktree par agent, les mêmes préfixes de branches, les mêmes commandes de préparation, la même validation des tickets. Escouade le lit à l’ouverture du projet, le relit quand il change, et sait l’écrire depuis un projet déjà réglé.

## Un exemple complet

```yaml
# yaml-language-server: $schema=https://escouade.dev/schemas/escouade.schema.json
version: 1
agents:
  engine: claude
private: false
worktrees:
  per_agent: true
  at_first_message: true
  base: target
  dir: ../app-worktrees
  copy:
    - '.env*'
    - config/local.json
  setup:
    - npm ci
    - command: cargo fetch
      cwd: src-tauri
  teardown:
    - command: docker compose down
      shell: bash
launch:
  - name: web
    command: npm run dev
    cwd: web
  - name: api
    command: cargo run
    cwd: api
git:
  agent_branch_prefix: agent/
  ticket_branch_prefix: ticket/
  commit_mode: agent
  conventional_commits: true
board:
  on_approve: merge
  target_branch: main
  merge_strategy: squash
  draft_pr: false
  test_command: npm test
  tests_before_approve: true
  on_conflict: agent
  remove_worktree: true
  model: sonnet
  effort: medium
  mode: auto
```

Un fichier minimal ne dit que ce qu’il veut régler :

```yaml
version: 1
worktrees:
  per_agent: true
board:
  merge_strategy: rebase
```

## Les règles du format

### Le style des clés

Les clés sont en anglais, en `snake_case` (mots en minuscules séparés par `_`), rangées en groupes par thème : `agents`, `worktrees`, `git` et `board` regroupent leurs clés en retrait ; `version` et `launch` sont au premier niveau. L’ordre des groupes et des clés est libre. Le fichier est du YAML 1.2 : un `#` commence un commentaire, et les commentaires sont gardés quand Escouade réécrit le fichier.

### La clé version

`version` est le numéro du format du fichier, un entier : `1` aujourd’hui. Escouade l’écrit en tête de chaque fichier qu’il crée. Elle n’est pas obligatoire : sans elle, le fichier est lu comme la version 1, et le groupe « .escouade.yml » de Réglages › Projets le signale (« pas de clé « version » : lu comme la version 1 »). Une valeur qui n’est pas un entier d’au moins 1 (`one`, `'1'`, `0`) est signalée de la même façon, et le fichier est lu comme la version 1.

### Une version plus ancienne ou plus récente

Le format peut évoluer : chaque version d’Escouade sait lire celles qu’elle connaît. Un fichier d’une version plus ancienne est migré vers la version actuelle à la lecture, d’une version à la suivante, sans être réécrit sur le disque : il reste valable tel quel. Un fichier d’une version plus récente que celle qu’Escouade connaît est lu au mieux : les clés connues s’appliquent, les autres sont signalées comme inconnues, avec l’avertissement « fait pour une version plus récente d’Escouade (version 2 du fichier, 1 connue ici) : lu au mieux ». Escouade ne réécrit pas un tel fichier : « Exporter vers .escouade.yml » le refuse.

### Une clé absente

Toutes les clés sont facultatives, `version` comprise. Une clé absente ne change rien : le projet garde sa valeur. La colonne « Absente » des tableaux ci-dessous donne celle d’un projet neuf, la valeur par défaut d’Escouade. Retirer une clé du fichier ne remet pas le projet à cette valeur : il garde la dernière que le fichier lui a donnée.

### Une clé inconnue

Une clé qu’Escouade ne connaît pas (une faute de frappe, ou une clé d’une version plus récente) n’empêche pas la lecture : elle est ignorée et signalée avec sa ligne. Quand elle ressemble à une clé connue du même groupe, ou porte le nom d’une clé d’un autre groupe, Escouade propose la bonne :

- `worktrees.per_agnet` : « clé inconnue : worktrees.per_agnet (voulais-tu dire worktrees.per_agent ?), ignorée » ;
- `worktrees.test_command` : « clé inconnue : worktrees.test_command (voulais-tu dire board.test_command ?), ignorée » ;
- un groupe inconnu, `bord:` par exemple, est signalé de la même façon (« voulais-tu dire board ? »), et tout ce qu’il contient est ignoré.

### Une valeur invalide

Une valeur du mauvais type ou hors des valeurs permises est ignorée et signalée avec sa ligne et ce qui est attendu ; le reste du fichier s’applique : « valeur invalide pour board.merge_strategy : « fast » (attendu : merge, squash, rebase) (ignorée) ». Dans une liste d’étapes ou de commandes, seule l’entrée invalide est ignorée, les autres restent. Un groupe qui n’est pas une suite de clés (`board: true`) est ignoré entier.

Un fichier qui n’est pas du YAML lisible (un crochet non fermé, une indentation cassée), ou dont le premier niveau n’est pas une suite de clés, n’applique rien : le message dit pourquoi et à quelle ligne.

## Les types de valeurs

| Type | Forme YAML | Exemple |
|---|---|---|
| Booléen | `true` ou `false`, sans guillemets (la casse est libre) | `per_agent: true` |
| Texte | Une valeur seule, avec ou sans guillemets ; `''` ou `~` vaut un texte vide | `target_branch: main` |
| Valeur parmi une liste | Un texte, exactement l’une des valeurs permises | `merge_strategy: squash` |
| Liste de textes | Une liste YAML, en lignes `- …` ou entre crochets ; une valeur seule vaut une liste d’un élément ; `[]` ou `~` vaut une liste vide | `copy: config/local.json` |
| Chemin relatif | Un texte, relatif au dépôt ; un chemin complet (`/…`, `\…`, `C:…`) ou qui part du dossier personnel (`~…`) est refusé | `dir: ../app-worktrees` |
| Préfixe de branche | Un texte qui peut commencer un nom de branche git : ni espace, ni `..`, `~`, `^`, `:`, `?`, `*`, `[`, `\` ou `@{`, ni `-` ou `/` au début, ni `//` | `agent_branch_prefix: agent/` |

`'true'` entre guillemets est un texte, pas un booléen, et `yes`, `no`, `on`, `off` ne sont pas des booléens (YAML 1.2). Un nombre est lu comme un texte là où un texte est attendu (`target_branch: 2024` donne la branche `2024`). Un motif qui commence par `*` s’écrit entre guillemets (`'*.local'`) : sans eux, YAML y lit un alias.

## Les clés

### agents

| Clé | Type | Valeurs | Absente | Exemple |
|---|---|---|---|---|
| `agents.engine` | Valeur parmi une liste | `claude`, `codex`, `copilot`, `opencode` | `claude` | `engine: codex` |

Le harness des nouveaux agents et des tickets du projet qui n’en choisissent pas un autre (« Harness par défaut »).

### private

| Clé | Type | Valeurs | Absente | Exemple |
|---|---|---|---|---|
| `private` | Booléen | `true`, `false` | le réglage de chaque machine | `private: true` |

« Projet privé » : les agents des autres projets ne le voient pas par le serveur MCP d’Escouade et ne lisent pas ses fichiers (voir [Réglages › Projets](settings-projects)). Le dépôt est sensible pour toute l’équipe, d’où sa place dans le fichier ; les comptes réservés au projet, eux, restent ceux de chaque machine.

Le fichier peut rendre un projet privé, jamais le rendre public : sur une machine où le projet est privé, `private: false` n’est pas appliqué, le réglage devient un réglage local de cette machine et un avertissement le dit (« private: false ignoré : ce projet est privé sur cette machine, décoche « Projet privé » pour le rendre public »). Pour le rendre public, décoche « Projet privé » toi-même.

### worktrees

| Clé | Type | Valeurs | Absente | Exemple |
|---|---|---|---|---|
| `worktrees.per_agent` | Booléen | `true`, `false` | `false` | `per_agent: true` |
| `worktrees.at_first_message` | Booléen | `true`, `false` | `true` | `at_first_message: false` |
| `worktrees.base` | Texte | `head`, `target` ou un nom de branche | `head` | `base: origin/release` |
| `worktrees.dir` | Chemin relatif | Un dossier relatif au dépôt | vide : `.claude/worktrees` | `dir: .worktrees` |
| `worktrees.copy` | Liste de textes | Des motifs de fichiers (`*`, `**`) | `['.env*']` | `copy: ['.env*', config/local.json]` |
| `worktrees.setup` | Liste d’étapes | Voir [les étapes de worktree](#les-etapes-de-worktree) | aucune | `setup: npm ci` |
| `worktrees.teardown` | Liste d’étapes | Voir [les étapes de worktree](#les-etapes-de-worktree) | aucune | `teardown: [docker compose down]` |

- `per_agent` : « Un worktree par agent », chaque nouvel agent dans un worktree à lui (voir [les worktrees](worktrees)).
- `at_first_message` : « Créer le worktree au premier message, au nom de l’agent ». Le worktree d’un nouvel agent attend son premier message, qui le nomme : sa branche et son dossier prennent ce nom (voir [au premier message](worktrees#au-premier-message)). `false` le crée tout de suite, en `agent-N`.
- `base` : « Branche de départ des nouveaux worktrees », la branche dont part le worktree d’un nouvel agent, sa base : `head` (la branche du dossier), `target` (la branche cible du Kanban) ou le nom d’une branche, locale ou distante (`origin/release`). Une branche qui s’appelle `head` ou `target` s’écrit `refs/heads/target`. Voir [la branche de départ](worktrees#la-branche-de-depart).
- `dir` : « Dossier des worktrees ». Il ne peut pas être le dossier du projet, un dossier qui le contient, ni un dossier dans `.git` : un tel chemin est signalé et ignoré.
- `copy` : « Fichiers copiés dans les worktrees », les fichiers non suivis par git copiés dans chaque nouveau worktree.
- `setup` : « À l’ouverture d’un worktree », lancées l’une après l’autre dans chaque nouveau worktree, avant le premier message de son agent. Des commandes : elles attendent ton accord.
- `teardown` : « Avant sa suppression », lancées dans un worktree avant qu’Escouade le supprime. Des commandes : elles attendent ton accord.

### launch

| Clé | Type | Valeurs | Absente | Exemple |
|---|---|---|---|---|
| `launch` | Liste de commandes de lancement | Voir [les commandes de lancement](#les-commandes-de-lancement) | aucune | `- name: web` puis `command: npm run dev` |

Les commandes de « Lancement » du projet (serveurs de développement, watchers), chacune dans son terminal (voir [le lancement](launch)). Des commandes : elles attendent ton accord.

### git

| Clé | Type | Valeurs | Absente | Exemple |
|---|---|---|---|---|
| `git.agent_branch_prefix` | Préfixe de branche | Un début de nom de branche | `escouade/` | `agent_branch_prefix: agent/` |
| `git.ticket_branch_prefix` | Préfixe de branche | Un début de nom de branche | `ticket/` | `ticket_branch_prefix: tickets/` |
| `git.commit_mode` | Valeur parmi une liste | `agent`, `direct` | `agent` | `commit_mode: direct` |
| `git.conventional_commits` | Booléen | `true`, `false` | `true` | `conventional_commits: false` |

- `agent_branch_prefix` : la branche d’un nouvel agent s’appelle `<préfixe><nom>`. Vide (`''`), c’est son nom seul.
- `ticket_branch_prefix` : la branche d’un ticket s’appelle `<préfixe><clé>` (`ticket/DEM-12`).
- `commit_mode` : « Commit », qui écrit les commits demandés depuis le panneau des fichiers : `agent` (« Rédigé par l’agent ») ou `direct` (« Direct, avec un message proposé »).
- `conventional_commits` : « Message de commit généré », les messages au format Conventional Commits.

### board

| Clé | Type | Valeurs | Absente | Exemple |
|---|---|---|---|---|
| `board.on_approve` | Valeur parmi une liste | `merge`, `pr`, `push`, `keep` | `merge` | `on_approve: pr` |
| `board.target_branch` | Texte | Un nom de branche | vide : la branche du dossier au premier ticket | `target_branch: develop` |
| `board.merge_strategy` | Valeur parmi une liste | `merge`, `squash`, `rebase` | `squash` | `merge_strategy: rebase` |
| `board.draft_pr` | Booléen | `true`, `false` | `false` | `draft_pr: true` |
| `board.test_command` | Texte | Une commande | vide : aucune | `test_command: npm test` |
| `board.tests_before_approve` | Booléen | `true`, `false` | `false` | `tests_before_approve: true` |
| `board.on_conflict` | Valeur parmi une liste | `ask`, `agent`, `abort` | `ask` | `on_conflict: agent` |
| `board.remove_worktree` | Booléen | `true`, `false` | `true` | `remove_worktree: false` |
| `board.model` | Texte | Un modèle, tel que le harness le nomme | vide : celui du compte | `model: opus` |
| `board.effort` | Valeur parmi une liste | `low`, `medium`, `high`, `xhigh`, `max` | vide : celui du compte | `effort: high` |
| `board.mode` | Valeur parmi une liste | `auto`, `default`, `plan`, `acceptEdits` | vide : celui du compte | `mode: plan` |

- `on_approve` : « Quand je valide un ticket « À tester » » : merger sa branche dans la branche cible (`merge`), ouvrir une pull request (`pr`), pousser sa branche (`push`), ou la garder telle quelle (`keep`) (voir [le Kanban](board)).
- `target_branch` : « Branche cible », où les tickets se mergent, et la base de leur branche et de leur pull request.
- `agents_from_target` : l’ancienne clé de `worktrees.base`, encore lue quand le fichier n’a pas `worktrees.base` (`true` vaut `target`, `false` vaut `head`), avec un avertissement. « Exporter vers .escouade.yml » la remplace par `worktrees.base`.
- `merge_strategy` : « Stratégie » du merge : commit de merge, squash ou rebase.
- `draft_pr` : « PR en brouillon ».
- `test_command` : « Commande de tests ». Une commande : elle attend ton accord.
- `tests_before_approve` : « Relancer les tests avant » la validation d’un ticket ; sans commande de tests, rien ne se relance.
- `on_conflict` : « En cas de conflit » au merge : te demander (`ask`), laisser l’agent le résoudre (`agent`), ou abandonner (`abort`).
- `remove_worktree` : « Supprimer le worktree d’un ticket terminé ou supprimé ».
- `model`, `effort`, `mode` : le modèle, l’effort et le mode des agents des tickets (Réglages › Kanban, groupe « Agents »). Une valeur vide (`model: ''`) laisse ceux du compte. `bypassPermissions` n’est jamais dans le fichier : un dépôt ne lève pas les autorisations de qui l’ouvre. Lu, il est refusé (« bypassPermissions ne se partage pas : il se règle sur chaque machine ») ; réglé dans la fenêtre, il reste un réglage local, que l’export n’écrit pas.

## Les étapes de worktree

Chaque élément de `worktrees.setup` et de `worktrees.teardown` est une étape, de l’une de ces deux formes :

```yaml
setup:
  # La commande seule, en texte : lancée par le shell par défaut, à la racine du worktree.
  - npm ci
  # Ou ses champs :
  - command: cargo fetch
    shell: bash
    cwd: src-tauri
```

| Champ | Obligatoire | Ce qu’il dit |
|---|---|---|
| `command` | oui | La ligne de commande, telle que le shell la lit. |
| `shell` | non | Le shell qui la lance (voir [les shells](#les-shells)). Absent : le shell par défaut du système. |
| `cwd` | non | Un sous-dossier du worktree, relatif. Absent ou `.` : la racine du worktree. |

Une étape sans `command`, ou dont `cwd` est un chemin complet, est signalée et ignorée ; un autre champ (`env:`, `name:`) est signalé comme clé inconnue. Les étapes tournent dans l’ordre du fichier. Une étape seule peut s’écrire sans liste : `setup: npm ci` vaut une liste d’une étape.

## Les commandes de lancement

Chaque élément de `launch` est une commande nommée :

```yaml
launch:
  - name: web
    command: npm run dev -- --port 5173
    cwd: web
  - name: docs
    command: npm run docs
    shell: pwsh
```

| Champ | Obligatoire | Ce qu’il dit |
|---|---|---|
| `name` | non | Le nom affiché de la commande et de son terminal. Absent : la commande sert de nom. |
| `command` | oui | La ligne de commande. |
| `shell` | non | Le shell qui la lance. Absent : le shell par défaut du système. |
| `cwd` | non | Un sous-dossier du projet, relatif. Absent : la racine du projet. |

Contrairement aux étapes, une commande de lancement n’a pas de forme courte : un texte seul est signalé comme valeur invalide. Une commande seule peut s’écrire sans liste (`launch:` suivi directement de `name:` et `command:` en retrait) : elle vaut une liste d’une commande.

## Les shells

`shell` prend l’identifiant d’un shell qu’Escouade détecte sur la machine :

| Système | Identifiants |
|---|---|
| Windows | `pwsh` (PowerShell 7), `powershell` (Windows PowerShell), `bash` (Git Bash), `wsl` |
| macOS et Linux | `zsh`, `bash`, `fish`, `sh` |

Un shell absent de la machine qui lit le fichier (`pwsh` sur un Mac, `zsh` sous Windows) n’est pas une erreur : la commande tourne dans le shell par défaut de ce système. Pour un fichier partagé entre systèmes, omets `shell` et écris des commandes que tous les shells comprennent.

## Le schéma JSON et l’autocomplétion

Le schéma JSON du format est publié à l’adresse `https://escouade.dev/schemas/escouade.schema.json`. Il suit chaque version d’Escouade : il décrit chaque clé, son type et ses valeurs permises.

Pour l’autocomplétion, mets en tête du fichier la ligne que les fichiers écrits par Escouade ont déjà :

```yaml
# yaml-language-server: $schema=https://escouade.dev/schemas/escouade.schema.json
```

Les éditeurs qui lisent les schémas YAML (VS Code avec l’extension YAML de Red Hat, les IDE JetBrains, Neovim avec `yaml-language-server`) complètent alors les clés, montrent leur description au survol, et signalent une clé inconnue ou une valeur hors liste pendant la frappe.

## Ce qui reste hors du fichier

Le fichier est commité et lu par toute l’équipe, et un dépôt public le montre à n’importe qui. Il ne porte donc que ce qui vaut pour tout le monde, jamais ce qui est propre à une personne, à une machine, ou secret :

- le nom et la couleur du projet : un choix d’affichage de chacun, dans sa propre barre d’onglets ;
- le compte préféré, les comptes réservés à un projet privé et « Comptes partagés à la limite » : chacun a ses comptes ;
- le remote control : un choix personnel, qui rend ses agents accessibles depuis ton claude.ai ;
- « En parallèle » et le pilote auto : ils dépensent ton quota ; une personne qui ne fait que relire ne veut pas que des tickets démarrent seuls chez elle ;
- le mode `bypassPermissions` des agents des tickets : un dépôt ne lève pas les autorisations de qui l’ouvre ;
- la clé des tickets : chaque machine a son Kanban et numérote ses propres tickets, et la clé est fixée par le premier ticket ;
- les intégrations (Jira, Trello, GitHub, Linear) : elles tiennent à tes comptes connectés ;
- les chemins complets : ils ne valent que sur une machine (d’où le refus d’un `dir` ou d’un `cwd` complet) ;
- les jetons et mots de passe : aucune clé n’en prend, et « Exporter vers .escouade.yml » n’en écrit jamais.

## À l’ouverture du projet

Quand le dossier ajouté a un `.escouade.yml`, l’assistant « Nouveau projet » propose « Réglages de .escouade.yml » comme point de départ, au-dessus de « Configuration recommandée », et le choisit tout seul. Une fois le projet créé (ou rouvert), Escouade applique le fichier et le dit : « réglages lus dans .escouade.yml ».

Le fichier est relu dès qu’il change : après un `git pull`, un changement de branche ou une modification à la main. Les réglages changés s’appliquent et un message le dit.

## Les commandes attendent ton accord

Un dépôt cloné peut venir de n’importe qui. Les commandes du fichier (`worktrees.setup`, `worktrees.teardown`, `launch` et `board.test_command`) ne tournent jamais avant ton accord, et la lecture du fichier n’en lance aucune. Tant que tu ne les as pas approuvées, le projet garde les siennes, et le Kanban affiche « Les commandes de .escouade.yml attendent ton accord : elles ne tournent pas encore. ».

« Les lire » ouvre Réglages › Projets, groupe « .escouade.yml », qui montre chaque commande en entier, avec son shell et son dossier. « Faire confiance à ces commandes » les applique. Si le fichier les change ensuite (une commande, son shell ou son dossier, une commande ajoutée ou retirée), Escouade te redemande ton accord ; d’ici là, celles que tu avais approuvées restent en place. Des commandes identiques à celles que le projet a déjà n’ont rien à approuver.

> [!WARNING]
> Lis chaque commande avant de lui faire confiance : la préparation tourne dans chaque nouveau worktree, avant le premier message de son agent.

## Les réglages marqués

Dans Réglages › Projets et Réglages › Kanban, un réglage défini par le fichier le dit sous son libellé : « Défini par .escouade.yml ». Le modifier et enregistrer en fait un réglage local de cette machine, qui passe devant le fichier, même quand il est relu : « Réglage local de cette machine : il passe devant .escouade.yml. ». Sous le réglage, « Écrire dans .escouade.yml » met ta valeur dans le fichier (à commiter), et « Reprendre la valeur du fichier » rend la main au fichier.

## Exporter les réglages d’un projet

Pour partager un projet déjà réglé, Réglages › Projets › « Exporter vers .escouade.yml » écrit le fichier à la racine du dépôt depuis les réglages enregistrés du projet : toutes les clés, dans l’ordre de l’exemple complet (une branche cible encore vide est omise, le mode `bypassPermissions` aussi). Un fichier existant garde ses commentaires, l’ordre de ses clés et les clés qu’Escouade ne connaît pas ; un nouveau fichier commence par la ligne du schéma et `version: 1`. Les commandes exportées sont celles du projet : elles n’ont pas besoin de ton accord. Il reste à commiter le fichier.
