# Goose

Ce plugin fait de Goose, l’agent open source de Block, un harness d’Escouade. Il ne contient pas de code : son manifeste décrit Goose par ACP (`goose acp`), et le plugin ACP d’Escouade (`dev.escouade.acp`) le lance, lui parle et montre sa conversation : réponses en direct, outils avec leurs diffs, cartes de permission et de question, tokens de chaque tour.

Il s’adresse à qui a déjà Goose et un fournisseur configuré.

## Ce qu’il demande

- Goose installé sur la machine (`goose --version` répond 1.57.0 ou plus), avec son fournisseur configuré (`goose configure`).
- Le plugin ACP (`dev.escouade.acp`) installé et allumé : c’est lui qui fait tourner les harness décrits.
- Un compte Goose ajouté dans Réglages › Comptes et harness.

## Ce qu’il peut faire

- Rien de lui ne s’exécute : c’est un plugin sans programme, que la fenêtre d’installation dit « Sans programme ».
- Ses agents lancent le programme `goose acp` de ta machine, avec tes droits.
- Il ne demande ni réseau, ni secret, ni fichier, ni compte à lui. Escouade ne lit aucune clé de Goose.

Goose écrit les fichiers lui-même, sans les faire passer par Escouade : dès qu’un de tes projets est privé, aucun agent Goose ne démarre.

## Les onglets

- **Installer** : Goose, le plugin, un compte.
- **Utiliser** : les modes, les discussions, les projets privés.
- **Permissions** : ce que le plugin fait faire, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le manifeste, son test.
