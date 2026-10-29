---
title: "Stack"
subject: "Introduction"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Les choix techniques d'Escouade, couche par couche.

| Couche | Choix |
|---|---|
| Shell applicatif | Tauri 2 (Windows 10 / 11, macOS 11+, Linux) |
| Backend | Rust (tokio) : PTY (portable-pty), git (gix/CLI), watchers (notify), SQLite (rusqlite), trousseau du système (keyring), corbeille (trash), serveur MCP (`rmcp` 3.5 derrière `hyper`), langue du système (`sys-locale`) |
| Frontend | Svelte 5 + TypeScript + Vite ; textes par catalogues maison |
| Éditeur | CodeMirror 6 (`@codemirror/merge` pour les changements dans le texte) |
| Terminaux | xterm.js (addon WebGL) |
| Markdown | marked (GFM) + DOMPurify ; pendant le streaming, le message est rendu de nouveau en entier, au plus une fois par image ; coloration syntaxique (shiki) une fois le texte final |
| Stockage | `~/.escouade/` : config JSON et SQLite |
