# Le prototype de « tous les plugins en WASM » (ESC-330)

L'étude, les capacités, les décisions et le plan sont dans `docs/PLUGINS.md` §8. Ce dossier n'est construit par rien dans
l'app : il prouve le chemin, et garde de quoi refaire chaque mesure.

**Depuis ESC-334, ce chemin est dans les crates du dépôt** : `escouade-turn`, le `serve` et la façade de reqwest ont rejoint
`crates/`, et `sh scripts/plugins-wasm.sh` compile Linear et Codex sans `generated/`. `prepare.mjs` cherche le code d'avant et
échoue : pour refaire ce qui suit, partir du commit `de8eba05`.

- `guest/` : ce qui compile en `wasm32-wasip1`.
  - `linear/`, `codex/` : les plugins du dépôt. `generated/` est leur source, copiée par `prepare.mjs`, qui imprime ce
    qu'il change : le prix du portage.
  - `kit/` : `crates/escouade-plugin-kit`, ses fichiers tels quels (`src/generated/`), sauf `serve.rs`, réécrit sur un
    fil, sans tokio.
  - `reqwest/` : la part de reqwest dont les plugins se servent, sur `net/fetch`.
  - `escouade-turn/` : le tour du réacteur, l'appel synchrone `escouade.call`.
- `rustc-canon/` : l'enveloppeur de rustc qui rend la compilation reproductible d'un système à l'autre (§8.5.2).
- `reproducible.sh` : une compilation, comme le store et la revue la feraient.
- `host/` : l'hôte (`src/lib.rs`, ce que `src-tauri/src/plugins/wasm.rs` deviendrait), les deux plugins natifs sur leurs
  vraies crates (`src/bin/`), les tests (`tests/parity.rs`), le banc (`examples/measure.rs`).
- `results/` : les mesures brutes des trois passes du §8.9.

Aucun identifiant réel : le banc parle à un faux Linear local et rejoue `tests/fixtures/codex/`.

## Refaire tout (Windows, Git Bash)

```sh
node prepare.mjs                                # les sources de guest/*/generated/
sh reproducible.sh                              # target/reproducible/wasm32-wasip1/release/{linear,codex}.wasm
cd host
cargo test --release                            # les 5 tests : les modules contre leur natif
cargo run --release --example measure > ../results/measures-1.json
```

Les modules des crates passent par l'hôte de l'app : après `sh scripts/plugins-wasm.sh` et `cargo build --bins`,
`ESCOUADE_PARITY=1 cargo nextest run --lib plugins_wasm_parity` dans `src-tauri/`. Sans `ESCOUADE_PARITY`, ces tests se
sautent quand les modules manquent.

`reproducible.sh` installe rust 1.98.0 en profil `minimal` dans `target/rustup` (avec `rust-src`, l'empreinte change),
sans toucher à la chaîne d'outils de l'utilisateur. `--plain` compile sans `rustc-canon`, pour voir ce qu'il change.

## La même empreinte sous Linux

Avec Podman (sous Windows, `export MSYS_NO_PATHCONV=1` d'abord ; `W` est le dossier du dépôt) :

```sh
I=docker.io/library/rust:1.98.0-bookworm
podman volume create esc330-tools
# Une fois, avec le réseau : la cible et les dépendances, rien du plugin ne tourne.
podman run --rm -v esc330-tools:/v -v "$W:/src:ro" $I sh -c 'cp -r /usr/local/rustup /v/rustup
  export RUSTUP_HOME=/v/rustup CARGO_HOME=/v/cargo
  cd /src/prototypes/wasm-all/guest && rustup target add wasm32-wasip1 && cargo fetch --locked
  cd ../rustc-canon && cargo fetch --locked'
# Sans réseau, en lecture seule, sans droits : autant de fois qu'on veut.
podman run --rm --network=none --read-only --tmpfs /tmp:exec --user 1000:1000 \
  -v esc330-tools:/v:ro -v "$W:/src:ro" -e HOME=/tmp -e RUSTUP_HOME=/v/rustup -e ESCOUADE_RUSTUP_HOME=/v/rustup \
  -e CARGO_HOME=/v/cargo -e CARGO_NET_OFFLINE=true -e ESCOUADE_OUT=/tmp/o $I \
  sh -c 'cd /src/prototypes/wasm-all && sh reproducible.sh'
```

Attendu, sous les deux systèmes : `5ae4aabb…` pour `linear.wasm`, `c5ab5a0a…` pour `codex.wasm`, tant que les sources et
les deux `Cargo.lock` ne changent pas. WSL ne répondait pas sur ce poste : Podman a donné le Linux de l'essai.
