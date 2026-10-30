# Les événements

Escouade dit ce qui s’y passe sous forme de **faits** : un agent créé, un tour fini, un ticket à tester… Un plugin s’abonne aux faits qu’il veut : son programme les reçoit, un **webhook** déclaré les envoie à une adresse sans code, ou une **commande** déclarée se lance.

Les faits sont dits par le cœur d’Escouade à l’endroit où l’état change, jamais relus dans ce qu’affiche la fenêtre : ils ne changent pas quand l’interface change.

## Un fait

Chaque fait a la même forme, en JSON :

```json
{
  "version": 1,
  "seq": 42,
  "at": "2026-10-29T14:03:12.511Z",
  "name": "ticket.column",
  "data": { "from": "doing", "to": "review" },
  "project": { "id": "8c1f…", "name": "Escouade", "private": false },
  "ticket": { "id": "e02a…", "key": "ESC-177", "title": "Les événements" },
  "agent": { "id": "5b7d…", "name": "esc-177-les-evenements" }
}
```

- `version` : la version de la forme des faits. Un champ ajouté la garde ; un champ retiré ou qui change de sens passe à la suivante.
- `seq` : son numéro depuis le démarrage d’Escouade. Deux faits arrivent dans l’ordre de leurs numéros.
- `name` et `data` : ce qui s’est passé, et ce qu’il en dit (la table plus bas).
- `project`, `ticket`, `agent` : ce dont il parle, par leurs ids et leurs noms, quand il en parle. Le fait d’un agent qui travaille sur un ticket dit aussi son ticket ; le projet dit s’il est privé (`private`).

Un fait ne porte **jamais la conversation** : un plugin apprend qu’un tour est fini et comment, pas ce qui s’y est dit.

## Les faits

| Fait                 | Quand                                                                                               | `data`                                                                                                                                                |
| -------------------- | --------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `agent.created`      | Un agent est créé (à la main, pour un ticket, par un client du serveur MCP).                        | `harness`, `model`                                                                                                                                    |
| `agent.renamed`      | Il est nommé (après son premier message, ou à la main) ; les faits suivants disent son nouveau nom. | `from`, `to`                                                                                                                                          |
| `agent.status`       | Son statut change.                                                                                  | `status` et `was` : `idle`, `running`, `waiting` (une demande attend une réponse), `done`, `error`                                                    |
| `agent.stopped`      | Il est archivé (`removed: false`) ou supprimé (`removed: true`).                                    | `removed`                                                                                                                                             |
| `agent.handover`     | Il continue sur un autre harness.                                                                   | `from`, `to`                                                                                                                                          |
| `turn.ended`         | Un tour se termine.                                                                                 | `end` : `finished`, `interrupted`, `limited` (la limite d’usage), `signIn` (une connexion à refaire) ou `error`, avec `error` (sa première ligne)     |
| `request.permission` | Une autorisation attend.                                                                            | `requestId`, `tool`, `arg` (ce sur quoi l’outil agit, résumé comme sur la carte)                                                                      |
| `request.question`   | Une question attend.                                                                                | `requestId`, `questions`                                                                                                                              |
| `request.answered`   | Une demande a sa réponse.                                                                           | `requestId`, `kind` (`permission` ou `question`), `decision` (`allow`, `always`, `deny`), `by` (le client du serveur MCP qui a répondu, sinon absent) |
| `ticket.created`     | Un ticket est créé.                                                                                 | `column`                                                                                                                                              |
| `ticket.column`      | Il change de colonne.                                                                               | `from`, `to` : `todo`, `doing`, `review`, `done`                                                                                                      |
| `ticket.report`      | Son agent rend le bilan de ses critères.                                                            | `met`, `total`                                                                                                                                        |
| `ticket.approved`    | Sa validation commence.                                                                             |                                                                                                                                                       |
| `ticket.rejected`    | Il est renvoyé à son agent.                                                                         | `comment`                                                                                                                                             |
| `ticket.merged`      | Il est mergé dans sa branche cible.                                                                 | `target`, `commit`                                                                                                                                    |
| `ticket.conflict`    | Son merge rencontre un conflit.                                                                     | `target`, `files`                                                                                                                                     |
| `pr.opened`          | Une pull request est ouverte pour lui.                                                              | `url`, `target`                                                                                                                                       |
| `tests.result`       | Les tests de sa validation sont passés ou non.                                                      | `passed`                                                                                                                                              |
| `project.added`      | Un projet est ouvert.                                                                               | `path`                                                                                                                                                |
| `project.closed`     | Il est fermé.                                                                                       |                                                                                                                                                       |
| `worktree.created`   | Un worktree est créé pour un agent.                                                                 | `branch`, `path`                                                                                                                                      |
| `worktree.removed`   | Il est retiré.                                                                                      | `branch`, `path`                                                                                                                                      |
| `launch.started`     | Un lancement de test d’un agent démarre.                                                            | `run` (son nom)                                                                                                                                       |
| `launch.ready`       | Il répond.                                                                                          | `url`                                                                                                                                                 |
| `launch.stopped`     | Il s’arrête.                                                                                        |                                                                                                                                                       |
| `docs.written`       | Une page des docs est écrite.                                                                       | `page`                                                                                                                                                |
| `docs.filed`         | La copie des docs d’un ticket est versée sur la branche des docs, à sa validation.                  | `pages`                                                                                                                                               |
| `docs.gaps`          | Une page est comparée au code.                                                                      | `page`, `gaps` (le nombre d’écarts)                                                                                                                   |
| `memory.written`     | Un souvenir de la mémoire du projet est écrit.                                                      | `memory`, `new`                                                                                                                                       |
| `memory.forgotten`   | Il est oublié.                                                                                      | `memory`                                                                                                                                              |
| `decision.proposed`  | Une décision est proposée ou révisée.                                                               | `id` (pas encore de numéro), `title`                                                                                                                             |
| `decision.accepted`  | Un humain l’accepte.                                                                                | `number` (donné alors), `id`, `title`                                                                                                                             |
| `account.switched`   | Un agent change de compte.                                                                          | `from`, `to`                                                                                                                                          |
| `app.started`        | Escouade démarre.                                                                                   | `version`                                                                                                                                             |

## S’abonner

Un plugin ne reçoit que les faits qu’il demande, dans `permissions.events` de son manifeste. Un nom finissant par `*` les prend tous (`ticket.*`, ou `*` pour tout) :

```json
"permissions": { "events": ["ticket.column", "agent.status"] }
```

La fenêtre d’installation les montre : « Reçoit ces événements : … ».

Et seulement ceux des projets où il est **activé**. Un plugin est activé pour tous les projets, sauf les **projets privés** : un fait d’un projet privé ne va qu’à un plugin activé pour lui, un par un, dans Réglages › Plugins. Un fait sans projet (`app.started`) va à tout plugin activé pour l’app. Une permission `events:…` retirée dans l’onglet Permissions de sa fiche coupe ces faits, pour son programme comme pour ses webhooks et ses commandes.

## Dans un programme

Le programme d’un plugin (`run`) reçoit chaque fait par la notification `events/fact`, le fait entier en paramètres. Il n’y répond pas : Escouade n’attend jamais un plugin.

```json
{
  "jsonrpc": "2.0",
  "method": "events/fact",
  "params": {
    "version": 1,
    "seq": 42,
    "name": "ticket.column",
    "data": { "from": "doing", "to": "review" },
    "ticket": { "id": "e02a…", "key": "ESC-177", "title": "Les événements" }
  }
}
```

Pour le démarrer au premier fait, mets-le dans `activation` : `"activation": ["event:ticket.column"]`.

Les faits d’un plugin passent par **une file à lui**, de 256 faits. Le suivant part quand son programme a pris le précédent sur son entrée ; un plugin lent ne retarde rien d’autre. Quand la file est pleine, le plus vieux fait est perdu, et son journal le dit : « the queue of facts is full (256): the oldest, ticket.column #41, is dropped ».

## Un webhook, sans code

Un plugin **déclaratif** (sans `run`) appelle une adresse sur un fait. Chaque webhook de `contributes.webhooks` dit :

| Champ         | Obligatoire | Ce qu’il dit                                                                                                                                  |
| ------------- | ----------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| `url`         | oui         | L’adresse, en `http://` ou `https://`, vers un hôte de `permissions.network`. Elle peut venir d’un secret du plugin : `{{secrets.webhook}}`.                                         |
| `events`      | oui         | Les faits qui l’appellent (`ticket.*` les prend tous). Ils doivent aussi être dans `permissions.events`.                                      |
| `when`        | non         | Ce que le fait doit dire, chemin par chemin : `{ "data.to": "review" }`, ou une liste de valeurs : `{ "data.status": ["waiting", "error"] }`. |
| `body`        | non         | Le gabarit du corps : un objet JSON, ou un texte. Sans lui, le fait entier en JSON.                                                           |
| `contentType` | non         | Le type du corps : `application/json` pour un objet ou sans gabarit, `text/plain` pour un texte.                                              |
| `headers`     | non         | Des en-têtes, qui peuvent prendre un secret : `{ "Authorization": "Bearer {{secrets.token}}" }`.                                              |

Un gabarit remplace `{{chemin}}` par la valeur du fait : `{{name}}`, `{{ticket.key}}`, `{{ticket.title}}`, `{{project.name}}`, `{{agent.name}}`, `{{data.to}}`… Dans un objet, un texte qui n’est qu’un `{{chemin}}` prend la valeur telle quelle (une liste reste une liste) ; dans un texte JSON, les valeurs sont échappées. Un secret ne va jamais dans le corps.

Le webhook Discord des tickets à tester :

```json
{
  "id": "dev.exemple.discord",
  "name": "Discord",
  "version": "1.0.0",
  "escouade": { "api": "1", "min": "2.1.0" },
  "permissions": {
    "events": ["ticket.column"],
    "network": ["discord.com"],
    "secrets": ["webhook"]
  },
  "contributes": {
    "secrets": [{ "key": "webhook", "title": { "fr": "Adresse du webhook", "en": "Webhook URL" } }],
    "webhooks": [
      {
        "url": "{{secrets.webhook}}",
        "events": ["ticket.column"],
        "when": { "data.to": "review" },
        "body": { "content": "{{ticket.key}} à tester : {{ticket.title}} ({{project.name}})" }
      }
    ]
  }
}
```

**Un webhook ne part que vers un hôte de `permissions.network`**, celui que la fenêtre d’installation montre, que son adresse soit écrite en clair, tirée d’un secret ou faite d’une valeur du fait. Escouade refuse au chargement un manifeste dont un webhook écrit en clair va ailleurs, et ne fait pas un appel dont l’adresse, une fois faite, va ailleurs (le journal le dit) : un titre de ticket comme `evil.dev/x` n’envoie rien. Un hôte se compare sans tenir compte de la casse, sans l’utilisateur ni le port de l’adresse. `hooks.slack.com` ne couvre que lui-même ; `*.exemple.dev` couvre ses sous-domaines à toute profondeur (`hooks.exemple.dev`, `a.b.exemple.dev`), mais pas `exemple.dev` lui-même, à déclarer à part, ni `evilexemple.dev`. Le webhook a 10 s pour répondre, sans redirection. Son journal note chaque appel par son hôte et sa réponse, jamais l’adresse entière (celle de Discord contient un jeton).

## Une commande sur un fait

`contributes.commands` lance une ligne de commande sur un fait, dans le shell de l’utilisateur, dans le dossier du projet du fait :

```json
"contributes": {
  "commands": [
    { "id": "fin-du-tour", "events": ["turn.ended"], "when": { "data.end": "finished" }, "run": "say \"Claude a fini\"" }
  ]
}
```

- **Rien ne tourne avant l’accord** de l’utilisateur, comme les commandes de `.escouade.yml` : l’onglet Permissions de sa fiche, dans Réglages › Plugins, les montre sous « Lance ces commandes » avec « Accepter ces commandes », et l’accord vaut pour elles telles qu’elles sont ; « Retirer l’accord » les arrête. Une commande, un fait ou une condition qui change, et il faut l’accord à nouveau. Le journal dit chaque commande refusée faute d’accord.
- Le fait n’entre **jamais dans la ligne de commande** : un titre de ticket ne lance rien. La commande le trouve dans ses variables : `ESCOUADE_FACT` (le fait entier en JSON), `ESCOUADE_FACT_NAME`, `ESCOUADE_PROJECT`, `ESCOUADE_TICKET` (sa clé) et `ESCOUADE_AGENT`.
- Elle a 60 s. Son journal note sa fin (`exit 0`) et, si elle échoue, ses dernières lignes.

## Le journal

Le journal de chaque plugin (Réglages › Plugins, et `plugin-logs/<id>.log` dans les données d’Escouade) dit les webhooks appelés et leurs réponses, les commandes lancées et leur fin, ce qui a été refusé (un secret qui n’est pas le sien, une adresse non déclarée, une commande sans accord) et les faits perdus d’une file pleine.

## Essayer

`escouade-plugin dev <dossier>` lance ton plugin hors d’Escouade et lui envoie les faits que tu choisis ; `escouade-plugin test <dossier>` vérifie qu’il reçoit un fait sans planter. Voir [Tester un plugin](plugins/testing).
