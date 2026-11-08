# Antigravity CLI

Ce plugin fait d’Antigravity CLI, l’agent de Google, un harness d’Escouade. Tes agents tournent sur le `agy` de ta machine, avec ton compte Google ou une clé d’API Gemini, et leur conversation s’affiche dans Escouade : réponses en direct, outils avec leurs diffs, sous-agents, tokens de chaque tour, et les permissions et questions demandées une fois le tour fini.

Il s’adresse à qui a un compte Google pour Antigravity, ou une clé d’API Gemini.

## Ce qu’il demande

- Antigravity CLI installé sur la machine (essayé avec la 1.4.9).
- Un compte Antigravity CLI ajouté dans Réglages › Comptes et harness : la connexion Google de la machine, ou une clé d’API Gemini.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans réseau ni commande.
- Il décrit à Escouade comment lancer `agy`, puis traduit ce qu’il écrit. C’est Escouade qui lance le programme, et le relance quand il le faut, jamais le plugin.
- Il dit à Escouade quelles règles de permission écrire pour chaque agent, dans le dossier propre à l’agent, selon le mode. Hors de Bypass, elles ne laissent rien passer d’office : ce qu’`agy` refuse revient au plugin à la fin du tour, qui l’accepte, le refuse ou en fait une carte.
- Il lit, dans ce dossier propre à l’agent, les réglages, la fin du journal de la conversation et le plan, au chemin d’écriture que le modèle a donné.
- Il n’accepte seul aucune commande, `git add` et `git commit` compris, tant que l’essai réel d’`agy` n’a pas montré comment il lit une règle : chaque acceptation devient une règle « début de texte » pour la reprise du tour.
- Il reçoit le jeton du serveur MCP d’Escouade de chaque agent qu’il décrit, et le fait écrire dans ce dossier, jamais dans les arguments.

Ton `~/.gemini` n’est jamais lu ni écrit.

## Les onglets

- **Installer** : Antigravity CLI, le plugin, un compte.
- **Utiliser** : les modes, les demandes en fin de tour, la vue Antigravity CLI, les projets privés.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le dossier, les tests, le lancer seul.
