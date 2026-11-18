# Unharnessed

Ce plugin fait d’Unharnessed, l’agent de code de Rayane Rachid qui n’a qu’un outil (un REPL Python où le modèle écrit tout le reste), un harness d’Escouade. Il ne contient pas de code : son manifeste décrit Unharnessed par ACP (`unharnessed --mode acp`), et le plugin ACP d’Escouade (`dev.escouade.acp`) le lance, lui parle et montre sa conversation : réponses en direct, chaque cellule Python avec son code et sa sortie, cartes de question.

**Unharnessed ne demande jamais rien, et son noyau n’est pas un bac à sable** : son code et ses commandes tournent avec tes droits, sur tout le disque. Son seul mode dans Escouade est Bypass. Lance-le dans un worktree jetable, un conteneur ou une machine virtuelle (onglet Permissions).

## Ce qu’il demande

- Unharnessed sur la machine (`unharnessed --version` répond 0.9.8 ou plus) et `uv` dans le `PATH`, avec un fournisseur de modèles configuré. Il n’a pas d’installeur officiel : l’extension VS Code (macOS Apple Silicon seulement) ou la construction depuis la source (onglet Installer).
- Le plugin ACP (`dev.escouade.acp`) installé et allumé.
- « Autoriser le mode Bypass » allumé dans Réglages › Agents.
- Un compte Unharnessed ajouté dans Réglages › Comptes et harness.

## Ce qu’il peut faire

- Rien de lui ne s’exécute : c’est un plugin sans programme.
- Ses agents lancent le programme `unharnessed --mode acp` de ta machine, avec tes droits.
- Il ne demande ni réseau, ni secret, ni fichier. Escouade ne lit aucun identifiant d’Unharnessed.

Pas de discussion de l’onglet Docs, et aucun agent Unharnessed dès qu’un de tes projets est privé.

## Les onglets

- **Installer** : Unharnessed, son fournisseur, le plugin, un compte.
- **Utiliser** : sa cellule Python, le modèle, les tours, le redémarrage, le serveur MCP d’Escouade.
- **Permissions** : ce qu’il fait sans demander, et pourquoi il demande Escouade 2.2.2.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le manifeste, son ACP, ses tests.
