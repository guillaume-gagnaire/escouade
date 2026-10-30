# Events

Escouade tells what happens in it as **facts**: an agent created, a turn over, a ticket to test… A plugin subscribes to the facts it wants: its program receives them, a declared **webhook** sends them to an address without code, or a declared **command** runs.

Facts are told by Escouade’s core where the state changes, never read back from what the window shows: they do not change when the interface does.

## A fact

Every fact has the same shape, in JSON:

```json
{
  "version": 1,
  "seq": 42,
  "at": "2026-10-29T14:03:12.511Z",
  "name": "ticket.column",
  "data": { "from": "doing", "to": "review" },
  "project": { "id": "8c1f…", "name": "Escouade", "private": false },
  "ticket": { "id": "e02a…", "key": "ESC-177", "title": "Events" },
  "agent": { "id": "5b7d…", "name": "esc-177-events" }
}
```

- `version`: the version of the facts’ shape. A field added keeps it; a field removed or changed in meaning moves to the next one.
- `seq`: its number since Escouade started. Two facts come in the order of their numbers.
- `name` and `data`: what happened, and what it says of it (the table below).
- `project`, `ticket`, `agent`: what it is about, by their ids and their names, when it is about them. The fact of an agent working on a ticket also says its ticket; the project says whether it is private (`private`).

A fact **never carries the conversation**: a plugin learns that a turn is over and how, not what was said in it.

## The facts

| Fact                 | When                                                                                    | `data`                                                                                                                                                   |
| -------------------- | --------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `agent.created`      | An agent is created (by hand, for a ticket, by a client of the MCP server).             | `harness`, `model`                                                                                                                                       |
| `agent.renamed`      | It is named (after its first message, or by hand); the facts after it say its new name. | `from`, `to`                                                                                                                                             |
| `agent.status`       | Its status changes.                                                                     | `status` and `was`: `idle`, `running`, `waiting` (a request waits for an answer), `done`, `error`                                                        |
| `agent.stopped`      | It is archived (`removed: false`) or deleted (`removed: true`).                         | `removed`                                                                                                                                                |
| `agent.handover`     | It goes on with another harness.                                                        | `from`, `to`                                                                                                                                             |
| `turn.ended`         | A turn ends.                                                                            | `end`: `finished`, `interrupted`, `limited` (the usage limit), `signIn` (a sign-in to do again) or `error`, with `error` (its first line)                |
| `request.permission` | A permission waits.                                                                     | `requestId`, `tool`, `arg` (what the tool acts on, summed up as on the card)                                                                             |
| `request.question`   | A question waits.                                                                       | `requestId`, `questions`                                                                                                                                 |
| `request.answered`   | A request has its answer.                                                               | `requestId`, `kind` (`permission` or `question`), `decision` (`allow`, `always`, `deny`), `by` (the client of the MCP server that answered, else absent) |
| `ticket.created`     | A ticket is created.                                                                    | `column`                                                                                                                                                 |
| `ticket.column`      | It moves to another column.                                                             | `from`, `to`: `todo`, `doing`, `review`, `done`                                                                                                          |
| `ticket.report`      | Its agent reports on its criteria.                                                      | `met`, `total`                                                                                                                                           |
| `ticket.approved`    | Its validation starts.                                                                  |                                                                                                                                                          |
| `ticket.rejected`    | It is sent back to its agent.                                                           | `comment`                                                                                                                                                |
| `ticket.merged`      | It is merged into its target branch.                                                    | `target`, `commit`                                                                                                                                       |
| `ticket.conflict`    | Its merge meets a conflict.                                                             | `target`, `files`                                                                                                                                        |
| `pr.opened`          | A pull request is opened for it.                                                        | `url`, `target`                                                                                                                                          |
| `tests.result`       | The tests of its validation passed or not.                                              | `passed`                                                                                                                                                 |
| `project.added`      | A project is opened.                                                                    | `path`                                                                                                                                                   |
| `project.closed`     | It is closed.                                                                           |                                                                                                                                                          |
| `worktree.created`   | A worktree is created for an agent.                                                     | `branch`, `path`                                                                                                                                         |
| `worktree.removed`   | It is removed.                                                                          | `branch`, `path`                                                                                                                                         |
| `launch.started`     | A test launch of an agent starts.                                                       | `run` (its name)                                                                                                                                         |
| `launch.ready`       | It answers.                                                                             | `url`                                                                                                                                                    |
| `launch.stopped`     | It stops.                                                                               |                                                                                                                                                          |
| `docs.written`       | A page of the docs is written.                                                          | `page`                                                                                                                                                   |
| `docs.filed`         | A ticket’s copy of the docs is poured on the docs’ branch, at its validation.           | `pages`                                                                                                                                                  |
| `docs.gaps`          | A page is compared to the code.                                                         | `page`, `gaps` (how many gaps)                                                                                                                           |
| `memory.written`     | A memory of the project’s memory is written.                                            | `memory`, `new`                                                                                                                                          |
| `memory.forgotten`   | It is forgotten.                                                                        | `memory`                                                                                                                                                 |
| `decision.proposed`  | A decision is proposed or revised.                                                      | `id` (no number yet), `title`                                                                                                                                |
| `decision.accepted`  | A human accepts it.                                                                     | `number` (given then), `id`, `title`                                                                                                                                |
| `account.switched`   | An agent changes account.                                                               | `from`, `to`                                                                                                                                             |
| `app.started`        | Escouade starts.                                                                        | `version`                                                                                                                                                |

## Subscribing

A plugin only receives the facts it asks for, in `permissions.events` of its manifest. A name ending with `*` takes them all (`ticket.*`, or `*` for everything):

```json
"permissions": { "events": ["ticket.column", "agent.status"] }
```

The install window shows them: “Receives these events: …”.

And only those of the projects it is **turned on** for. A plugin is on for every project, except **private projects**: a private project’s fact only goes to a plugin turned on for it, one by one, in Settings › Plugins. A fact of no project (`app.started`) goes to every plugin on for the app. An `events:…` permission taken back in the Permissions tab of its page cuts those facts, for its program as for its webhooks and its commands.

## In a program

A plugin’s program (`run`) receives each fact by the `events/fact` notification, the whole fact as its parameters. It does not answer: Escouade never waits for a plugin.

```json
{
  "jsonrpc": "2.0",
  "method": "events/fact",
  "params": {
    "version": 1,
    "seq": 42,
    "name": "ticket.column",
    "data": { "from": "doing", "to": "review" },
    "ticket": { "id": "e02a…", "key": "ESC-177", "title": "Events" }
  }
}
```

To start it at the first fact, put it in `activation`: `"activation": ["event:ticket.column"]`.

A plugin’s facts go through **a queue of its own**, of 256 facts. The next one goes once its program took the last one on its input; a slow plugin holds nothing else back. When the queue is full, the oldest fact is lost, and its journal says so: “the queue of facts is full (256): the oldest, ticket.column #41, is dropped”.

## A webhook, without code

A **declarative** plugin (without `run`) calls an address on a fact. Each webhook of `contributes.webhooks` says:

| Field         | Required | What it says                                                                                                                     |
| ------------- | -------- | -------------------------------------------------------------------------------------------------------------------------------- |
| `url`         | yes      | The address, `http://` or `https://`, to a host of `permissions.network`. It may come from a secret of the plugin: `{{secrets.webhook}}`.                            |
| `events`      | yes      | The facts that call it (`ticket.*` takes them all). They must also be in `permissions.events`.                                   |
| `when`        | no       | What the fact must say, path by path: `{ "data.to": "review" }`, or a list of values: `{ "data.status": ["waiting", "error"] }`. |
| `body`        | no       | The body’s template: a JSON object, or a text. Without it, the whole fact in JSON.                                               |
| `contentType` | no       | The body’s type: `application/json` for an object or without a template, `text/plain` for a text.                                |
| `headers`     | no       | Headers, which may take a secret: `{ "Authorization": "Bearer {{secrets.token}}" }`.                                             |

A template replaces `{{path}}` with the fact’s value: `{{name}}`, `{{ticket.key}}`, `{{ticket.title}}`, `{{project.name}}`, `{{agent.name}}`, `{{data.to}}`… In an object, a text that is only a `{{path}}` takes the value as it is (a list stays a list); in a JSON text, the values are escaped. A secret never goes in the body.

The Discord webhook of the tickets to test:

```json
{
  "id": "dev.example.discord",
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
        "body": { "content": "{{ticket.key}} to test: {{ticket.title}} ({{project.name}})" }
      }
    ]
  }
}
```

**A webhook only goes to a host of `permissions.network`**, the one the install window shows, whether its address is written whole, taken from a secret or made of a value of the fact. Escouade refuses at load a manifest whose webhook written whole goes elsewhere, and makes no call whose address, once made, goes elsewhere (the journal says so): a ticket title like `evil.dev/x` sends nothing. A host is compared without regard to case, without the address’s user or port. `hooks.slack.com` covers itself alone; `*.example.dev` covers its subdomains at any depth (`hooks.example.dev`, `a.b.example.dev`), but not `example.dev` itself, to declare apart, nor `evilexample.dev`. The webhook has 10 s to answer, without redirects. Its journal notes each call by its host and its answer, never the whole address (Discord’s holds a token).

## A command on a fact

`contributes.commands` runs a command line on a fact, in the user’s shell, in the folder of the fact’s project:

```json
"contributes": {
  "commands": [
    { "id": "turn-over", "events": ["turn.ended"], "when": { "data.end": "finished" }, "run": "say \"Claude is done\"" }
  ]
}
```

- **Nothing runs before the user agrees**, as with the commands of `.escouade.yml`: the Permissions tab of its page, in Settings › Plugins, shows them under “Runs these commands” with “Agree to these commands”, and the agreement holds for them as they are; “Take the agreement back” stops them. A command, a fact or a condition that changes, and it takes agreeing again. The journal tells each command refused for want of agreement.
- The fact **never goes in the command line**: a ticket’s title runs nothing. The command finds it in its variables: `ESCOUADE_FACT` (the whole fact in JSON), `ESCOUADE_FACT_NAME`, `ESCOUADE_PROJECT`, `ESCOUADE_TICKET` (its key) and `ESCOUADE_AGENT`.
- It has 60 s. Its journal notes its end (`exit 0`) and, when it fails, its last lines.

## The journal

Each plugin’s journal (Settings › Plugins, and `plugin-logs/<id>.log` in Escouade’s data) tells the webhooks called and their answers, the commands run and how they ended, what was refused (a secret not its own, an address not declared, a command not agreed to) and the facts lost from a full queue.

## Trying it

`escouade-plugin dev <folder>` runs your plugin outside Escouade and sends it the facts you pick; `escouade-plugin test <folder>` checks that it takes a fact without crashing. See [Testing a plugin](plugins/testing).
