# escouade-plugin-kit

Ce que partagent les plugins Rust d'[Escouade](https://escouade.dev), pour qu'un plugin ne soit que son client. La même source
compile en process et pour `wasm32-wasip1`, sans tokio ni reqwest dans ce second cas.

- `serve` : la boucle d'un plugin. Les appels répondent quand ils finissent, plusieurs à la fois, et le plugin peut demander à
  Escouade ses secrets et ses réglages pendant qu'il répond (`Cx`).
- `hosts` : le service des hébergeurs de code et des sources du Kanban. Un plugin donne ses genres (`ForgeKind`, `SourceKind`).
- `http` : les appels, et `http::reqwest`, le client d'un plugin. C'est reqwest en process et, dans un module, la part de reqwest
  dont les plugins se servent, sur `net/fetch`.
- `time` : `sleep` et `timeout`, sur tokio en process et sur les minuteurs de l'hôte dans un module.
- `i18n` : la langue de l'interface, dite par `initialize`.

```rust
use escouade_plugin_kit::forge::BoxFuture;
use escouade_plugin_kit::serve::{self, Cx, Plugin, Refusal};
use serde_json::Value;

struct Exemple;

impl Plugin for Exemple {
    fn call<'a>(&'a self, _cx: &'a Cx, method: &'a str, _params: Value) -> BoxFuture<'a, Result<Value, Refusal>> {
        Box::pin(async move { Err(Refusal::not_found(method)) })
    }
}

pub fn run() -> i32 {
    serve::run("exemple", Exemple)
}

escouade_plugin_kit::protocol::module!(run);
```

La référence des plugins : <https://escouade.dev/docs/plugins/api>. Le code est dans [le dépôt d'Escouade](https://github.com/guillaume-gagnaire/escouade),
sous licence GPL-3.0-or-later.
