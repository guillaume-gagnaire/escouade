# Le manifeste

Chaque plugin d’Escouade se décrit dans un fichier escouade-plugin.json à la racine de son dossier. Cette page en est la référence : chaque champ, ce qu’Escouade vérifie, comment il installe un plugin et pourquoi il en désactive un.

## Un exemple complet

Le plugin Discord, l’un des [plugins d’exemple](plugins/examples), en entier : un plugin sans programme, qui garde un secret et envoie deux webhooks.

`plugins/discord/escouade-plugin.json`

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.escouade.discord",
  "name": "Discord",
  "version": "1.1.2",
  "description": {
    "fr": "Les tickets à tester et les agents qui attendent, dans un salon Discord : un webhook, sans code.",
    "en": "Tickets to test and agents waiting, in a Discord channel: a webhook, no code."
  },
  "author": { "name": "Escouade", "url": "https://escouade.dev" },
  "license": "MIT",
  "repository": "https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/discord",
  "escouade": { "api": "1", "min": "2.1.0" },
  "permissions": {
    "events": ["ticket.column", "agent.status"],
    "network": ["discord.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook Discord", "en": "Discord webhook URL" } }],
    "webhooks": [
      {
        "url": "{{secrets.webhook}}",
        "events": ["ticket.column"],
        "when": { "data.to": "review" },
        "body": {
          "content": "🧪 **{{ticket.key}}** to test: {{ticket.title}} ({{project.name}})",
          "allowed_mentions": { "parse": [] }
        }
      },
      {
        "url": "{{secrets.webhook}}",
        "events": ["agent.status"],
        "when": { "data.status": "waiting" },
        "body": {
          "content": "✋ **{{agent.name}}** waits for you ({{project.name}})",
          "allowed_mentions": { "parse": [] }
        }
      }
    ]
  },
  "store": { "docs": "docs" }
}
```

La ligne `$schema` donne l’autocomplétion et la vérification dans VS Code et les éditeurs qui lisent les schémas JSON. Le schéma est publié à l’adresse `https://escouade.dev/schemas/plugin-1.json`.

## Les champs

| Champ | Obligatoire | Ce qu’il dit |
| --- | --- | --- |
| `id` | oui | L’identifiant unique du plugin, en nom de domaine inversé et en minuscules : `dev.exemple.slack`. Les ids `dev.escouade.*` sont réservés aux plugins d’Escouade : le store ne les accepte que de son dépôt (`plugins/<dossier>/`), l’app ne les installe que signés par le store. |
| `name` | oui | Son nom : un texte, ou un texte par langue. |
| `version` | oui | Sa version, en trois nombres : `1.2.0`, `0.3.0-beta.1`. La même que l’étiquette git de la version. |
| `description` | non | Ce qu’il fait, en une phrase : un texte, ou un texte par langue. |
| `author` | non | Qui l’a fait : `name` obligatoire, `url` et `email` facultatifs. |
| `license` | non | Sa licence, en identifiant SPDX : `MIT`, `Apache-2.0`. |
| `repository` | non | L’adresse de son dépôt, en `https://`. |
| `escouade` | oui | L’Escouade pour lequel il est écrit : `api`, `min` et `max` (voir plus bas). |
| `icon` | non | Son logo : un PNG carré de son dossier, 256 px de côté au moins, 1 Mo au plus (voir plus bas). |
| `run` | non | Son programme. Sans lui, le plugin est déclaratif : rien ne s’exécute. |
| `activation` | non | Ce qui le démarre, par exemple `event:ticket.column`. Sans activation, son programme ne tourne pas. |
| `permissions` | non | Ce qu’il demande : `events`, `network`, `secrets`, `files`, `commands`, `env` (des listes) et `decide` (`true` ou `false`). Voir [les capacités de l’hôte](plugins/capabilities). |
| `limits` | non | Ce qu’il demande de plus à l’hôte : `fetchSeconds`, `fetchBytes`, `fileBytes` ([les limites](plugins/capabilities#les-limites-limits)), et `processes`, les programmes à la fois, avec sa raison `reason`. |
| `contributes` | non | Ses points d’extension : ce qu’il ajoute à Escouade. |
| `store` | non | Ce que la place de marché montre de lui : `categories`, `keywords`, `screenshots`, `docs` (le dossier de sa documentation, une page par langue sous `docs/fr/` et `docs/en/` : [la documentation, par langue](plugins/publishing#la-documentation-par-langue)). |

### Les textes dans deux langues

`name`, `description` et les titres des réglages prennent un texte seul, ou un objet de textes par langue : `{ "fr": "Salon", "en": "Channel" }`. Escouade montre celui de la langue de l’interface, sinon l’anglais, sinon le français. Un objet doit avoir au moins `en` ou `fr`.

### La plage d’Escouade

- `api` : la version majeure du protocole des plugins que parle le plugin, entre guillemets : `"1"`. Escouade 2.1 sert la version 1.
- `min` : la plus ancienne version d’Escouade avec laquelle le plugin a été essayé, `2.1.0` ou `2.1`.
- `max` (facultatif) : la plus récente. `2.4` comprend toutes les 2.4.x.

Un plugin d’une autre API, ou pour un Escouade hors de sa plage, reste installé mais désactivé, et Escouade dit pourquoi : par exemple « Ce plugin demande Escouade 2.3.0 ou plus récent ; tu as la 2.1.0. Mets Escouade à jour. »

### L’icône

`icon` est le chemin, dans le dossier du plugin, de son logo : un **PNG carré de 256 px de côté au moins** (512 conseillé), de **1 Mo au plus**. Ni SVG (il peut porter du script), ni image distante, ni chemin qui sort du dossier (absolu, ou qui remonte avec `..`). Escouade et la place de marché en lisent l’en-tête, et refusent le plugin en disant pourquoi : « `icon` n’est pas carrée (un PNG carré de 256 px au moins, 512 conseillé). », « `icon` pèse plus de 1 Mo. », « `icon` n’est pas un PNG (ni SVG, ni JPEG : un PNG carré de 256 px au moins). », « `icon` n’est pas un fichier du dossier du plugin. »

La place de marché la réencode sans ses métadonnées en 256 et 64 px, et la montre à la place des deux lettres de la tuile du plugin. Sans icône, la tuile garde ses lettres.

### Le programme

Un programme est l’un des trois, jamais deux :

- `js` : le bundle d’un plugin en JS ou en TypeScript, dans son dossier. Escouade le fait tourner dans l’app, en WASM, sans Node ; la fenêtre d’installation dit « Tourne dans Escouade, isolé ». C’est ce qu’écrit `npm create escouade-plugin` (voir [le SDK](plugins/sdk#sans-node)) :

`sdk/create-escouade-plugin/templates/program/escouade-plugin.json` (extrait)

```json
  "run": { "js": "dist/plugin.mjs" },
```

- `wasm` : le `Cargo.toml` d’un plugin en Rust, jamais un binaire. Le store le compile en module, à côté du manifeste sous le nom `plugin.wasm` (en développement, `escouade-plugin build` l’écrit dans `packs/`, où Escouade le prend) ; la fenêtre dit la même chose que pour `js`, et les mêmes [capacités](plugins/capabilities) s’appliquent (voir [le kit Rust](plugins/rust)) :

`sdk/create-escouade-plugin/templates/rust/escouade-plugin.json` (extrait)

```json
  "run": { "wasm": "Cargo.toml" },
```

- `command` : un programme natif, que **Escouade ne lance plus**, quelle que soit l’origine du plugin : le manifeste se lit, et la page du plugin dit pourquoi il ne démarre pas. Le store le refuse. Pour lancer un programme de l’ordinateur, un plugin en WASM demande `permissions.commands` ([les programmes](plugins/capabilities#les-programmes-commands)).

Un chemin absolu ou qui remonte avec `..` est refusé : un plugin ne lance que ce qu’il apporte.

### Les points d’extension

`contributes` regroupe ce que le plugin ajoute. Escouade vérifie déjà leur forme :

- `settings` : ses réglages, montrés sur sa page dans l’ordre du manifeste. Chacun a une clé (`key`), un type (`string`, `number`, `boolean` ou `select`) et un titre (`title`) ; il peut avoir une `description` (un texte, ou un texte par langue), montrée sous son titre, une valeur par défaut (`default`, de son type ; sans elle, vide, ou `false` pour un `boolean`) et, pour un `select`, ses choix (`options`), dans l’ordre : chacun une valeur (`"udp"`), ou `{ "value": "udp", "title": "UDP" }` pour montrer un titre à sa place ;
- `secrets` : les secrets qu’il garde, chacun avec une clé et un titre ;
- `groups` : des groupes de ses réglages et de ses secrets, mêlés, dans l’ordre où les montrer ([les groupes](#les-groupes)) ;
- `profiles` : des profils, que l’utilisateur crée autant de fois qu’il veut et que chaque projet choisit ([les profils](#les-profils)) ;
- sur un réglage ou un secret, `scope`, `repo`, `multiline`, `lines`, `import` et `description` ([les réglages d’un projet](#les-reglages-d-un-projet), [plusieurs lignes et un fichier](#plusieurs-lignes-et-un-fichier)) ; un réglage `"type": "profile"` choisit un profil ;
- `accounts` : un genre de compte, partagé par ses autres points ;
- `ticketSources`, `forges` et `harnesses` : une source du Kanban, un hébergeur de code, un harness ; chacun avec un `id`, et le compte qu’il utilise (`account`) parmi ceux de `accounts` ;
- `launchEnvironments` : un environnement de lancement, qui prend en charge `up`, `ls`, `down` et `destroy` d’un worktree et ses services ; pour l’instant réservé aux plugins d’Escouade (isola) ;
- `workflowReaders` : un lecteur de workflows, pour apprendre à Escouade un workflow qu’elle ne lit pas seule (le contrat est fixé, mais **Escouade ne l’appelle pas encore**). Chacun a un `id` et `files`, les globs (depuis le dossier du projet, sans `..`) des fichiers qu’il reçoit ; voir [Le lecteur de workflow](plugins/events#le-lecteur-de-workflow) ;
- `themes` : des thèmes, décrits dans [Les thèmes](plugins/themes) ;
- `webhooks` et `commands` : un webhook ou une commande sur un fait, décrits dans [Les événements](plugins/events) ;
- `menus`, `statusBar`, `badges` et `projectTemplates` : **réservés, sans effet dans cette version**. Le manifeste les accepte toujours, mais Escouade n’en montre rien, ne les compte pas dans ce qu’apporte le plugin et n’en appelle rien ; `escouade-plugin validate` et le store le disent par un avertissement, jamais un refus. N’écris pas un plugin qui compte dessus. Les réglages d’un plugin dans `.escouade.yml` se donnent sous `plugins.<id>`, avec les clés de `settings`.

Un même plugin peut tenir plusieurs points : un plugin GitHub donne ses issues au Kanban et suit ses PR sur un seul compte.

### Les genres de compte

Un genre de compte (`accounts`) dit comment on ajoute un compte, et ce compte sert tous les points du plugin qui le nomment (`account`) : une connexion, tous les usages.

```json
"accounts": [{
  "id": "gitlab",
  "name": "GitLab",
  "defaultHost": "gitlab.com",
  "selfHosted": true,
  "connect": ["token", { "cli": "glab", "login": "glab auth login" }],
  "fields": [{ "key": "url" }, { "key": "token", "secret": true }],
  "scopes": ["api"],
  "tokenPage": "{url}/-/user_settings/personal_access_tokens"
}]
```

- `defaultHost` : son instance publique ; `selfHosted` : il a des instances à soi ; `probe` : le chemin qui reconnaît une instance dont le nom ne dit rien ;
- `connect` : un jeton saisi (`token`), ou le jeton de la CLI du service ; un jeton par défaut ;
- `fields` : ce que demande le formulaire (`site`, `email`, `key`, `token`…), `secret` pour ce qui va au trousseau ; un jeton seul par défaut ;
- `scopes` : les droits que le jeton doit avoir ; `tokenPage` : la page qui en crée un.

Un genre qui porte l’`id` d’une intégration d’Escouade (`github`, `gitlab`, `bitbucket`, `gitea`, `azure`, `jira`, `trello`, `linear`, `clickup`) lui est proposé à côté : « Connecter… » demande alors lequel sert le compte, Escouade d’abord. Jamais en silence.

Le secret d’un compte est au trousseau sous `plugin:<id>:account:<compte>` : le compte est l’hôte de son instance (`gitlab.com`), ou l’`id` du genre pour un genre sans instances (`jira`). Le plugin le lit par `secrets/get` de `account:<compte>`, s’il a `account:*` dans `permissions.secrets` ; jamais le compte d’un autre plugin. Ce qu’il tient : le jeton, ou `{ "key": …, "token": … }` pour un genre à deux secrets (Trello).

### Les groupes

`contributes.groups` range la page du plugin. Chaque groupe a un `id`, un titre (`title`), une `description` facultative, `folded` (replié d’office, comme « Avancé ») et `fields` : les clés de ses réglages et de ses secrets, mêlées, dans l’ordre où les montrer. Les groupes viennent dans l’ordre de la liste ; un champ hors de tout groupe vient après, sous « Réglages » puis « Secrets ». Une clé qui n’existe pas, ou qui est dans deux groupes, est refusée à ce champ. Sans groupes, la page est la même qu’avant.

```json
"contributes": {
  "groups": [
    { "id": "connection", "title": { "fr": "Connexion", "en": "Connection" }, "fields": ["server", "token"] },
    { "id": "advanced", "title": { "fr": "Avancé", "en": "Advanced" }, "folded": true, "fields": ["verbosity"] }
  ],
  "settings": [
    { "key": "server", "type": "string", "title": "Server" },
    { "key": "verbosity", "type": "select", "title": "Verbosity", "options": ["1", "3"], "default": "1" }
  ],
  "secrets": [{ "key": "token", "title": "Token", "description": "The token of your account." }]
}
```

### Les réglages d’un projet

`"scope": "project"`, sur un réglage simple (`string`, `number`, `boolean`, `select`) ou `profile`, le rend propre à chaque projet. Il n’est plus sur la page du plugin, mais dans Réglages › Projets › *projet* › Plugins, et `settings/get` de ce projet le rend par ses rangs : la valeur de cette machine pour le projet, puis celle de son `.escouade.yml` (`plugins.<id>.<clé>`), puis `default`. Un plugin n’a ni le fichier ni la valeur de cette machine d’un projet où il n’est pas actif : le défaut seul.

- `"repo": false` écarte le fichier : la valeur ne vient jamais de `.escouade.yml`. `repo` n’est jamais `true` pour un secret, un profil ou une valeur importée d’un fichier.
- **Un secret n’a pas de portée de projet** : un secret de `"scope": "project"` se lit, mais le plugin reste coupé, la raison dite. Les identifiants d’un projet vont dans un [profil](#les-profils).

### Plusieurs lignes et un fichier

- `multiline: true`, sur un réglage `string` ou un secret : une zone de texte à chasse fixe, ses lignes numérotées, `lines` lignes visibles (de 3 à 20, 6 d’office), 64 Kio au plus. C’est un drapeau, pas un type : une Escouade plus ancienne montre un champ d’une ligne.
- `import`, une liste d’extensions (`[".ovpn", ".conf"]`) : « Importer un fichier… » lit le fichier une fois, en UTF-8, 64 Kio au plus, et remplit le champ. Le chemin n’est jamais gardé, et le plugin ne gagne aucun droit sur les fichiers.
- `description`, sur un secret comme sur un réglage : une phrase sous son titre.
- Gardé, un secret sur plusieurs lignes ne se relit pas : « Gardé · 84 lignes · 5,1 Ko », « Remplacer… », « Oublier ». Le journal du plugin en masque chaque ligne de plus de 8 caractères.

```json
"secrets": [{
  "key": "ovpn",
  "title": { "fr": "Configuration OpenVPN", "en": "OpenVPN configuration" },
  "description": { "fr": "Le fichier .ovpn de ton fournisseur, certificats compris.", "en": "Your provider’s .ovpn file, certificates included." },
  "multiline": true,
  "lines": 8,
  "import": [".ovpn", ".conf"]
}]
```

### Les profils

`contributes.profiles` déclare des **genres** de profil : un `id`, un titre, `nameField` (le champ `string` d’une ligne, non secret, qui donne son nom au profil) et `fields`. Un champ simple a une clé, un titre et un `type` parmi `string`, `number`, `boolean`, `select` (avec `description`, `default` et `options` comme un réglage) ; un champ secret dit `"secret": true`, sans type, et prend `multiline` et `import` comme un secret. L’utilisateur crée ses profils sur la page du plugin, autant qu’il veut, et Escouade tire leur identifiant.

- Un réglage `"type": "profile"` dit son genre par `of`. De portée `project`, c’est le profil que prend chaque projet ; de portée `app`, le profil par défaut des projets qui n’en choisissent aucun. `settings/get` en rend l’identifiant, ou `null`.
- `profile:<genre>` dans `permissions.secrets` demande les secrets des profils de ce genre ; un genre qu’il ne déclare pas, ou sans champ secret, est refusé à ce champ.
- Un profil supprimé efface ses secrets, et les projets qui le prenaient n’en ont plus.
- `.escouade.yml` ne choisit jamais un profil : il peut en proposer un par son nom, que l’utilisateur prend ou non.

Le cas complet, du manifeste au programme : [des identifiants par projet](plugins/project-credentials).

## Ce qu’Escouade vérifie

- **Un manifeste invalide ne charge rien.** Escouade dit pourquoi, champ par champ, avec le chemin du champ : « `escouade.min` manque. », « `contributes.settings[0].type` vaut l’un de : string, number, boolean, select, profile. »
- **Le champ fautif, et pourquoi.** `escouade-plugin validate` dit ce qui ne va pas à l’endroit où c’est : un mot d’une commande (« `--pid={pid}` colle un joker à autre chose : un joker est un mot entier »), un champ hors de sa place (« ne va que sur un réglage `string` ou un secret »), une limite sans sa raison. Il prévient aussi (⚠), sans refuser, un plugin qui prend des profils, un réglage de chaque projet, des jokers ou `limits.processes` avec un `escouade.min` plus ancien que 2.2.5 ([les versions](plugins/versioning#le-contrat-de-la-2-2-5)).
- **Un champ inconnu est ignoré.** Un manifeste écrit pour un Escouade plus récent se charge quand même ; les champs ignorés sont listés.
- Le même contrôle tourne sur la place de marché quand un auteur y soumet son plugin : les deux disent les mêmes erreurs ([publier sur le store](plugins/publishing)).

## Installer un plugin

Escouade installe un plugin de quatre façons :

- **Depuis le store** : la version est téléchargée de store.escouade.dev et vérifiée (son empreinte, sa signature quand elle est vérifiée par Escouade) avant d’être copiée au même endroit (voir [installer un plugin](plugins/installer#depuis-le-store)).
- **Depuis un dossier** : le plugin seul est copié dans `~/.escouade/plugins/<id>/`, sans son historique git, `target/`, `node_modules/` ni ce qu’ignore Git, avec de `packs/` le module de sa version, et fixé par l’empreinte SHA-256 de ses fichiers.
- **Depuis un dépôt git et une étiquette** (`v1.2.0`) : Escouade clone cette étiquette seule, la copie au même endroit, et la fixe par son commit. Un plugin est toujours fixé à une version ; installer une autre étiquette remplace la précédente.
- **Un dossier en développement** : chargé là où il est, sans copie, et relu dès qu’un de ses fichiers change. C’est le mode des auteurs de plugins.

La liste des plugins installés, leur origine et leur empreinte sont dans `~/.escouade/plugins.json`. Une copie dont les fichiers ont changé depuis l’installation est désactivée, avec la raison.

Le programme d’un plugin démarre à sa première activation et parle [le protocole](plugins/api). Les faits qui le démarrent et qu’il reçoit sont dans [Les événements](plugins/events). La vue Plugins installe, active et désinstalle les plugins depuis l’interface : voir [installer un plugin](plugins/installer). Les plugins d’Escouade n’ont pas à être installés : ils viennent avec l’app.
