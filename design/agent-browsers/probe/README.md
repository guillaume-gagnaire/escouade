# La sonde d'ESC-663

Ce que pourrait faire un agent d'Escouade, par le cœur, dans une vraie webview de ce système, et ce qu'une page hostile peut faire contre lui. Un crate à part (son propre workspace), **hors du produit** : la CI de l'app ne le construit pas.

```sh
cargo run -- <dossier des captures>   # « out » par défaut
```

- Les versions de wry (0.57.0) et tao (0.37.1) sont celles de Tauri dans l'app (`src-tauri/Cargo.lock`).
- Elle ouvre sa fenêtre et un serveur sur un port libre de la boucle locale, déroule ses étapes, écrit une ligne JSON par étape sur la sortie, puis le rapport `report-<os>.json` et les captures dans le dossier. Elle s'arrête seule et ne lance aucun autre programme.
- Sous Windows (WebView2) : le protocole DevTools, dans le process. Sous Linux (WebKitGTK) : `run_javascript_in_world` et `snapshot` ; il faut `libwebkit2gtk-4.1-dev` et un affichage (WSLg, ou `xvfb-run -a`). Sous macOS (WKWebView) : `WKContentWorld` et `takeSnapshot`. Cette branche compile (clippy en ciblant `x86_64-apple-darwin`), mais elle n'a pas encore tourné sur un Mac.
- Si D: est plein, mettre `CARGO_TARGET_DIR` ailleurs.

Les résultats du 2026-11-25 (Windows 11 et WebKitGTK 2.52 sous WSL) sont dans `../results/`, et leur lecture dans `docs/AGENT-BROWSERS.md` § 7.

Pour contrôler la branche macOS sans Mac (types et clippy, rien ne se lie ni ne tourne), mettre dans `CC_x86_64_apple_darwin` et `AR_x86_64_apple_darwin` un programme qui écrit un fichier vide là où on le lui demande (`-o <fichier>`, ou le premier `.a`). Il remplace le fichier Objective-C d'`objc2-exception-helper`. Puis lancer `cargo clippy --target x86_64-apple-darwin -- -D warnings`.
