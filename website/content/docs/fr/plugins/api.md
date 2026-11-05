# Le protocole

Un plugin qui a un programme parle à Escouade en JSON-RPC 2.0 : dans l’app, en WASM, s’il est en JS, en TypeScript ou en Rust ; sur son entrée et sa sortie standard s’il est un process. Cette page décrit les messages et le process.

Un plugin `run.js` ou `run.wasm` parle le même protocole : [ce qui change en WASM](#un-programme-en-wasm) vient plus bas, et [les appels](plugins/calls) donnent un exemple en TS et en Rust pour chaque méthode.

## Quand le programme démarre

- **À sa première activation, jamais avant.** Escouade lance le programme de `run` quand un fait de sa liste `activation` se produit (`event:ticket.column`), ou au premier appel qu’il lui fait. Un plugin sans `activation` ne tourne pas ; un plugin sans `run` est déclaratif : rien ne s’exécute.
- **Un process par plugin.** Il sert tous les projets et tous les agents où le plugin est actif : Escouade distingue ses appels par leur `id`.
- **Dans le dossier du plugin**, avec trois variables : `ESCOUADE_PLUGIN_ID` (son id), `ESCOUADE_PLUGIN_API` (la version du protocole, `1`) et `ESCOUADE_VERSION` (celle d’Escouade).
- **Avec tout ce qu’il lance.** Sous Windows, le process est dans un Job object ; sous macOS et Linux, il mène son groupe de processus. Quand Escouade l’arrête, ce qu’il a lancé s’arrête avec lui ; sous Windows, même si Escouade plante.

## Les messages

- Du JSON-RPC 2.0, **un message par ligne** : du JSON en UTF-8, sans retour à la ligne, terminé par `\n`. La sortie standard ne sert qu’au protocole ; écris tes traces sur la sortie d’erreur, qui va au [journal](#le-journal).
- **Des lots dans les deux sens** : un tableau JSON de messages, sur une ligne. Réponds à un lot par un lot ; les réponses peuvent venir dans n’importe quel ordre, chacune avec l’`id` de sa requête.
- Une notification n’a pas d’`id` et n’attend pas de réponse.

```json
{"jsonrpc":"2.0","id":7,"method":"echo","params":{"a":1}}
[{"jsonrpc":"2.0","id":8,"method":"echo","params":1},{"jsonrpc":"2.0","method":"log","params":{"level":"info","message":"prêt"}}]
```

## La poignée de main

Le premier message d’Escouade est `initialize`. Il donne la version du protocole qu’il parle et ce qu’il sait faire :

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": 1,
    "escouade": { "version": "2.1.1" },
    "plugin": { "id": "dev.escouade.slack", "version": "1.1.0" },
    "lang": "fr",
    "capabilities": { "batch": true, "secrets": true, "settings": true, "log": true, "attach": true }
  }
}
```

Le plugin répond la version qu’il parle et ses propres capacités :

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"capabilities":{"batch":true}}}
```

- `lang` est la langue de l’interface (`fr`, `en`), pour ce que le plugin écrit à l’utilisateur.
- Une autre version, ou une réponse sans version, arrête le plugin, avec la raison : « it speaks version 2 of the protocol; this Escouade speaks version 1 ».
- Sans réponse en 10 secondes, le plugin est arrêté aussi.
- Rien d’autre ne lui est envoyé avant sa réponse : ce qui arrive entre-temps attend dans une file.

## Les délais

Chaque appel d’Escouade a un délai. Passé ce délai, Escouade n’attend plus : **sa réponse par défaut s’applique** (une demande d’autorisation va à l’utilisateur, comme sans plugin), et une réponse tardive est ignorée.

| Méthode | Délai |
| --- | --- |
| `initialize` | 10 s |
| `shutdown` | 1 s |
| `permissions/…` (une décision qui fait attendre un agent) | 500 ms |
| `tickets/…`, `forge/…`, `workflow/…`, `engine/oneShot` (un import, un appel à un service) | 60 s |
| Les autres | 5 s |

Chaque délai dépassé est noté au journal : « test/do: no answer within 300 ms ».

## Ce qu’Escouade sert au plugin

| Méthode | Paramètres | Réponse |
| --- | --- | --- |
| `secrets/get` | `{ "key": "webhook" }` | Le secret, ou `null` si aucun n’est gardé sous cette clé. |
| `settings/get` | `{}`, ou `{ "project": "<id>" }` | Ses réglages, tels que l’utilisateur les a choisis dans l’onglet Réglages de sa page : un objet par clé de `contributes.settings`, sans celles laissées à leur valeur par défaut. Avec un projet (le `project.id` d’un fait), ceux que son [`.escouade.yml`](escouade-yml) lui donne sous `plugins.<id>` passent devant, s’il est activé dans ce projet : seulement les clés qu’il déclare, d’une valeur de leur type, jamais un secret. |
| `log` (notification) | `{ "level": "info", "message": "…" }` | Aucune : la ligne va au journal du plugin. |
| `files/read` | `{ "path": "data/words.txt" }` | `"…"`, le texte d’un fichier de son dossier, en texte UTF-8, 256 Ko au plus, aux deux transports. Refusé (`-32003`, gardé au journal) : un chemin absolu, avec `..` ou un lecteur, un lien qui mène hors du dossier, un nom caché (`.env`, `.git`), le manifeste. `-32005` pour un fichier absent, `-32004` pour un fichier trop gros ou qui n’est pas du texte. |
| `net/fetch` | `{ "url": "https://…", "method": "POST", "headers": {}, "body": "…" }` | `{ "status": 200, "headers": {}, "body": "…" }`, pour un plugin en WASM seulement : voir [le réseau](#le-reseau-par-escouade). |
| `net/stream`, `ws/*`, `timer/*`, `fs/*`, `proc/*` | Voir [les appels](plugins/calls). | Pour un plugin en WASM seulement ; un process reçoit `-32601`. |

Une autre méthode répond l’erreur `-32601` et le note au journal. Les erreurs propres à Escouade : `-32001` pour un secret qui n’est pas celui du plugin, `-32002` pour un trousseau injoignable, `-32003` pour ce que le plugin ne peut pas atteindre (un hôte, un fichier), `-32004` pour une requête qui échoue en route, `-32005` pour un fichier absent, `-32602` pour des paramètres qui ne tiennent pas.

## Les secrets

Les secrets d’un plugin sont dans le trousseau du système, sous un nom à lui : `plugin:<id>:<clé>`. Il les demande par `secrets/get`, au moment où il en a besoin.

- **Seulement les siens.** La clé doit être l’une de ses `permissions.secrets` ; un motif finissant par `*` en prend plusieurs (`account:*`). Tout autre clé est refusée par `-32001` et notée au journal, même celle d’un autre plugin écrite en entier : `plugin:dev.autre:token` n’est pas une clé de ses permissions.
- Un plugin n’obtient jamais le secret d’un compte, d’un harness ou d’une intégration d’Escouade.
- Un secret donné au plugin est masqué (`••••`) partout dans son journal, sa sortie d’erreur comprise.

## Les plantages et les relances

- **Un plantage relance le plugin**, trois fois en cinq minutes. Au quatrième, il est arrêté : « stopped after 4 crashes within 5 minutes ». Il ne redémarre plus seul ; le bouton « Relancer » de son journal, dans la vue Plugins, le relance, ses plantages oubliés.
- **Les appels en cours** quand il plante échouent : leur réponse par défaut s’applique.
- **Pendant la relance**, ce qu’Escouade lui envoie attend dans une file de 256 messages, puis part dans l’ordre une fois `initialize` répondu. Une file pleine perd son plus vieux message, et le journal le dit.
- **Ce que chaque agent lui avait attaché** (pour un harness, `agent/attach` : sa session, son mode, son modèle) est redonné au nouveau process avant tout autre message.

## La mémoire

Escouade lit la mémoire du plugin et de tout ce qu’il a lancé toutes les deux secondes, comme celle des agents. Au-delà de 1 Go, le plugin est arrêté, sans relance : « stopped: 1100 MB of memory, past the limit of 1024 MB ».

## La fermeture d’Escouade

Quand Escouade se ferme, il envoie `shutdown` à chaque plugin qui tourne, attend sa réponse une seconde au plus pour tous, puis arrête son process et tout ce qu’il a lancé. Réponds à `shutdown`, puis termine-toi : ce que tu n’as pas écrit avant est perdu.

## Le journal

Chaque plugin a son journal, dans `~/.escouade/plugin-logs/<id>.log` ; au-delà de 1 Mo, il devient `<id>.log.1` et repart de zéro. Ses lignes sont en anglais, comme le journal de l’app, et notent :

- ses démarrages, sa réponse à `initialize`, ses arrêts et leur raison ;
- ses plantages, avec leur code de sortie et ses dernières lignes de sortie d’erreur ;
- chaque ligne de sa sortie d’erreur ;
- ses messages : ses `log`, les secrets qu’il a demandés (jamais leur valeur) ;
- les appels restés sans réponse, ce qu’il a demandé et qu’on lui a refusé, les messages perdus d’une file pleine.

Les 500 dernières lignes se lisent dans l’onglet Journal de sa page, dans la vue Plugins ([installer un plugin](plugins/installer#le-journal)).

Ses méthodes, leurs paramètres, leurs réponses et leurs délais sont décrits par un schéma, [`plugin-protocol-1.json`](/schemas/plugin-protocol-1.json), d’où viennent les types du [SDK en TypeScript](plugins/sdk). Pour vérifier qu’un plugin tient ce protocole sans lancer l’app, joue [le banc de conformité](plugins/testing) : `escouade-plugin test <dossier>`.

## Un programme en WASM

Un plugin dont le manifeste dit `"run": { "js": "dist/plugin.mjs" }` ou `"run": { "wasm": "Cargo.toml" }` ne lance pas de process : Escouade fait tourner son module **dans l’app, en WASM**, sans Node sur la machine. Une instance par plugin, sur un fil à elle, qui garde son état d’un appel à l’autre comme un process. Le protocole est le même : les mêmes messages, portés par l’entrée et la sortie standard du module, la même poignée de main, les mêmes délais, le même journal et les mêmes relances ; [le SDK](plugins/sdk) et [le kit Rust](plugins/rust) le parlent pour toi.

Le module est compilé une fois : par le store pour un plugin du store (l’app le télécharge et vérifie son empreinte contre `version.json`), par l’app pour un `run.js` installé depuis un dossier ou un dépôt git. Un `run.wasm` hors du store vient avec son `plugin.wasm`, à côté du manifeste. L’app le précompile à l’installation, le garde dans son cache et le précompile à nouveau après une mise à jour d’Escouade. Il lui faut Escouade 2.2.0 ou plus récent.

### Le réseau, par Escouade

Un plugin en WASM n’ouvre pas de connexion : tout passe par Escouade, par `net/fetch`, `net/stream` et `ws/open`, que seul un plugin en WASM reçoit (un process a son propre réseau, et ces méthodes lui répondent `-32601`). `fetch` en JS et le client du kit en Rust y mènent. Escouade envoie la requête :

- **en https seulement**, vers un hôte de `permissions.network` ou l’un de ses sous-domaines (`slack.com` couvre `api.slack.com`) ; jamais vers une adresse IP écrite dans l’URL, que `permissions.network` ne nomme pas ;
- **sans suivre de redirection** : le plugin reçoit la réponse 3xx ;
- **en 10 s, pour 1 Mio de réponse au plus**, sauf si `limits` en demande plus ;
- **jamais vers une adresse de la machine ou du réseau local**, même pour un hôte déclaré : bouclage, privée, de lien local, non spécifiée, de multidiffusion ou de diffusion, en IPv4 comme en IPv6. Escouade juge les adresses que la connexion emploie elle-même, après la résolution du nom, et non une résolution à part que le DNS pourrait contredire ;
- **par le proxy de Réglages › Réseau**, s’il y en a un : seul le nom est alors jugé, et le journal le dit.

`{account}`, l’hôte d’un compte que l’utilisateur a saisi, et `local:shared`, le serveur local d’un harness, ouvrent http et le réseau local pour ces hôtes seuls : voir [les capacités de l’hôte](plugins/capabilities#le-reseau).

Un appel refusé répond `-32003` et le journal le garde (« slack.com resolves to 127.0.0.1, an address of this machine or of the local network », « example.org is not in permissions.network of … ») ; un appel qui échoue en route (pas de réponse à temps, une réponse trop grande, un hôte injoignable) répond `-32004`. Le plugin voit un `fetch` qui rejette.

### Les fichiers de son dossier

- **Les fichiers JSON de sa racine** (`rules.json`, `channels.json`) lui sont donnés au démarrage : 256 Ko chacun, 1 Mo en tout, jamais son manifeste. `readFileSync` de `node:fs` les lit, comme sous Node (`readFileSync(new URL('../rules.json', import.meta.url), 'utf8')`).
- **Les autres** se lisent par `files/read` (`p.file()` du SDK), en texte UTF-8, 256 Ko au plus, un sous-dossier compris (`data/words.txt`).
- **Le reste** passe par `fs/*`, sous les racines de `permissions.files` : voir [les fichiers](plugins/capabilities#les-fichiers-files).

### Les limites

- **64 Mo de mémoire, 2 s par tour de travail**, 500 ms quand il tient une décision.
- **Une boucle sans fin, une mémoire qui grossit ou une exception non rattrapée** finissent l’instance, et le journal dit pourquoi (« a turn ran past 2000 ms: Escouade stopped its instance », « its instance crashed: … ») ; une nouvelle est faite, comme après le plantage d’un process, avec les mêmes trois relances en cinq minutes, puis le plugin est arrêté.
- **Des minuteurs** : `setTimeout` et `clearTimeout` existent en JS, `sleep` et `timeout` dans le kit, 10 ms au moins, 64 à la fois.
- **Un bundle fait avec le SDK 0.2** ne tourne pas en WASM : le plugin s’arrête et son journal dit de le reconstruire avec `@escouade-dev/plugin` 0.3 ou plus récent.

### Ce qu’un plugin en WASM ne peut pas faire

Rien de ce qu’il ne déclare pas : ni réseau, ni fichier hors de son dossier, ni programme. Il ne lit pas l’environnement et ne charge aucun module natif ou distant. Un bundle qui importe autre chose que `node:fs` et `node:readline`, ou qui a des `export`, n’est pas compilé, et le plugin s’arrête avec la raison. Ses secrets et ses réglages sont les siens seulement, comme pour un process. C’est pourquoi la fenêtre d’installation dit « Tourne dans Escouade, isolé », puis chaque [capacité](plugins/capabilities) qu’il demande, et non « Lance un programme sur ton ordinateur, avec tes droits ».

## Ce que l’utilisateur change

Un plugin désactivé, une permission retirée ou rendue, une mise à jour installée : son process est arrêté, et le suivant reçoit le manifeste et les permissions de maintenant. Une permission de secret retirée fait répondre `-32001` à `secrets/get` pour cette clé, comme pour un secret qui n’est pas le sien.
