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
| `permissions` | non | Ce qu’il demande : `events`, `network`, `secrets`, `files`, `commands` (des listes) et `decide` (`true` ou `false`). Voir [les capacités de l’hôte](plugins/capabilities). |
| `limits` | non | Ce qu’il demande de plus à l’hôte : `fetchSeconds`, `fetchBytes`, `fileBytes` ([les limites](plugins/capabilities#les-limites-limits)). |
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

- `wasm` : le `Cargo.toml` d’un plugin en Rust, jamais un binaire. Le store le compile en module, à côté du manifeste sous le nom `plugin.wasm` ; la fenêtre dit la même chose que pour `js`, et les mêmes [capacités](plugins/capabilities) s’appliquent (voir [le kit Rust](plugins/rust)) :

`sdk/create-escouade-plugin/templates/rust/escouade-plugin.json` (extrait)

```json
  "run": { "wasm": "Cargo.toml" },
```

- `command` : le chemin d’un programme dans le dossier du plugin, que lance un process ; `windows`, `macos` et `linux` en donnent un autre pour chaque système ; `args` ajoute des arguments. La fenêtre d’installation dit « Lance un programme sur ton ordinateur, avec tes droits ». **Le store le refuse** : un plugin du store n’a jamais de `run.command`, chaque plugin y tourne en WASM. Il reste lancé pour un plugin installé depuis un dossier ou un dépôt git, non vérifié. Pour lancer un programme de l’ordinateur, un plugin en WASM demande `permissions.commands` ([les programmes](plugins/capabilities#les-programmes-commands)).

Un chemin absolu ou qui remonte avec `..` est refusé : un plugin ne lance que ce qu’il apporte.

### Les points d’extension

`contributes` regroupe ce que le plugin ajoute. Escouade vérifie déjà leur forme :

- `settings` : ses réglages, chacun avec une clé (`key`), un type (`string`, `number`, `boolean` ou `select`) et un titre (`title`) ;
- `secrets` : les secrets qu’il garde, chacun avec une clé et un titre ;
- `accounts` : un genre de compte, partagé par ses autres points ;
- `ticketSources`, `forges` et `harnesses` : une source du Kanban, un hébergeur de code, un harness ; chacun avec un `id`, et le compte qu’il utilise (`account`) parmi ceux de `accounts` ;
- `launchEnvironments` : un environnement de lancement, qui prend en charge `up`, `ls`, `down` et `destroy` d’un worktree et ses services ; pour l’instant réservé aux plugins livrés avec Escouade (isola) ;
- `workflowReaders` : un lecteur de workflows, pour apprendre à Escouade un workflow qu’elle ne lit pas seule. Chacun a un `id` et `files`, les globs (depuis le dossier du projet, sans `..`) des fichiers qu’il reçoit ; voir [Le lecteur de workflow](plugins/events#le-lecteur-de-workflow) ;
- `themes` : des thèmes, décrits dans [Les thèmes](plugins/themes) ;
- `webhooks` et `commands` : un webhook ou une commande sur un fait, décrits dans [Les événements](plugins/events) ;
- `menus`, `statusBar`, `badges`, `projectTemplates` et `escouadeYml` : décrits avec les versions qui les mettent en service.

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

## Ce qu’Escouade vérifie

- **Un manifeste invalide ne charge rien.** Escouade dit pourquoi, champ par champ, avec le chemin du champ : « `escouade.min` manque. », « `contributes.settings[0].type` vaut l’un de : string, number, boolean, select. »
- **Un champ inconnu est ignoré.** Un manifeste écrit pour un Escouade plus récent se charge quand même ; les champs ignorés sont listés.
- Le même contrôle tourne sur la place de marché quand un auteur y soumet son plugin : les deux disent les mêmes erreurs ([publier sur le store](plugins/publishing)).

## Installer un plugin

Escouade installe un plugin de quatre façons :

- **Depuis le store** : la version est téléchargée de store.escouade.dev et vérifiée (son empreinte, sa signature quand elle est vérifiée par Escouade) avant d’être copiée au même endroit (voir [installer un plugin](plugins/installer#depuis-le-store)).
- **Depuis un dossier** : le dossier est copié dans `~/.escouade/plugins/<id>/`, sans son historique git, et fixé par l’empreinte SHA-256 de ses fichiers.
- **Depuis un dépôt git et une étiquette** (`v1.2.0`) : Escouade clone cette étiquette seule, la copie au même endroit, et la fixe par son commit. Un plugin est toujours fixé à une version ; installer une autre étiquette remplace la précédente.
- **Un dossier en développement** : chargé là où il est, sans copie, et relu dès qu’un de ses fichiers change. C’est le mode des auteurs de plugins.

La liste des plugins installés, leur origine et leur empreinte sont dans `~/.escouade/plugins.json`. Une copie dont les fichiers ont changé depuis l’installation est désactivée, avec la raison.

Le programme d’un plugin démarre à sa première activation et parle [le protocole](plugins/api). Les faits qui le démarrent et qu’il reçoit sont dans [Les événements](plugins/events). La vue Plugins installe, active et désinstalle les plugins depuis l’interface : voir [installer un plugin](plugins/installer). Les plugins livrés avec Escouade n’ont pas à être installés : ils viennent avec l’app.
