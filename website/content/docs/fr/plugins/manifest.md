# Le manifeste

Chaque plugin d’Escouade se décrit dans un fichier escouade-plugin.json à la racine de son dossier. Cette page en est la référence : chaque champ, ce qu’Escouade vérifie, comment il installe un plugin et pourquoi il en désactive un.

## Un exemple complet

```json
{
  "$schema": "https://escouade.dev/schemas/plugin-1.json",
  "id": "dev.exemple.slack",
  "name": "Slack",
  "version": "1.2.0",
  "description": { "fr": "Les tickets à tester dans un salon Slack.", "en": "Tickets to review in a Slack channel." },
  "author": { "name": "Exemple", "url": "https://exemple.dev" },
  "license": "MIT",
  "repository": "https://github.com/exemple/escouade-slack",
  "escouade": { "api": "1", "min": "2.1.0" },
  "run": { "command": "bin/escouade-slack", "windows": "bin/escouade-slack.exe" },
  "activation": ["event:ticket.column"],
  "permissions": {
    "events": ["ticket.*"],
    "network": ["hooks.slack.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "settings": [{ "key": "channel", "type": "string", "title": { "fr": "Salon", "en": "Channel" } }],
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook", "en": "Webhook URL" } }]
  }
}
```

La ligne `$schema` donne l’autocomplétion et la vérification dans VS Code et les éditeurs qui lisent les schémas JSON. Le schéma est publié à l’adresse `https://escouade.dev/schemas/plugin-1.json`.

## Les champs

| Champ | Obligatoire | Ce qu’il dit |
| --- | --- | --- |
| `id` | oui | L’identifiant unique du plugin, en nom de domaine inversé et en minuscules : `dev.exemple.slack`. Les ids `dev.escouade.*` sont réservés aux plugins d’Escouade. |
| `name` | oui | Son nom : un texte, ou un texte par langue. |
| `version` | oui | Sa version, en trois nombres : `1.2.0`, `0.3.0-beta.1`. La même que l’étiquette git de la version. |
| `description` | non | Ce qu’il fait, en une phrase : un texte, ou un texte par langue. |
| `author` | non | Qui l’a fait : `name` obligatoire, `url` et `email` facultatifs. |
| `license` | non | Sa licence, en identifiant SPDX : `MIT`, `Apache-2.0`. |
| `repository` | non | L’adresse de son dépôt, en `https://`. |
| `escouade` | oui | L’Escouade pour lequel il est écrit : `api`, `min` et `max` (voir plus bas). |
| `run` | non | Son programme. Sans lui, le plugin est déclaratif : rien ne s’exécute. |
| `activation` | non | Ce qui le démarre, par exemple `event:ticket.column`. Sans activation, son programme ne tourne pas. |
| `permissions` | non | Ce qu’il demande : `events`, `network`, `secrets`, `commands` (des listes) et `decide` (`true` ou `false`). |
| `contributes` | non | Ses points d’extension : ce qu’il ajoute à Escouade. |
| `store` | non | Ce que la place de marché montre de lui : `categories`, `keywords`, `screenshots`, `docs`. |

### Les textes dans deux langues

`name`, `description` et les titres des réglages prennent un texte seul, ou un objet de textes par langue : `{ "fr": "Salon", "en": "Channel" }`. Escouade montre celui de la langue de l’interface, sinon l’anglais, sinon le français. Un objet doit avoir au moins `en` ou `fr`.

### La plage d’Escouade

- `api` : la version majeure du protocole des plugins que parle le plugin, entre guillemets : `"1"`. Escouade 2.1 sert la version 1.
- `min` : la plus ancienne version d’Escouade avec laquelle le plugin a été essayé, `2.1.0` ou `2.1`.
- `max` (facultatif) : la plus récente. `2.4` comprend toutes les 2.4.x.

Un plugin d’une autre API, ou pour un Escouade hors de sa plage, reste installé mais désactivé, et Escouade dit pourquoi : par exemple « Ce plugin demande Escouade 2.3.0 ou plus récent ; tu as la 2.1.0. Mets Escouade à jour. »

### Le programme

`command` est le chemin du programme dans le dossier du plugin ; `windows`, `macos` et `linux` en donnent un autre pour chaque système qui a le sien ; `args` ajoute des arguments. Un chemin absolu ou qui remonte avec `..` est refusé : un plugin ne lance que ce qu’il apporte.

### Les points d’extension

`contributes` regroupe ce que le plugin ajoute. Escouade vérifie déjà leur forme :

- `settings` : ses réglages, chacun avec une clé (`key`), un type (`string`, `number`, `boolean` ou `select`) et un titre (`title`) ;
- `secrets` : les secrets qu’il garde, chacun avec une clé et un titre ;
- `accounts` : un genre de compte, partagé par ses autres points ;
- `ticketSources`, `forges` et `harnesses` : une source du Kanban, un hébergeur de code, un harness ; chacun avec un `id`, et le compte qu’il utilise (`account`) parmi ceux de `accounts` ;
- `themes`, `webhooks`, `commands`, `menus`, `statusBar`, `badges`, `projectTemplates` et `escouadeYml` : décrits avec les versions qui les mettent en service.

Un même plugin peut tenir plusieurs points : un plugin GitHub donne ses issues au Kanban et suit ses PR sur un seul compte.

## Ce qu’Escouade vérifie

- **Un manifeste invalide ne charge rien.** Escouade dit pourquoi, champ par champ, avec le chemin du champ : « `escouade.min` manque. », « `contributes.settings[0].type` vaut l’un de : string, number, boolean, select. »
- **Un champ inconnu est ignoré.** Un manifeste écrit pour un Escouade plus récent se charge quand même ; les champs ignorés sont listés.
- Le même contrôle tourne sur la place de marché quand un auteur y soumet son plugin : les deux disent les mêmes erreurs ([publier sur le store](plugins/publishing)).

## Installer un plugin

Escouade installe un plugin de trois façons :

- **Depuis un dossier** : le dossier est copié dans `~/.escouade/plugins/<id>/`, sans son historique git, et fixé par l’empreinte SHA-256 de ses fichiers.
- **Depuis un dépôt git et une étiquette** (`v1.2.0`) : Escouade clone cette étiquette seule, la copie au même endroit, et la fixe par son commit. Un plugin est toujours fixé à une version ; installer une autre étiquette remplace la précédente.
- **Un dossier en développement** : chargé là où il est, sans copie, et relu dès qu’un de ses fichiers change. C’est le mode des auteurs de plugins.

La liste des plugins installés, leur origine et leur empreinte sont dans `~/.escouade/plugins.json`. Une copie dont les fichiers ont changé depuis l’installation est désactivée, avec la raison.

L’onglet Réglages › Plugins, qui installe, active et désinstalle les plugins depuis l’interface, arrive avec la suite de la 2.1. Aucun plugin ne lance encore de programme : cette version lit et vérifie les manifestes.
