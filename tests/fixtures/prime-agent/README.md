# Flux réels de Prime Agent 0.10.3 (ESC-485)

Enregistrés le 2026-11-16 dans une distribution WSL jetable (Ubuntu 24.04, sans accès au disque de Windows), Prime Agent installé par son script officiel (`install.sh`, canal stable 0.10.3), sur un modèle local : Ollama de la machine hôte, `qwen3-coder:30b-32k`, déclaré dans `~/.prime/agent/models.json` comme fournisseur `ollama` (`api: openai-completions`). Aucun compte, aucun `/login`, aucune clé. La télémétrie de Prime Agent était coupée (`prime-agent telemetry off`).

Chaque ligne est `{dir, msg}` : `c2s` ce que le client de l’étude a envoyé, `s2c` ce que `prime-agent --mode acp` a répondu.

| Fichier | Ce qui s’y passe |
|---|---|
| `hello.jsonl` | `--no-session` : `initialize`, `session/new`, un tour de texte seul. |
| `full-turn.jsonl` | `--no-session`, dans un dossier où `calc.py` a un bug : six cellules Python (lire, corriger, lancer le test), puis la réponse. |
| `mcp.jsonl` | `--no-session` : un serveur MCP HTTP donné à `session/new` (le faux serveur `escouade` de l’étude, jeton `jeton-esc485`), `session/set_config_option` du modèle, `session/set_mode` refusé, deux appels MCP depuis Python, un tour arrêté par `session/cancel` (`cancelled`), un tour de plus. |
| `config-options.jsonl` | Un second modèle déclaré `reasoning` : l’option `thought_level` paraît, `thinking` est refusé, `thought_level` accepté. Aucun tour. |
