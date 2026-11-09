# ACP

Ce plugin fait tourner dans Escouade les harness qui parlent ACP (Agent Client Protocol) et qu’un autre plugin décrit, sans code. Goose en est un. Escouade lance l’agent, le plugin ACP lui parle et montre sa conversation : réponses en direct, outils avec leurs diffs, cartes de permission et de question, tokens de chaque tour.

Il s’adresse à qui veut utiliser un harness décrit, comme Goose, et à l’auteur qui veut en décrire un.

## Ce qu’il demande

- Rien à installer de plus que le harness décrit lui-même et le plugin qui le décrit.
- Un compte du harness décrit, dans Réglages › Comptes et harness.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans réseau ni commande.
- Il décrit à Escouade comment lancer le harness, tel que son manifeste le dit, puis traduit ce qu’il écrit. C’est Escouade qui lance le programme, jamais le plugin.
- Il lit, pour l’agent, dans le dossier de l’agent et son dépôt, et écrit dans le dossier de l’agent, 4 Mio par fichier au plus, quand le harness passe ses fichiers par Escouade. L’hôte juge chaque chemin et refuse ceux d’un projet clôturé.
- Il accepte seul les écritures dans le dossier de l’agent en Édits auto, et tout en Bypass.
- Il reçoit le jeton du serveur MCP d’Escouade de chaque agent qu’il décrit, pour le donner à la session du harness, jamais dans les arguments.

Coupé ou désinstallé, les agents des harness décrits restent, marqués « harness absent », avec leurs conversations, et reviennent avec lui.

## Les onglets

- **Installer** : le plugin, un harness décrit, un compte.
- **Utiliser** : les modes, les discussions, les projets privés, et comment décrire un harness.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le crate, les tests, le lancer seul.
