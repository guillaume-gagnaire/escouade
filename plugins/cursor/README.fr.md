# Cursor CLI

Ce plugin fait de Cursor CLI un harness d’Escouade, par ACP. Tes agents tournent sur le `agent` de ta machine, avec ton compte Cursor, et leur conversation s’affiche dans Escouade : réponses et raisonnement en direct, outils avec leurs diffs, cartes de permission, de question et de plan, tâches et sous-agents.

Il s’adresse à qui a un compte Cursor, Free compris.

## Ce qu’il demande

- Cursor CLI installé sur la machine (essayé avec la 2026.10.22), connecté par `agent login`.
- Un compte Cursor CLI ajouté dans Réglages › Comptes et harness : le compte de la machine, ou un autre par sa clé d’API.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans réseau, sans fichier et sans commande.
- Il décrit à Escouade comment lancer `agent acp`, puis traduit ce que Cursor écrit. C’est Escouade qui lance le programme, jamais le plugin.
- Il accepte seul, en Édits auto, les écritures que Cursor soumet dans le dossier de l’agent (hors `.git`), et tout en Bypass.
- Il refuse aussitôt une demande qui nomme un chemin d’un autre projet quand l’un des deux est privé.
- Il reçoit le jeton du serveur MCP d’Escouade de chaque agent qu’il décrit, pour le donner à la session de Cursor, jamais dans les arguments.

Cursor lit et écrit les fichiers lui-même, sans les faire passer par Escouade : ce qu’il ne demande pas, Escouade ne peut pas l’empêcher.

## Les onglets

- **Installer** : Cursor CLI, le plugin, un compte.
- **Utiliser** : les modes, la vue Cursor CLI, les projets privés.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le dossier, les tests, le lancer seul.
