# Prototype GPUI (ESC-365)

Jetable, limité à la mesure, **jamais livré** : l'étude est `docs/GPUI.md`. Une crate hors de l'espace de travail de
l'app (son propre `[workspace]`), qui ne touche à rien de `src/`, `src-tauri/`, `plugins/` ni `crates/`.

Une fenêtre GPUI, sans webview : une colonne de Kanban qu'on réordonne au glisser-déposer, et des terminaux sur de
vrais PTY (`portable-pty`, comme l'app), émulés par `alacritty_terminal` (comme Zed), avec leurs couleurs.

## Construire et lancer (Windows)

```sh
cd spikes/gpui
cargo build --release          # Rust 1.98.1 (rust-toolchain.toml) : 1.94 refuse gpui-pre 0.3.11
target/release/gpui-spike.exe  # un PowerShell ; SPIKE_TERMINALS=10 pour dix
```

Variables : `SPIKE_TERMINALS=<n>`, `SPIKE_BENCH=flood` (lance le gros flux dans chaque terminal une fois les shells
prêts, et mesure), `SPIKE_FLOOD_LINES=<n>`, `SPIKE_OUT=<fichier>` (mesures en JSON, une par ligne), `SPIKE_QUIT_AFTER=1`.
`gpui-spike.exe --flood <n>` écrit `n` lignes colorées sur sa sortie : c'est le gros flux, lancé dans un terminal.

## Les bancs (`bench/`)

- `startup-spike.mjs` : 10 lancements du prototype, de `spawn` à la première frame (`node bench/startup-spike.mjs 10`).
- `spike.ps1` : démarrage, mémoire au repos, 3 gros flux de 100 000 lignes, dix terminaux.
- `spike-ten.ps1` : dix terminaux au repos, puis tous en flux à la fois (20 000 lignes chacun).
- `tauri.spec.ts` : les mêmes mesures sur l'app d'aujourd'hui, par la fixture des e2e (dossier de données jetable) :

  ```sh
  ESCOUADE_E2E_EXE="$LOCALAPPDATA\\Escouade\\escouade.exe" ESCOUADE_E2E_CDP_PORT=<port libre> ESCOUADE_E2E_SHOW=1 \
    npx playwright test -c spikes/gpui/bench/playwright.config.ts
  ```

  `ESC365_RESTARTS` (9 par défaut), `ESC365_FLOODS` (3), `ESC365_OUT` (`tauri.jsonl`). Le gros flux y est écrit au
  terminal par la commande `term_write`, comme le prototype l'écrit à son PTY : construire le prototype d'abord.
- `mem.ps1 -Root <pid>` : ensemble de travail privé et octets privés d'un process et de son arbre ; `shot.ps1` : la
  capture d'une fenêtre (PrintWindow, rend aussi DirectX et WebView2).

Les résultats et les captures sont dans `results/` ; leur lecture, dans `docs/GPUI.md` §5.

## Ce qu'il ne fait pas

Pas de cœur, d'agents, de conversation ni de serveur MCP : il mesure le coût de **dessiner**. Pas de sélection, de
défilement de l'historique, de recherche, de redimensionnement du PTY (fixé à 120 × 32) ni de saisie au-delà des
touches simples.

Piège appris : ConPTY demande la position du curseur (`ESC[6n`) et n'affiche rien tant qu'on ne lui répond pas ;
l'`EventListener` d'`alacritty_terminal` doit relayer `Event::PtyWrite` au PTY.
