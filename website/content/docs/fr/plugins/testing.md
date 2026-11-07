# Tester un plugin

Escouade donne aux auteurs de plugins une commande, `escouade-plugin`, qui fait tourner un plugin sans l’app : le **banc de conformité** le passe au crible de ce qu’Escouade attend de lui, et le **faux Escouade** lui envoie des faits et des demandes à la main. Le banc est celui que joue Escouade sur ses propres plugins.

## La commande

`escouade-plugin` est livrée avec Escouade, à côté de son programme : `%LOCALAPPDATA%\Escouade\escouade-plugin.exe` sous Windows, `Escouade.app/Contents/MacOS/escouade-plugin` sous macOS, `/usr/bin/escouade-plugin` avec le paquet `.deb`. Pour un plugin `run.js` ou `run.wasm`, elle n’a besoin ni de Node ni de Rust : elle compile son module comme le store et le joue en WASM, dans l’hôte de l’app. Pour un plugin lancé par une commande, il lui faut ce qu’il faut à ce programme pour démarrer.

```sh
escouade-plugin validate mon-plugin   # le manifeste, champ par champ
escouade-plugin test mon-plugin       # le banc de conformité
escouade-plugin dev mon-plugin        # le faux Escouade
```

`--lang fr` ou `--lang en` choisit la langue de ses messages ; sans lui, c’est celle du système.

`--escouade <version>` joue le plugin comme cette version d’Escouade, avec `validate`, `test` et `dev` : la plage `escouade` du manifeste se contrôle contre elle, et c’est elle que le plugin voit dans `initialize`. Sans lui, c’est la version de la commande. Il sert à écrire pour une version à venir :

```sh
escouade-plugin test mon-plugin --escouade 2.2.0
```

## Le banc de conformité

`escouade-plugin test <dossier>` lit le manifeste, lance le programme de `run` comme Escouade le lance (dans son dossier, avec ses trois variables, avec tout ce qu’il démarre), et joue jusqu’à onze scénarios. Chacun dit ce qui s’est passé ; un scénario qui échoue dit aussi **ce qui manque**.

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
| Source de tickets | Seulement si `contributes.ticketSources` est déclaré : `account/me` et `tickets/containers`. | Une réponse dans le délai, ou une erreur que le contrat nomme : `-(32000 + statut HTTP)` (`-32401` : jeton refusé), `-32010` (un refus durable), `-32601` (un appel qu’il ne sert pas). |
| Hébergeur | Seulement si `contributes.forges` est déclaré : `account/me` et `forge/pullRequestsOfBranch`. | La même. |

Un scénario qui ne concerne pas le plugin n’est pas joué et le dit : pas de fait sans `permissions.events`, pas de décision sans `permissions.decide`, pas de source ni d’hébergeur sans `contributes.ticketSources` ou `contributes.forges`, et rien d’un programme pour un plugin déclaratif, dont seul le manifeste est lu.

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

- `--json` donne le rapport en JSON : chaque scénario, son verdict (`pass`, `fail`, `skip`), ce qui s’est passé, ce qui manque, la durée de chaque appel, et ce que le plugin a fait de lui-même (sa sortie d’erreur, ses journaux, les secrets demandés ; les 200 derniers). C’est ce que lit la revue « Vérifié par Escouade », qui passe le banc dans un conteneur sans réseau : une connexion ou une écriture refusée s’y voit.
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

## Un plugin en Rust

Trois niveaux, du plus rapide au plus vrai :

```sh
cargo test                                 # le plugin, en natif, sur ta machine
cargo check --target wasm32-wasip1         # il compile en module (rustup target add wasm32-wasip1)
escouade-plugin test mon-plugin            # le banc, sur le module
```

- **`cargo test`** tourne en natif, avec tokio et reqwest : la même source que le module, sans son hôte. `serve_on` du kit joue le plugin sur des lignes en mémoire, les mêmes qu’envoie Escouade. Il lui faut un compilateur C, pour `ring`. Les tests du modèle :
`sdk/create-escouade-plugin/templates/rust/src/tests.rs` (extrait)

```rust
/// What the plugin writes for `messages`, one message a line.
fn play(messages: &[Value]) -> Vec<Value> {
    let input: String = messages.iter().map(|m| format!("{m}\n")).collect();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let (output, written) = tokio::io::duplex(64 * 1024);
        serve_on(Arc::new(Template), input.as_bytes(), output)
            .await
            .unwrap();
        let mut lines = BufReader::new(written).lines();
        let mut read = Vec::new();
        while let Some(line) = lines.next_line().await.unwrap() {
            read.push(serde_json::from_str(&line).unwrap());
        }
        read
    })
}
```

- **`wasm32-wasip1`** est la cible du module. `cargo check --target wasm32-wasip1` dit vite ce qui n’y compile pas : `tokio`, `std::process`, une dépendance qui suppose un système. Ce n’est pas le module publié : celui-là sort de `escouade-plugin build`.
- **Le banc et le faux Escouade** compilent le module comme le store, puis le jouent dans l’hôte de l’app. Une erreur du store se voit donc ici : pas de `Cargo.lock`, un crate hors de la liste. La chaîne d’outils, rustc 1.98.0 et `wasm32-wasip1`, est téléchargée au premier besoin dans `~/.escouade/tools` (650 Mo, une minute) et vérifiée à chaque usage, sans rustup. `ESCOUADE_RUST_TOOLCHAIN` en nomme une autre.

`--wasm <module>` joue un module donné au lieu de compiler ; `--offline` refuse tout appel réseau, comme la revue ; `--account <adresse>` donne une adresse à `{account}`. Le banc sert `fs/*` et `proc/*` autour d’un agent à lui, et `secrets/get`. Il ne sert pas `settings/get`, qui répond `-32601` : un plugin doit y survivre.

## En CI

Le banc tient dans une étape : installe Escouade (ou seulement `escouade-plugin`, à côté de l’app), puis

```sh
escouade-plugin validate .
escouade-plugin test .
```

Les tests d’Escouade jouent ce banc sur son faux plugin et sur ses propres plugins (`plugins/` de son dépôt), avec `--first-party` : leurs identifiants `dev.escouade.*` sont réservés, le banc les refuse à tout autre plugin. Le [SDK en TypeScript](plugins/sdk) (`@escouade-dev/plugin`) installe la même commande dans les `node_modules` d’un plugin, et sa CI joue ce banc sur les gabarits de `npm create escouade-plugin`.
