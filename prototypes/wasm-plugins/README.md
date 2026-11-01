# Le prototype du runtime WASM des plugins JS/TS (ESC-319)

L'étude, les mesures et les décisions sont dans `docs/PLUGINS.md` §5. Ce dossier n'est pas construit par l'app : il
prouve le chemin et garde de quoi refaire les mesures.

- `host/` : l'hôte (`src/lib.rs`, ce qui irait dans `src-tauri/src/plugins/wasm.rs`) et le prélude JS
  (`src/prelude.js`) ; `tests/plugins.rs`, les `dist/plugin.mjs` de `plugins/team-rules` et `plugins/slack`, inchangés,
  compilés et joués en WASM ; `examples/bench.rs`, les mesures de Javy et de Node ; `src/bin/run.rs`, le moins de
  l'hôte, pour le peser.
- `escouade-js/` : le plugin Javy d'Escouade (QuickJS de Javy 9.1, l'export `escouade-turn`), construit pour
  `wasm32-wasip1` par `build-escouade-js.sh`.
- `extism/`, `component/` : les mêmes bundles dans Extism et en composant (componentize-js), mesurés pareil.
- `weight/` : le témoin des poids (les mêmes crates, sans runtime).

## Refaire les mesures (Windows, Git Bash)

Les outils épinglés, chacun dans son dossier, leur SHA-256 vérifié contre celui de leur release :

```sh
T=$(cygpath -u "$TEMP")/esc319
mkdir -p $T/dl-javy && cd $T/dl-javy
curl -sLO https://github.com/bytecodealliance/javy/releases/download/v9.1.0/javy-x86_64-windows-v9.1.0.gz
curl -sLO https://github.com/bytecodealliance/javy/releases/download/v9.1.0/javy-x86_64-windows-v9.1.0.gz.sha256
sha256sum javy-x86_64-windows-v9.1.0.gz; cat javy-x86_64-windows-v9.1.0.gz.sha256   # 7148baab…
gunzip -k javy-x86_64-windows-v9.1.0.gz && mv javy-x86_64-windows-v9.1.0 javy.exe
# extism-js 1.7.0 (409ac023…) dans $T/dl-extism/extism-js.exe, binaryen version_133 (17a2cbea…) dans $T/dl-binaryen
# componentize-js 0.23.0 : npm install dans $T/dl-cjs (il lui faut Node)
```

Puis, depuis ce dossier :

```sh
rustup target add wasm32-wasip1 --toolchain 1.98.0
sh build-escouade-js.sh                         # target/escouade-js.wasm
cargo test -p escouade-wasm-host                # le prototype : 9 tests
cargo run --release --example bench -p escouade-wasm-host
PATH="$T/dl-binaryen/binaryen-version_133/bin:$PATH" cargo run --release -p escouade-wasm-extism
(cd component && node componentize.mjs "$(cygpath -m $T/dl-cjs)" <bundle> ../host/src/prelude.js glue.js wit/plugin.wit <out>)
cargo run --release -p escouade-wasm-component -- <team-rules>.component.wasm <slack>.component.wasm
cargo build --release --workspace --bins        # les poids : run.exe, escouade-wasm-*.exe contre escouade-wasm-weight.exe
```

`ESCOUADE_JAVY` et `ESCOUADE_JS` nomment un autre Javy et un autre `escouade-js.wasm`.
