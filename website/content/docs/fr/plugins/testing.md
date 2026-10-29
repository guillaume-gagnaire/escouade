# Tester un plugin

Escouade donne aux auteurs de plugins une commande, `escouade-plugin`, qui fait tourner un plugin sans l’app : le **banc de conformité** le passe au crible de ce qu’Escouade attend de lui, et le **faux Escouade** lui envoie des faits et des demandes à la main. Le banc est celui que joue Escouade sur ses propres plugins.

## La commande

`escouade-plugin` est livrée avec Escouade, à côté de son programme : `%LOCALAPPDATA%\Escouade\escouade-plugin.exe` sous Windows, `Escouade.app/Contents/MacOS/escouade-plugin` sous macOS, `/usr/bin/escouade-plugin` avec le paquet `.deb`. Elle n’a besoin que de ce qu’il faut au programme du plugin pour démarrer (Node pour un plugin en JavaScript).

```sh
escouade-plugin validate mon-plugin   # le manifeste, champ par champ
escouade-plugin test mon-plugin       # le banc de conformité
escouade-plugin dev mon-plugin        # le faux Escouade
```

`--lang fr` ou `--lang en` choisit la langue de ses messages ; sans lui, c’est celle du système.

## Le banc de conformité

`escouade-plugin test <dossier>` lit le manifeste, lance le programme de `run` comme Escouade le lance (dans son dossier, avec ses trois variables, avec tout ce qu’il démarre), et joue neuf scénarios. Chacun dit ce qui s’est passé ; un scénario qui échoue dit aussi **ce qui manque**.

| Scénario | Ce que le banc fait | Ce qu’il attend |
| --- | --- | --- |
| Manifeste | Lit `escouade-plugin.json`. | Un manifeste valide, pour cette version d’Escouade. |
| Initialisation | Envoie `initialize`. | `{"protocolVersion": 1, "capabilities": {…}}` en moins de 10 s. |
| Version refusée | Relance le plugin et lui offre la version 999 du protocole. | Sa propre version, ou une erreur : jamais la version offerte. |
| Délais | Mesure chaque appel du banc, et appelle `bench/unknown`, qu’aucun plugin ne sert. | Chaque réponse dans le délai de sa méthode ; l’erreur `-32601` tout de suite à une méthode inconnue. |
| Un fait reçu | Envoie un fait de `permissions.events` (la notification `events/fact`), puis un appel. | Pas de réponse à la notification, pas de plantage, et la réponse à l’appel suivant. |
| Une décision | Envoie `permissions/decide` pour une commande `npm test`. | `{"decision": "allow" \| "deny" \| "ask"}`, avec un `message` pour un refus. |
| Un secret | Sert `secrets/get` pendant tout le banc, avec une valeur de test. | Seulement les clés de `permissions.secrets`, et jamais leur valeur écrite sur la sortie d’erreur ou dans un `log`. |
| Plantage | Tue le plugin et tout ce qu’il a lancé au milieu d’un appel, puis le relance. | Un nouveau process qui s’initialise comme le premier. |
| Fermeture | Envoie `shutdown`. | Une réponse en moins d’une seconde, puis la fin du process. |

Un scénario qui ne concerne pas le plugin n’est pas joué et le dit : pas de fait sans `permissions.events`, pas de décision sans `permissions.decide`, et rien d’un programme pour un plugin déclaratif, dont seul le manifeste est lu.

```text
Banc de conformité : dev.exemple.slow-rules 1.0.0

✓ Manifeste        valide, avec un programme
✓ Initialisation   répond en 61 ms, protocole 1
✓ Version refusée  offerte la version 999, il répond la sienne (1) : Escouade l'arrête
✗ Délais           permissions/decide : répondu en 812 ms, au-delà de son délai de 500 ms
                   Ce qui manque : répondre dans le délai de chaque méthode (500 ms pour une décision, 5 s en général) ; passé ce délai, Escouade n'attend plus et sa réponse par défaut s'applique.
– Un fait reçu     il ne demande aucun fait (`permissions.events`)
✓ Une décision     « allow » en 812 ms
– Un secret        il ne demande aucun secret (`permissions.secrets`)
✓ Plantage         tué au milieu d'un appel, il redémarre : répond en 58 ms, protocole 1
✓ Fermeture        répond en 2 ms, puis se termine

6 réussis, 1 échoués, 2 sans objet
```

Le code de sortie est `1` si un scénario échoue, `0` sinon : la commande sert telle quelle en CI.

- `--json` donne le rapport en JSON : chaque scénario, son verdict (`pass`, `fail`, `skip`), ce qui s’est passé, ce qui manque, et la durée de chaque appel.
- `--measure [n]` mesure ensuite `n` allers-retours (1 000 par défaut) et donne leur médiane et leur 99e centile, en microsecondes.

### Les faits et les décisions du banc

Le banc envoie les faits et les décisions dans la forme que leur donnera Escouade : le projet, le ticket et l’agent par leurs ids et leurs noms, jamais la conversation.

```json
{"jsonrpc":"2.0","method":"events/fact","params":{"name":"ticket.column","version":1,"at":"2026-10-29T10:00:00Z","project":{"id":"bench-project","name":"Banc","private":false},"ticket":{"id":"bench-ticket","key":"BEN-1","title":"Un ticket du banc","from":"doing","column":"test"},"agent":{"id":"bench-agent","name":"Agent du banc"}}}
{"jsonrpc":"2.0","id":3,"method":"permissions/decide","params":{"request":{"id":"bench-request","kind":"command","tool":"Bash","input":{"command":"npm test"}},"project":{"id":"bench-project","name":"Banc","private":false},"ticket":{"id":"bench-ticket","key":"BEN-1","title":"Un ticket du banc"},"agent":{"id":"bench-agent","name":"Agent du banc"}}}
```

Un fait est une notification : il n’a pas d’`id`, et le plugin n’y répond pas. Les faits qu’un plugin reçoit dans Escouade, et leurs champs, arrivent avec les événements, dans une prochaine version ; ignore les champs que tu ne connais pas.

## Le faux Escouade

`escouade-plugin dev <dossier>` lance le plugin, l’initialise, puis envoie ce que tu tapes, ligne par ligne, et montre ce qui revient : ses réponses avec leur durée, ses demandes de secrets, ses `log`, sa sortie d’erreur.

| Ligne | Ce qui part |
| --- | --- |
| `fact <nom> [json]` | Le fait `<nom>` (`events/fact`) ; le JSON remplace ses champs, par exemple `fact ticket.column {"project":{"id":"p","name":"Privé","private":true}}`. |
| `decide [json]` | Une demande d’autorisation (`permissions/decide`), et sa réponse ; une réponse au-delà de 500 ms est signalée. |
| `call <méthode> [json]` | Un appel, et sa réponse. |
| `notify <méthode> [json]` | Une notification. |
| `restart` | Tue le plugin et le relance. |
| `quit` | `shutdown`, puis la fin. |

`--secret <clé>=<valeur>` donne la valeur d’un secret (répétable) ; une clé du plugin sans valeur reçoit `null`, comme dans Escouade quand rien n’est gardé, et une clé hors de ses `permissions.secrets` est refusée par `-32001`.

```text
$ escouade-plugin dev slack --secret webhook=https://example.invalid
slack/slack.cmd (pid 30112)
→ initialize
← {"capabilities":{},"protocolVersion":1}  (47 ms)
> fact ticket.column
→ events/fact {"agent":{…},"name":"ticket.column","project":{…},"ticket":{…},"version":1}
→ secrets/get "webhook"
→ log [info] no Slack webhook kept: nothing sent
> call nope {"a":1}
← error -32601 unknown method nope  (0 ms)
> quit
→ shutdown
← null  (0 ms)
```

## En CI

Le banc tient dans une étape : installe Escouade (ou seulement `escouade-plugin`, à côté de l’app), puis

```sh
escouade-plugin validate .
escouade-plugin test .
```

Les tests d’Escouade jouent ce banc sur son faux plugin et sur ses plugins d’exemple. Le [SDK en TypeScript](plugins/sdk) (`@escouade-dev/plugin`) installe la même commande dans les `node_modules` d’un plugin, et sa CI joue ce banc sur les deux gabarits de `npm create escouade-plugin`.
