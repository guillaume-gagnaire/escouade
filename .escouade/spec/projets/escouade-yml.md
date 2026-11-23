---
title: ".escouade.yml"
subject: "Projets"
sources: ["docs/SPEC.md","docs/PLUGINS.md","docs/STORE.md"]
tickets: ["ESC-146","ESC-163","ESC-156","ESC-160","ESC-167","ESC-288","ESC-309","ESC-446","ESC-481","ESC-493","ESC-530","ESC-560","ESC-561","ESC-653"]
decisions: [165,201,194,255,294,308]
compared: "2026-10-31T10:05:41+01:00"
---

## Ce que c'est

Les réglages d'un projet que partage son équipe, dans un fichier `.escouade.yml` à la racine du dépôt, commité avec le code. Escouade le lit à l'ouverture du projet et chaque fois qu'il change, et l'écrit à la demande en gardant ses commentaires, son ordre et ses clés inconnues. Ses commandes ne tournent qu'une fois approuvées, sur chaque machine.

## Comportement

### Contenu

Seulement ce qui vaut pour toute l'équipe. Clés en anglais, en `snake_case`, par thème :

| Clé | Valeur |
|---|---|
| `version` | entier, 1 aujourd'hui |
| `agents.engine` | le harness : `claude`, `codex`, `copilot`, `opencode` |
| `private` | « Projet privé » (voir plus bas) |
| `worktrees.per_agent` | un worktree par agent |
| `worktrees.at_first_message` | le worktree d'un nouvel agent naît à son premier message, à son nom ; vrai si absente |
| `worktrees.base` | d'où partent les nouveaux worktrees : `head` (la branche du dossier ; si absente), `target` (la branche cible du Kanban), ou le nom d'une branche, locale ou distante comme `origin/main` ; une branche nommée `head` ou `target` s'écrit `refs/heads/<nom>` |
| `worktrees.dir` | relatif au dépôt ; un chemin absolu, `~` compris, est refusé |
| `worktrees.copy` | motifs des fichiers copiés |
| `worktrees.setup`, `worktrees.teardown` | listes d'étapes : une commande seule, ou `command` avec `shell` et `cwd` facultatifs (`cwd` relatif) |
| `launch` | commandes de lancement : `name`, `command`, `shell`, `cwd` |
| `git.agent_branch_prefix`, `git.ticket_branch_prefix` | préfixes des branches |
| `git.commit_mode` | `agent`, `direct` |
| `git.conventional_commits` | commits conventionnels |
| `board.on_approve` | `merge`, `pr`, `push`, `keep` |
| `board.target_branch`, `board.draft_pr`, `board.remove_worktree` | branche cible, PR en brouillon, suppression du worktree |
| `board.merge_strategy` | `merge`, `squash`, `rebase` |
| `board.test_command`, `board.tests_before_approve` | commande de tests, tests avant la validation |
| `board.on_conflict` | `ask`, `agent`, `abort` |
| `board.model`, `board.effort`, `board.mode` | ceux des agents des tickets ; vides : ceux du compte. `model` : un texte, tel que le harness le nomme ; `effort` : `low`, `medium`, `high`, `xhigh`, `max` ; `mode` : `auto`, `default`, `plan`, `acceptEdits` |
| `workflows.enabled` | `false` : « Pas de workflow pour ce projet », ni lecture ni écriture de ses workflows (voir *Workflows*) ; vrai si absente |
| `memory.review` | « Valider les souvenirs des agents » d'un projet d'équipe (TS-10, C14-B) : `false` envoie les souvenirs des agents à l'équipe sans validation ; vrai si absente ; une équipe peut l'imposer (voir *La mémoire d'équipe*) |
| `plugins.recommended` | les plugins que le projet propose : des identifiants du store (`dev.exemple.slack`) ; voir *Plugins proposés* |
| `plugins.<id>` | les réglages que le projet donne au plugin `<id>` : des valeurs simples (texte, nombre, `true`, `false`), jamais un secret ; pour un réglage `profile`, le nom d'un profil qu'il **propose**, jamais un choix |
| `language_servers.<id>` | le serveur de langage `<id>` du catalogue (`typescript`, `rust`, `python`, `go`, `json`, `css`, `html`) : `enabled` (`true`, `false`), `path` (son programme, relatif au dépôt), `args` (ses arguments) ; voir *Serveurs de langage* |
| `editor.extensions_allowed` | les extensions de l'éditeur permises dans ce projet, comme `extensions.allowed` de VS Code : des clés `"*"` (entre guillemets : toutes), un éditeur d'Open VSX ou une extension (`éditeur.nom`), chacune `true`, `false`, `stable` ou une liste de versions ; voir *Extensions permises* |

Une valeur ou une étape seule, hors liste, vaut une liste d'un élément (`launch`, `worktrees.copy`, `worktrees.setup`, `worktrees.teardown`, `plugins.recommended`, `language_servers.<id>.args`).

L'ancienne clé `board.agents_from_target` est encore lue quand le fichier n'a pas `worktrees.base` (`true` vaut `target`, `false` vaut `head`), avec un avertissement à sa ligne. Elle n'est jamais écrite : l'export, qui écrit `worktrees.base`, la retire ; le schéma la garde, marquée `deprecated`.

### Jamais dedans

Chaque cas est tranché :

- le nom, la couleur, les lettres et le logo de la tuile : un choix d'affichage de chacun ;
- le compte préféré, les comptes réservés au projet privé et « Comptes partagés à la limite » : les comptes sont ceux de chacun, sur sa machine ;
- le remote control : il ouvre les sessions de quelqu'un à son propre claude.ai, un choix personnel ;
- en parallèle et le pilote auto : ils dépensent le quota de chacun, que l'équipe ne partage pas ; une personne qui ne fait que relire ne veut pas que des tickets démarrent seuls chez elle ;
- la clé des tickets et le numéro suivant : chaque machine a son Kanban et numérote ses tickets ; une clé partagée ferait croire à une numérotation commune, et elle est fixée par le premier ticket, que le fichier ne pourrait plus changer ;
- le mode `bypassPermissions` : un dépôt ne lève pas les autorisations de qui l'ouvre. Refusé à la lecture (« bypassPermissions ne se partage pas : il se règle sur chaque machine »), jamais écrit, gardé sur la machine ;
- les intégrations : liées aux comptes connectés de chacun ;
- les workflows supprimés d'un projet : retenus par chaque machine (voir *Workflows*) ;
- les chemins absolus, les jetons, les empreintes approuvées ; les secrets d'un plugin (ils vont au trousseau du système, par sa page) ; la confiance accordée au projet pour ses serveurs de langage (de chaque machine) ; les extensions installées et les éditeurs d'extensions approuvés (de chaque machine) ;
- le profil d'un plugin qu'un projet prend : un dépôt qui choisirait « le VPN de l'entreprise » ferait passer ses agents par le réseau de l'entreprise. Il peut en proposer un par son nom (voir *Plugins proposés*).

### Projet privé

- `private` est partagé : le dépôt est sensible pour toute l'équipe. Ses comptes réservés restent ceux de chaque machine.
- Le fichier peut rendre un projet privé, jamais le rendre public. `private: false` sur une machine où le projet est privé n'est pas appliqué : la clé est gardée sur cette machine (« Réglage local ») et c'est dit (« private: false ignoré : ce projet est privé sur cette machine, décoche « Projet privé » pour le rendre public »). Sinon ses agents et ses fichiers seraient vus des agents des autres projets sans que personne ne l'ait choisi ici.
- L'assistant suit la même règle : le point de départ « Réglages de .escouade.yml » est privé si le fichier ou « Nouveaux projets privés » le dit.

### Plugins proposés

- `plugins.recommended` nomme les plugins que le projet propose, comme les extensions recommandées de VS Code. Leur source est le store (store.escouade.dev) : un dépôt git ou un dossier ne se propose pas. Un identifiant qui n'en est pas un est dit avec sa ligne et ignoré, un doublon tombe.
- Escouade **propose**, n'installe ni n'active rien seul : un dépôt cloné ne doit pas pouvoir lancer un programme.
- À l'ouverture du projet (ajout, assistant, réouverture), un toast dit combien il en manque sur la machine : « demo propose 2 plugins : Réglages › Projets. ».
- En tête de Réglages › Projets, une bande « Ce projet propose N plugins » nomme ceux qui ne sont pas installés, chacun avec sa tuile, son nom et sa description du catalogue, son identifiant, ce que demande la version que l'app installerait (« Lance un programme · hooks.slack.com · Un secret »), ou « Absent du catalogue du store. ». « Voir dans le store » ferme les réglages et ouvre sa page du store dans la vue Plugins : l'installation reste le geste de l'utilisateur, plugin par plugin, après la fenêtre qui montre ses permissions. La bande attend de connaître les plugins installés avant de parler.
- Un plugin installé quitte la bande. « Pas pour ce projet » la fait taire pour les plugins qu'elle nommait, sur cette machine ; un plugin que le fichier proposerait plus tard la fait revenir, pour lui seul.
- Dans un projet privé, la bande ajoute qu'un plugin installé n'y est actif qu'une fois activé pour lui, dans sa page.
- `plugins.<id>` : les réglages que le projet donne au plugin. Quand le plugin les demande pour ce projet (`settings/get` avec `project`), ils passent devant ceux de l'utilisateur, mais seulement les clés qu'il déclare (`contributes.settings`), d'une valeur de leur type (un `select` parmi ses options), jamais l'un de ses secrets, ni un réglage que son auteur écarte (`"repo": false`) ou qui importe un fichier, et seulement s'il est actif dans ce projet : un projet privé ne donne rien à un plugin qui n'y est pas activé. Une valeur qui n'est pas simple (une liste, un groupe) est dite et ignorée.
- Pour un réglage de portée `project` (ESC-530), la valeur que cette machine a réglée pour le projet passe devant celle du fichier, elle-même devant le défaut du manifeste (voir *Les profils d'un plugin et ses réglages d'un projet*).
- Pour un réglage `profile`, la valeur du fichier n'est jamais prise : c'est le nom d'un profil que le projet **propose**. Réglages › Projets › Plugins le dit dans une bande (« .escouade.yml propose le profil « client-a » pour Tunnel. Un dépôt ne choisit pas tes identifiants : c'est toi qui choisis, ici, sur cette machine. ») et l'utilisateur choisit.
- Ces clés ne sont jamais appliquées au projet ni écrites par l'export, qui les garde telles qu'elles sont. La page Plugins d'un projet les marque (« Défini par .escouade.yml », « Réglage local de cette machine ») et écrit **une clé à la fois** (« Écrire dans .escouade.yml », voir *Dans la fenêtre*).

### Serveurs de langage

- `language_servers.<id>` règle pour le projet un serveur de langage de l'éditeur (voir *Serveurs de langage*) : `enabled` l'allume ou le coupe, devant les réglages de l'app ; `path` et `args` désignent son programme et ses arguments.
- Un serveur inconnu, un champ inconnu, un `path` absolu (ou `~`) sont dits avec leur ligne et ignorés ; un `path` qui sort du dépôt (`..`) ne mène à rien.
- `path` et `args` sont des choix du dépôt : ils ne servent qu'une fois que l'utilisateur fait confiance au projet pour ses serveurs, sur sa machine. Un chemin réglé à la main dans l'app passe devant `path`.
- Comme `plugins.*`, ces clés sont celles du fichier seul : jamais appliquées au projet, jamais écrites par l'export.

### Extensions permises

- `editor.extensions_allowed` restreint les extensions d'Open VSX de l'éditeur quand le projet est ouvert dans la vue VS Code (voir *Extensions*) : une liste de plus, que le cœur croise avec celles des réglages du poste, de l'équipe et du poste géré. Une extension doit être permise par chacune ; refusée, elle ne se télécharge pas, et une extension installée qu'elle exclut n'est plus chargée tant que le projet est dans la vue. Le refus nomme le fichier : « … n'est pas permise par le .escouade.yml de demo. ».
- Exemple :

  ```yaml
  editor:
    extensions_allowed:
      "*": false
      dracula-theme: true
      vscodevim.vim: stable
      acme.lang: [1.2.3]
  ```

- `*` doit être entre guillemets (sans eux, YAML le lit comme un alias). Une clé qui n'est ni `*`, ni un éditeur, ni une extension, ou une valeur qui n'est ni `true`, ni `false`, ni `stable`, ni une liste de versions, est dite avec sa ligne et ignorée.
- Comme `plugins.*`, c'est une clé du fichier seul : jamais appliquée au projet, jamais écrite par l'export.

### Versions et erreurs

- Le format a un lecteur par version et une migration de chaque version à la suivante (aucune encore).
- Sans `version`, ou avec une valeur illisible, le fichier est lu comme la version 1, et c'est dit. Une version plus récente que celle de l'app est lue au mieux : « fait pour une version plus récente d'Escouade (version N du fichier, 1 connue ici) : lu au mieux ».
- Une clé inconnue est ignorée et dite avec sa ligne et la clé connue qui lui ressemble (proche par l'orthographe, ou le même nom dans un autre groupe) : « clé inconnue : worktrees.per_agnet (voulais-tu dire worktrees.per_agent ?), ignorée ».
- Une valeur invalide est ignorée et dite avec sa ligne et ce qui est attendu : « valeur invalide pour board.merge_strategy : « fast » (attendu : merge, squash, rebase) (ignorée) ». Une étape illisible d'une liste est seule ignorée.
- Un booléen entre guillemets n'est pas un booléen (YAML 1.2 : `yes` non plus).
- Un fichier qui n'est pas du YAML, ou pas une suite de clés, n'applique rien et le dit avec sa ligne.

### Schéma

- Un JSON Schema (draft-07), `schemas/escouade.schema.json` dans le dépôt d'Escouade, publié à `https://escouade.dev/schemas/escouade.schema.json`. Il est fait depuis la même liste de clés que le lecteur et l'écriture : il ne peut pas en diverger. Les réglages d'un plugin y sont un groupe par identifiant (`patternProperties`), de valeurs simples ; ceux des serveurs de langage, un groupe par serveur du catalogue ; `editor.extensions_allowed`, un objet de booléens, de `stable` ou de listes.
- Le fichier écrit par Escouade commence par `# yaml-language-server: $schema=…`, pour l'autocomplétion des éditeurs.

### À l'ouverture

- À l'ajout d'un dossier, dans l'assistant et à la réouverture d'un projet, le fichier est lu avant que le dossier ne soit surveillé, puis appliqué, et un toast le dit : « demo : réglages lus dans .escouade.yml. », ou « … Ses commandes attendent ton accord (Réglages › Projets). ». S'il propose des plugins absents de la machine, un toast le dit aussi (voir *Plugins proposés*).
- L'assistant le lit sans rien lancer : le point de départ « Réglages de .escouade.yml » (« Réglages lus dans .escouade.yml, le fichier partagé du dépôt : N réglages. », plus « Ses commandes attendront ton accord une fois le projet créé. ») apparaît au-dessus de la copie et de « Configuration recommandée », choisi par défaut tant que l'utilisateur n'en a pas choisi un autre (voir *Nouveau projet*).

### Relu quand il change

- À chaque passage de la surveillance du dossier (après un `git pull`, un changement de branche, une modification), le fichier est relu s'il a changé (l'empreinte de son texte n'est jamais enregistrée : il est relu à chaque démarrage). Les réglages changés s'appliquent : « demo : .escouade.yml relu, 2 réglages appliqués. ». Un `workflows.enabled` changé est suivi tout de suite : les agents ne sont plus suivis dans les workflows, ou le sont à nouveau.
- Un fichier supprimé laisse au projet ses réglages ; plus rien n'est marqué.

### Commandes : l'accord de chaque machine

- Un dépôt cloné peut venir de n'importe qui. `worktrees.setup`, `worktrees.teardown`, `launch` et `board.test_command` ne s'appliquent au projet qu'une fois approuvées : avant, le projet garde les siennes et rien du fichier ne tourne. La lecture ne lance jamais rien.
- Leur empreinte (SHA-256 de leurs valeurs) est comparée à celle approuvée, enregistrée avec le projet : différente, elles attendent. Des commandes que le projet a déjà telles quelles (le fichier exporté depuis cette machine) n'ont rien à approuver.
- Réglages › Projets › « .escouade.yml » les montre en entier (commande, shell, dossier), avec « Faire confiance à ces commandes ». Refusé si le fichier a changé depuis : « Les commandes de .escouade.yml ont changé depuis : relis-les. ».
- Le Kanban du projet le dit sous son en-tête : « Les commandes de .escouade.yml attendent ton accord : elles ne tournent pas encore. », bouton « Les lire ».
- Des commandes changées dans le fichier redemandent l'accord (toast) ; les approuvées restent en place jusque-là.

### Dans la fenêtre

- Chaque réglage que le fichier définit le dit sous son libellé, dans Réglages › Projets (« Projet privé » et « Workflows du projet » compris), Réglages › Kanban (modèle, effort et mode compris ; ni « En parallèle » ni le pilote auto) et Réglages › Projets › Plugins (les réglages de portée `project` d'un plugin) : « Défini par .escouade.yml », ou « Défini par .escouade.yml, en attente de ton accord » pour une commande.
- Le modifier dans la fenêtre et enregistrer le garde sur cette machine : il passe devant le fichier, relu ou pas, et c'est dit (un toast, et sous le réglage « Réglage local de cette machine : il passe devant .escouade.yml. »), avec « Écrire dans .escouade.yml » et « Reprendre la valeur du fichier ».
- Sur la page Plugins d'un projet, « Écrire dans .escouade.yml » écrit la seule clé `plugins.<id>.<clé>` de ce réglage, en gardant les commentaires, son ordre et les clés inconnues du fichier, puis oublie la valeur de cette machine. Un réglage que son auteur écarte (`"repo": false`), qui importe un fichier, un profil ou un secret dit « Gardé sur cette machine, jamais dans .escouade.yml. » et ne s'écrit pas.
- « Relancer les tests avant », éteint par la fenêtre faute de commande de tests (celle du fichier attend l'accord), n'est pas un choix : la valeur du fichier reste.

### Exporter

- Réglages › Projets › « Exporter vers .escouade.yml » écrit toutes les clés avec les réglages enregistrés du projet (une branche cible vide est omise, le mode `bypassPermissions` aussi, et les clés `plugins.*`, `language_servers.*` et `editor.extensions_allowed`, qui ne sont pas des réglages du projet : un réglage d'un plugin s'écrit une clé à la fois, depuis la page Plugins du projet) : par-dessus le fichier existant, en gardant ses commentaires, son ordre et ses clés inconnues ; sinon un nouveau fichier, avec la ligne du schéma, un commentaire qui renvoie à la doc et `version: 1`.
- Les commandes écrites sont celles du projet : approuvées d'office.
- « Écrire dans .escouade.yml » d'un mode `bypassPermissions` gardé sur la machine n'écrit rien et le garde.
- Un fichier illisible ou d'une version plus récente n'est pas réécrit : « modifie-le à la main ».

### La clé `team` (TS-7)

```yaml
team:
  server: https://team.acme.fr
  id: 0123456789abcdef0123456789abcdef      # l'id de l'équipe sur son serveur
  project: ffffffffffffffffffffffffffffffff # l'id du projet dans la liste de l'équipe, s'il y est
  name: Acme                                 # le nom de l'équipe, pour la bande
```

- **Ce qu'elle dit** : à quelle équipe le projet appartient. Rien de secret : ni lien d'invitation, ni jeton, ni clé. Elle est du fichier seul (elle ne s'applique à aucun réglage du projet) et figure au schéma.
- **Écrite au rattachement** (« Rattacher à l'équipe », la case de l'écran de l'équipe, le clone depuis la liste), au-dessus du reste du fichier gardé tel quel ; un toast dit de la commiter, Escouade ne commite rien. Retirée quand le projet quitte l'équipe qu'elle nomme.
- **Lue à l'ouverture** et à chaque relecture du fichier. Elle vient du dépôt, donc **rien d'elle n'est de confiance** :
  - l'adresse est vérifiée comme une adresse de serveur tapée dans l'app : `https://` (`http://` seulement vers cette machine), sans chemin, sans requête, sans fragment, sans utilisateur ni mot de passe ; un hôte d'autres lettres s'affiche dans sa forme punycode (`xn--…`). Une adresse ou un id qui ne passe pas ne montre aucune bande, et l'avertissement du fichier le dit ;
  - le nom est nettoyé (caractères de contrôle, de direction du texte comme `U+202E`, invisibles ; 60 caractères au plus) et la bande dit qu'il vient du dépôt ; sans nom, l'hôte ;
  - **aucune requête ne part vers cette adresse** : ni à la lecture, ni en montrant la bande, ni par « Demander une invitation » (vérifié par un test avec un serveur qui compte ses requêtes).
- **La bande**, en haut de la fenêtre quand le projet est à l'écran :
  - sur un poste qui n'est pas dans cette équipe (un autre id, ou le même id sur un autre serveur, ou une équipe pas encore rejointe) : « Ce projet appartient à l'équipe Acme sur team.acme.fr, d'après le dépôt. », « Ce nom et cette adresse viennent du fichier .escouade.yml du dépôt : Escouade ne joint ce serveur qu'à ton geste. », **« Demander une invitation »** (ouvre l'écran « Rejoindre une équipe » avec « D'après le dépôt, l'équipe est sur team.acme.fr : demande un lien d'invitation à un de ses administrateurs, puis colle-le ici. » ; un lien collé vers un autre serveur le fait dire) ;
  - sur un poste de cette équipe : « Ce projet est de ton équipe Acme, d'après le dépôt. », **« Rattacher à l'équipe »** (le rattachement de la page « Les projets de l'équipe » : le Kanban de la liste est rejoint seulement quand un remote du projet est un dépôt de la liste, jamais sur la seule foi de l'id du fichier) ;
  - aucune sur un projet déjà rattaché à cette équipe ;
  - **« Ne plus proposer pour ce projet »** : gardé sur ce poste (l'empreinte du serveur et de l'équipe nommés) ; un fichier qui nomme ensuite une autre équipe ou un autre serveur montre la bande de nouveau.
- **Un dépôt hostile** ne fait rien de plus qu'afficher cette bande : il ne branche le poste sur aucun serveur, n'ajoute le projet à aucune équipe, ne fait rejoindre aucun Kanban.
- Les outils `list_projects` et `get_board` du serveur MCP disent `team.namedByRepository` (l'hôte) quand le fichier nomme une équipe à laquelle le projet n'est pas rattaché.

### L'adresse d'une commande de lancement (ESC-560)

- Une commande de `launch` a aussi `url` et `open_in`, facultatives : l'adresse ouverte quand l'app lancée répond, et où (`page`, la page d'un lancement, par défaut ; `browser`, le navigateur du système). Les clés de la table ci-dessus deviennent `name`, `command`, `shell`, `cwd`, `url`, `open_in`.
- `url` est une adresse `http://` ou `https://` ; elle peut nommer `${ESCOUADE_PORT_BASE}` et `${ESCOUADE_PORT_END}`, avec une valeur pour le projet après `:-`. Une autre adresse, une autre variable, ou un `open_in` autre que `page` et `browser` est dit avec sa ligne et la commande est ignorée.
- Elles comptent dans l'empreinte des commandes : une adresse changée redemande l'accord, et elle n'est appliquée qu'avec sa commande, au geste de l'utilisateur. L'export écrit `url` quand il y en a une et `open_in: browser` seulement (`page` est le défaut). Le schéma les décrit.

```yaml
launch:
  - name: web
    command: npm run dev -- --port ${ESCOUADE_PORT_BASE:-5173}
    cwd: web
    url: http://localhost:${ESCOUADE_PORT_BASE:-5173}/
```

### Le fetch automatique d'un projet (ESC-561)

Trois clés de plus dans le groupe `git`, après `git.conventional_commits` (voir *Fetch automatique*) :

| Clé | Valeurs |
|---|---|
| `git.auto_fetch` | `default` (le réglage général de chaque machine ; si absente), `always`, `never` |
| `git.fetch_every` | minutes entre deux fetch de fond, de `1` à `1440` ; `0` (si absente) : celles du réglage général |
| `git.fetch_timeout` | secondes qu'un fetch de fond peut prendre, de `30` à `1800` ; `0` (si absente) : celles du réglage général |

- Un nombre entier, écrit nu ou entre guillemets. Hors de ses bornes, il est ramené à la plus proche et le fichier le dit (« git.fetch_every : 5000 est hors des bornes (1 à 1440), ramené à 1440 ») ; ce qui n'est pas un nombre entier est refusé et laissé de côté, comme toute valeur invalide.
- Le schéma (`schemas/escouade.schema.json`) les dit : un entier entre les bornes, ou `0`.

## Un fichier venu du dépôt (ESC-653)

- `.escouade.yml` n'est lu que s'il est un fichier ordinaire de 1 Mio au plus : un lien (une jonction sous Windows en est un), un tube nommé ou un fichier plus gros n'est pas lu, et le journal le dit. Un lien mis à sa place pendant la lecture n'est pas suivi non plus, sous Windows comme ailleurs.
- Il n'est jamais écrit à travers un lien, ni par-dessus un fichier qu'il ne lit pas (trop gros) : l'export et « Écrire dans .escouade.yml » le refusent.
- Sa lecture prend un temps proportionnel à sa taille : un gros fichier ne fige plus l'app.
