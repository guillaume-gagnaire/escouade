# Copilot CLI

Ce plugin fait de GitHub Copilot CLI un harness d’Escouade. Tes agents tournent sur le `copilot` de ta machine, avec ton compte GitHub, et leur conversation s’affiche dans Escouade : réponses et raisonnement en direct, commandes et diffs, cartes de permission, de question et de plan, formulaires des serveurs MCP, coût en crédits d’IA de chaque tour et crédits du mois.

Il s’adresse à qui a un compte GitHub avec Copilot, Free compris.

## Ce qu’il demande

- Copilot CLI installé sur la machine (`npm install -g @github/copilot`, essayé avec la 1.0.98).
- Un compte Copilot CLI ajouté dans Réglages › Comptes et harness, connecté par `copilot login` : sans compte, aucun agent ne démarre.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans réseau ni commande.
- Il décrit à Escouade comment lancer `copilot --server --stdio`, puis traduit ce que Copilot écrit. C’est Escouade qui lance le programme, jamais le plugin.
- Il répond seul aux demandes sûres : une lecture dans le dossier de l’agent, une écriture là hors de `.git` et hors du mode Plan, et `git add` et `git commit` du worktree en Édits auto et en Auto.
- Il refuse aussitôt une demande qui nomme un chemin d’un autre projet quand l’un des deux est privé.
- Il lit, par l’hôte d’Escouade, si la configuration et les hooks du dépôt de l’agent sont protégés.
- Il reçoit le jeton du serveur MCP d’Escouade de chaque agent qu’il décrit, et le donne à Copilot par une variable, jamais dans les arguments.

## Les onglets

- **Installer** : Copilot CLI, le plugin, un compte.
- **Utiliser** : les modes, la vue Copilot CLI, les projets privés.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le dossier, les tests, le lancer seul.
