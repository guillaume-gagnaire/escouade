# Le kit Rust

Un plugin en Rust est un module WASM, compilé par le store depuis ses sources, sur les mêmes méthodes qu’un plugin en TypeScript. Cette page dit quelles crates utiliser, comment une source, un hébergeur ou un harness s’écrit, et ce que la compilation refuse.

## Les trois crates

Elles sont sur crates.io en `0.1`, sous licence MIT : un plugin garde le choix de la sienne. Leur documentation est sur docs.rs, pour le natif et pour `wasm32-wasip1`.

| Crate | Ce qu’elle donne | Quand t’en servir |
| --- | --- | --- |
| [`escouade-plugin-kit`](https://docs.rs/escouade-plugin-kit) | La boucle (`serve`), le client http, `sleep` et `timeout`, les tours d’un plugin qui lit un flux (`host`), le service des hébergeurs et des sources (`hosts`), les erreurs typées, la langue. | Presque toujours. |
| [`escouade-plugin-protocol`](https://docs.rs/escouade-plugin-protocol) | Le protocole : ses types, `turn` et `module!`, le contrat des harness (`engine`). Le kit le réexporte (`escouade_plugin_kit::protocol`). | Pour un harness, seul. |
| [`escouade-acp`](https://docs.rs/escouade-acp) | Le client ACP, une machine à états sans process. | Pour un harness qui parle ACP et que le [manifeste seul](plugins/harnesses) ne suffit pas à décrire. |

Un plugin en dépend par version, avec son `Cargo.lock` :

`sdk/create-escouade-plugin/templates/rust/Cargo.toml` (extrait)

```toml
[lib]
name = "plugin"
crate-type = ["cdylib", "rlib"]

[dependencies]
escouade-plugin-kit = "0.1.1"
serde_json = "1"

# Only for `cargo test`, on your machine: the module has no tokio.
[target.'cfg(not(target_family = "wasm"))'.dev-dependencies]
tokio = { version = "1", features = ["io-util", "rt"] }

# The plugin stands alone: a repository with several plugins does not make it a member of anything.
[workspace]
```

La même source compile deux fois : en process sur ta machine pour `cargo test`, avec tokio et reqwest, et pour `wasm32-wasip1` en module, sans eux.

## Un plugin

`Plugin` a une méthode, `call`, que le kit appelle pour tout ce qui arrive, sauf `initialize` et `shutdown`, qu’il répond lui-même :

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (extrait)

```rust
struct Template;

impl Plugin for Template {
    // Every method of Escouade comes here but `initialize` and `shutdown`, which the kit answers. A notification too: what
    // it returns goes nowhere. Nothing may block here: an `.await` gives the turn back to Escouade.
    fn call<'a>(
        &'a self,
        cx: &'a Cx,
        method: &'a str,
        params: Value,
    ) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(async move {
            match method {
                "events/fact" => {
                    fact(cx, &params);
                    Ok(Value::Null)
                }
                "permissions/decide" => Ok(decide(&params)),
                other => Err(Refusal::not_found(other)),
            }
        })
    }
}
```

`Refusal::not_found` répond `-32601` à ce que le plugin ne sert pas. Reste à dire ce qui tourne, et à en faire un module, en une ligne, qui ne produit rien en natif :

`sdk/create-escouade-plugin/templates/rust/src/lib.rs` (extrait)

```rust
/// What the module runs at its first turn: the plugin, kept for the turns to come. Natively, the loop on stdin and stdout.
pub fn run() -> i32 {
    serve::run("plugin", Template)
}

// The one line that makes this crate a module of Escouade. Natively, it is nothing.
escouade_plugin_kit::protocol::module!(run);
```

`npm create escouade-plugin@latest mon-plugin -- --kind rust` écrit tout cela, avec ses tests : voir [un premier plugin](plugins/quick-start). Chaque appel de l’hôte a son exemple dans [les appels](plugins/calls).

## Le modèle par tours

Une instance par plugin, sur un fil, réveillée par ce qui arrive : une ligne d’Escouade, la fin d’un `net/fetch`, un morceau de flux, un message de socket, la sortie d’un programme, un minuteur. Escouade écrit tout ce qui est arrivé sur l’entrée du module, lance **un** tour, puis sert ce que le module a écrit.

**Rien n’attend dans un tour.** Une demande à l’hôte est une ligne, et sa réponse arrive à un tour suivant.

- Un `.await` rend la main. Le kit garde la tâche et la reprend au tour où sa réponse arrive : plusieurs appels avancent à la fois, et un appel lent ne retient pas les autres.
- Ce qui bloque le fil bloque tout le plugin : une boucle, un calcul long, `std::thread::sleep`. Il n’y a pas de thread, et `std::process` échoue. `escouade_plugin_kit::time::sleep` remplace l’attente.
- Il n’y a pas d’environnement : la langue vient d’`initialize`, lue par `escouade_plugin_kit::i18n`, et le dossier d’un agent par `agent/attach`.
- Le réseau passe par `http::reqwest`, la façade du kit : la surface de reqwest dont les plugins se servent, sur `net/fetch`. Le proxy, les certificats et le délai restent à l’hôte, et ce que le plugin en dit n’a pas d’effet. `send` suit les redirections comme reqwest : dix sauts, un GET après 301, 302 et 303, le même appel après 307 et 308. Hors du service, seuls `Accept`, `Accept-Language` et `User-Agent` restent, et un appel qui a encore son corps s’arrête au 3xx. `http::redirect` est pour un client qui décide à chaque saut.
- Une tâche réveillée par une autre repart dans le même tour : `Waker` marche comme ailleurs.
- `serve::run` crée une tâche par appel. Pour lire `net/stream`, garder un socket `ws/*` ou lancer une tâche qu’aucun appel n’a commencée, `host::Host` donne les mêmes tours à un plugin qui sert ses lignes lui-même.

Un tour ne garde rien pour lui : l’état vit dans le plugin, entre deux tours, comme dans un process. Il n’est pas gardé d’une instance à l’autre.

### La porte synchrone

Une machine à états qui doit lire un fichier avant de répondre, comme un harness, a un appel de plus : `escouade.call`, un import du module, servi sur le fil du plugin. `turn::read_file` lit par lui, et `turn::host_call` envoie `fs/*` ou `proc/*`. Il compte dans le temps du tour.

## Les limites

| Limite | Valeur |
| --- | --- |
| Mémoire | 64 Mio |
| Un tour | 2 s, 500 ms quand il tient une décision |
| `net/fetch` | 16 à la fois |
| `net/stream` | 8 à la fois |
| WebSocket | 4 à la fois |
| Minuteurs | 64 à la fois |
| Programmes | 4 à la fois |

Le temps d’attente du réseau ne compte pas : il n’est pas dans un tour. Une instance qui dépasse, ou qui panique, est finie et refaite, avec les mêmes trois relances en cinq minutes qu’un process. Le journal dit pourquoi, et ce qu’elle tenait lui est redonné par `agent/attach`.

## Écrire une source

Une source du Kanban sert `tickets/*`. Le kit fait presque tout : `Hosts` répond le protocole, demande le jeton du compte, garde un client par compte et met les erreurs dans la forme attendue. Un plugin donne sa sorte (`SourceKind`) et un client (`TicketSource`) :

`plugins/linear/src/lib.rs` (extrait)

```rust
impl SourceKind for LinearSource {
    fn id(&self) -> &str {
        ID
    }
    fn name(&self) -> &str {
        S
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn TicketSource> {
        let base = account.api.clone().filter(|a| !a.trim().is_empty());
        Box::new(Linear::new(http, account, base.as_deref().unwrap_or(API)))
    }
}
```

et les branche, dans `run` :

`plugins/linear/src/lib.rs` (extrait)

```rust
pub fn run() -> i32 {
    escouade_plugin_kit::serve::run(
        "linear",
        escouade_plugin_kit::hosts::Hosts::new(vec![], vec![Box::new(LinearSource)]),
    )
}

escouade_plugin_kit::protocol::module!(run);
```

Le manifeste déclare le compte et la source, avec `network` et `account:<nom>` dans `permissions.secrets` :

`plugins/linear/escouade-plugin.json` (extrait)

```json
    "ticketSources": [{ "id": "linear", "name": "Linear", "account": "linear" }]
```

Une erreur http reste une erreur : `HttpError` devient `-(32000 + statut)`, `-32401` pour un jeton refusé, et `Refused` devient `-32010`, un refus durable. Le code entier est dans [`plugins/linear`](https://github.com/guillaume-gagnaire/escouade/tree/main/plugins/linear), avec Jira, Trello et ClickUp à côté.

## Écrire un hébergeur

Un hébergeur de code sert `forge/*`, de la même façon : `ForgeKind` et le client `Forge` du kit, dont chaque méthode répond `Unsupported` tant que tu ne l’écris pas. Le plugin Gitea, qui sert toutes les instances d’un service auto-hébergé, tient en quelques lignes :

`plugins/gitea/src/lib.rs` (extrait)

```rust
impl ForgeKind for GiteaForge {
    fn id(&self) -> &str {
        "gitea"
    }
    fn name(&self) -> &str {
        "Gitea / Forgejo"
    }
    /// An account always names its instance: this is for the bench, whose account is empty, so that it is refused like a
    /// token and not as a malformed call.
    fn default_url(&self) -> Option<&str> {
        Some("https://codeberg.org")
    }
    fn client(&self, http: reqwest::Client, account: &Credentials) -> Box<dyn Forge> {
        let base = account
            .api
            .clone()
            .filter(|a| !a.trim().is_empty())
            .unwrap_or_else(|| api_base(&account.url));
        Box::new(pulls::Gitea::new(http, base, account.token.clone()))
    }
}
```

`account.url` est l’instance que l’utilisateur a saisie, d’où `{account}` dans `permissions.network`. Un jeton refusé, une limite atteinte et le budget d’appels sont traités par `Hosts`. Le banc joue `account/me` et `forge/pullRequestsOfBranch` dès que `contributes.forges` est déclaré.

## Écrire un harness

Pour un agent qui parle ACP, le [manifeste](plugins/harnesses) suffit, sans une ligne de code. Sinon, un harness est une machine à états sans attente, qui ne dépend que du protocole : `turn::run` tient la boucle, sur les deux cibles.

`plugins/codex/src/lib.rs` (extrait)

```rust
pub fn run() -> i32 {
    let mut plugin = Plugin::default();
    turn::run(
        "codex",
        json!({ "harnesses": ["codex"] }),
        move |m, params| plugin.handle(m, params),
    )
}

escouade_plugin_protocol::module!(run);
```

Le contrat est dans `escouade_plugin_protocol::engine`. Le cœur lance le programme de l’agent et lit sa sortie ; le plugin dit comment le lancer, puis traduit ses lignes en éléments de conversation. `std::process::id` se remplace par `turn::instance_id`, et `std::fs` par `turn::read_file`, sous une racine de `permissions.files`. Un client ACP se branche par `escouade_acp::Agent`, dont `frames` lit les lignes de l’agent.

## Ce que la compilation refuse

`escouade-plugin build` compile comme le store, et `escouade-plugin validate` dit la même chose sans rien lancer. Un plugin compile **seul dans son dossier**, sur une copie, avec rustc 1.98.0 et `wasm32-wasip1` que la commande télécharge elle-même, un profil imposé et `--locked` : pour les mêmes sources et le même `Cargo.lock`, le module est le même à l’octet sous Windows et sous Linux.

| Refusé | Pourquoi | Quoi faire |
| --- | --- | --- |
| Pas de `Cargo.lock` | Le store compile ce qu’il dit, et rien d’autre. | Commite-le. |
| Un `build.rs` dans le plugin | Il tournerait en natif sur le store. | Génère le code avant et commite-le. |
| Une macro procédurale à toi | La même raison. | Une macro `macro_rules!`, ou un crate de la liste. |
| Un crate hors de la liste fermée qui tourne à la compilation | Un script, une macro ou ce dont ils dépendent. Ce qui ne tourne qu’en WASM n’a pas besoin d’y être. | Retire-le, ou reste sur les versions de la liste. |
| Une dépendance `git` ou d’un autre registre | Le lock doit venir de crates.io seul. | Publie-la sur crates.io. |
| Un `path` qui sort du dossier | Le store ne lit que le dossier du plugin. | Garde tes sous-crates dans le dossier. |
| `[patch]`, `[replace]` | Le module ne serait plus ce que dit le lock. | Retire-les. |
| `.cargo/config.toml`, `rust-toolchain`, `[profile.*.package]`, `build-override` | La chaîne et le profil sont imposés. | Retire-les. Un `[profile]` simple est permis, et ignoré. |
| Autre chose qu’un `cdylib` (une `rlib` à côté est permise) ; un `[[bin]]`, `src/main.rs`, `src/bin` | Un plugin est un module, sans `main`. | `crate-type = ["cdylib"]`. |
| `module!` manquant | Le module n’a pas l’export `escouade-turn`. | La ligne de `module!`. |

> [!WARNING]
> La liste fermée nomme des versions : `serde` 1.0.229, `syn` 3.0.9… Un `cargo update` qui monte l’un de ces crates peut faire refuser le plugin (« … n’est pas sur la liste relue »). Lance `escouade-plugin validate` et `build` après chaque mise à jour du lock. Le modèle écrit un `Cargo.lock` qui passe.

## Ensuite

- [Tester un plugin](plugins/testing) : `cargo test`, le banc et le faux Escouade.
- [Publier sur le store](plugins/publishing) : l’étiquette, les contrôles, la revue.
