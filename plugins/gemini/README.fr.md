# Gemini CLI

Ce plugin fait de Gemini CLI, l’agent de Google, un harness d’Escouade, par ACP. Tes agents tournent sur le `gemini` de ta machine, avec ton compte Google ou une clé d’API Gemini, et leur conversation s’affiche dans Escouade : réponses et raisonnement en direct, outils avec leurs diffs, cartes de permission, de question et de plan, tokens de chaque tour.

Il s’adresse à qui a un compte Google ou une clé d’API Gemini.

Gemini CLI n’a pas encore été essayé avec un vrai compte : le plugin suit sa documentation et ce que son programme fait contre un faux serveur.

## Ce qu’il demande

- Gemini CLI installé sur la machine (`npm install -g @google/gemini-cli`, Node 20 ou plus ; essayé avec la 0.66.0).
- Un compte Gemini CLI ajouté dans Réglages › Comptes et harness : un compte Google connecté par `gemini`, ou une clé d’API.

## Ce qu’il peut faire

- Il tourne dans Escouade, isolé : c’est un module WASM, sans réseau ni commande.
- Il décrit à Escouade comment lancer `gemini --acp`, puis traduit ce que Gemini écrit. C’est Escouade qui lance le programme, jamais le plugin.
- Il fait approuver le dossier de l’agent par Gemini (`GEMINI_CLI_TRUST_WORKSPACE`) : sans cela, Gemini ne lance pas le serveur MCP d’Escouade. Gemini exécute alors les hooks et lit les réglages du dépôt ouvert dans ce dossier.
- Il accepte seul les écritures de Gemini dans le dossier de l’agent (hors `.git`), et tout en Bypass ; il refuse aussitôt une demande qui nomme un chemin d’un autre projet quand l’un des deux est privé.
- Il lit, dans le dossier propre à l’agent, le plan que Gemini a écrit.
- Il reçoit le jeton du serveur MCP d’Escouade de chaque agent qu’il décrit, pour le donner à la session de Gemini, jamais dans les arguments.

## Les onglets

- **Installer** : Gemini CLI, le plugin, un compte.
- **Utiliser** : les modes, la vue Gemini CLI, les projets privés.
- **Permissions** : ce que le plugin lit, reçoit et décide, mode par mode.
- **Dépannage** : les messages et quoi faire.
- **Développer** : le dossier, les tests, le lancer seul.
