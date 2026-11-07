# Un long tour réel de Claude Code 2.1.312 (ESC-206, INT-0)

Enregistré le 2026-10-30 sous Windows 11 par `node scripts/bench/record-claude-turn.mjs` : `claude -p --output-format stream-json --include-partial-messages --verbose --model sonnet`, sans rien de la configuration de l'utilisateur (`--setting-sources project --strict-mcp-config --disable-slash-commands`), avec les seuls outils de lecture, dans un dossier temporaire où sont copiés trois fichiers du dépôt.

Anonymisé : chemins (`C:\Users\user\repo`) et nom de l'utilisateur.

| Fichier | Contenu |
|---|---|
| `long-turn.jsonl` | 3 371 frames (1,2 Mo), une par ligne, telles qu'elles sont venues : quatre lectures (`Read`, `Grep`) puis une explication d'environ 8 000 mots, en deltas (`stream_event`) |
| `long-turn.times.json` | l'instant de chaque frame, en ms depuis le lancement (138 s en tout) |

C'est le flux le plus long du banc de flux d'INT-0 (`src-tauri/src/fluxbench.rs`, `npm run bench:int0`), rejoué au plus vite et à son rythme.
