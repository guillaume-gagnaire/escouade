# Claude Code

Ce plugin fait de Claude Code un harness d’Escouade. Tes agents tournent sur le `claude` de ta machine, avec ton compte, et leur conversation s’affiche dans Escouade : réponses et raisonnement en direct, outils avec leurs diffs, cartes de permission, de question et de plan, coût de chaque tour et quota du compte.

Il s’adresse à qui a déjà Claude Code, avec un abonnement Claude ou une clé API.

## Ce qu’il demande

- Claude Code installé sur la machine (testé de la 2.1.283 à la 2.1.301).
- Un compte Claude Code ajouté dans Réglages › Comptes et harness : sans compte, aucun agent ne démarre.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans accès au disque, au réseau ni aux commandes. Son manifeste ne demande aucune permission.
- Il décrit à Escouade comment lancer `claude`, puis traduit ce que `claude` écrit. C’est Escouade qui lance le programme, jamais le plugin.
- Il reçoit le jeton du serveur MCP d’Escouade de chaque agent qu’il décrit. Il ne le met jamais dans les arguments : `claude` le lit dans un fichier privé de l’agent.
- Il ne voit jamais ta connexion à Claude : Escouade seul la lit.

## Les onglets

- **Installer** : Claude Code, le plugin, un compte.
- **Utiliser** : les modes, la vue Claude Code, les projets privés.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le dossier, les tests, le lancer seul.
