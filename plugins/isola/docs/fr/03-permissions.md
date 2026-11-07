# Permissions d’isola

Ce que le plugin peut faire, ce qu’il ne peut pas faire, et ce qu’Escouade fait pour lui.

## Ce qu’il est

Un module WASM (`run.wasm`), compilé depuis son dossier, qui tourne isolé dans Escouade : il n’a pas de processus à lui, et n’accède ni au disque, ni au réseau, ni à tes secrets. Pour agir, il demande à Escouade de lancer un programme.

## Ce qu’il lance

Exactement trois commandes, celles de `permissions.commands` de son manifeste, montrées dans l’onglet **Permissions** de sa page :

| Commande | Quand |
| --- | --- |
| `isola ls --json` | pour lister les services du worktree, pendant un test |
| `isola down` | pour arrêter les services du worktree |
| `isola destroy` | pour détruire les services du worktree avant sa suppression |

Escouade tient ces règles, pas le plugin :

- la commande et ses arguments doivent être une de ces trois lignes, mot pour mot : `isola ls`, `isola down now` ou `isola logs` sont refusés ;
- `isola` est cherché sur le PATH, jamais un chemin ; pas de shell ; sous Windows, pas de `.cmd` ni de `.bat` ;
- le programme tourne dans le worktree qu’Escouade prête au plugin pour la durée de l’appel, et nulle part ailleurs : le plugin ne reçoit pas de chemin, seulement un nom donné pour cet appel ;
- son environnement est celui d’un agent, sans les variables propres à Escouade, plus `NO_COLOR=1` ;
- son entrée est fermée aussitôt ; il a 60 secondes, et il est tué au-delà ; il meurt avec le plugin ;
- chaque lancement est écrit au journal du plugin.

**Le programme, lui, n’est pas isolé** : `isola` tourne avec tes droits d’utilisateur. Escouade borne ce que le plugin lance, pas ce que fait isola.

## Ce qu’il ne lance pas

`isola up` n’est pas une commande du plugin : c’est Escouade qui le lance, dans un terminal du projet, après que tu as lu et approuvé le `.isola.toml`. C’est `up` qui lance les commandes que ce fichier déclare (ses services, leur `setup`).

`ls`, `down` et `destroy` ne lancent aucune commande du `.isola.toml`. C’est vrai d’isola tel qu’il est aujourd’hui, et c’est à revérifier quand isola change.

## Ce qu’il lit et écrit

- Il ne lit aucun fichier. Escouade lit le `.isola.toml` pour te le montrer, en entier (256 Kio au plus), et le compare à celui que tu as approuvé quand les services démarrent.
- Il reçoit ce que `isola ls --json` écrit, et n’en garde que les services de la branche du worktree : leur nom, leur état, leur adresse et leur port.
- Il n’écrit rien. Ce qu’écrivent les services, c’est isola qui le fait.

## Les faits et les décisions

Il ne reçoit aucun événement d’Escouade et ne décide d’aucune autorisation de tes agents.

## Les secrets

Aucun : il n’en demande pas et n’en reçoit pas.

## Ce qui part ailleurs

Rien. Le plugin n’a pas de réseau. Les services qu’isola lance sont tes programmes : ce qu’ils joignent ne dépend pas du plugin.

## Ce que tu gardes en main

- Rien ne tourne avant **Lancer**, et un `.isola.toml` modifié t’est montré de nouveau.
- Tu peux couper le plugin pour toute l’app ou pour un projet ; un projet privé ne l’a que si tu l’y actives.
- Coupé, il ne lance plus rien, et les tickets partent avec une recette sur les ports réservés.
