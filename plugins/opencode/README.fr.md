# OpenCode

Ce plugin fait d’OpenCode un harness d’Escouade, par son serveur. Tes agents tournent sur l’`opencode` de ta machine, avec les fournisseurs que tu y as connectés, modèles locaux compris, et leur conversation s’affiche dans Escouade : réponses et raisonnement en direct, outils avec leurs diffs, cartes de permission, de question et de plan, coût de chaque tour.

Il s’adresse à qui a déjà OpenCode 1 et au moins un fournisseur connecté.

## Ce qu’il demande

- OpenCode 1 installé sur la machine (`npm install -g opencode-ai`, essayé avec la 1.19.3). OpenCode 2 n’est pas pris en charge.
- Un fournisseur connecté à OpenCode (`opencode auth login`), et son compte ajouté dans Réglages › Comptes et harness.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans fichier ni commande.
- Il décrit à Escouade comment lancer `opencode serve` ; Escouade le lance, une fois pour les agents du compte. Le plugin tire au hasard son mot de passe.
- Il parle à ce serveur sur 127.0.0.1, au seul port qu’Escouade a lu, et à rien d’autre.
- Il donne à chaque session les règles de permission de son mode, la clôture d’un projet privé et la lecture seule d’une discussion. OpenCode n’a pas de bac à sable : ce sont ces règles qui protègent.
- Il donne au serveur, pour le dossier de chaque agent, le serveur MCP d’Escouade avec le jeton de l’agent, jamais dans les arguments.

## Les onglets

- **Installer** : OpenCode, le plugin, le compte.
- **Utiliser** : les modes, la vue OpenCode, les projets privés.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode, et ce que protège son mot de passe.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le dossier, les tests, le lancer seul.
