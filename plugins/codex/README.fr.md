# Codex

Ce plugin fait de Codex, l’agent d’OpenAI, un harness d’Escouade. Tes agents tournent sur le `codex` de ta machine, avec ton compte ChatGPT, et leur conversation s’affiche dans Escouade : réponses et raisonnement en direct, commandes et diffs, cartes de permission, de question et de plan, coût estimé de chaque tour et quota du compte.

Il s’adresse à qui a déjà Codex et un compte ChatGPT.

## Ce qu’il demande

- Codex installé sur la machine (`npm install -g @openai/codex`, essayé avec la 0.163.0).
- Un compte Codex ajouté dans Réglages › Comptes et harness : sans compte, aucun agent ne démarre.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans réseau ni commande.
- Il décrit à Escouade comment lancer `codex app-server`, puis traduit ce que Codex écrit. C’est Escouade qui lance le programme, jamais le plugin.
- Il accepte seul `git add` et `git commit` du worktree de l’agent, en Édits auto et en Auto, quand la commande n’est que cela.
- Il lit, par l’hôte d’Escouade, si la configuration et les hooks du dépôt de l’agent sont protégés, et le `config.toml` du compte quand le serveur d’Escouade est refusé à un agent.
- Il reçoit le jeton du serveur MCP d’Escouade de chaque agent qu’il décrit, et le donne à Codex par une variable, jamais dans les arguments.

## Les onglets

- **Installer** : Codex, le plugin, un compte.
- **Utiliser** : les modes, la vue Codex, les projets privés.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le dossier, les tests, le lancer seul.
