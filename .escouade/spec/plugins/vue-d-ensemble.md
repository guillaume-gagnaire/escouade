---
title: "Les plugins : vue d'ensemble"
subject: "plugins"
sources: ["docs/PLUGINS.md","docs/STORE.md","docs/TODO-2.2.md","schemas/plugin-1.json","schemas/plugin-protocol-1.json","website/content/docs/fr/plugins/introduction.md"]
tickets: ["ESC-184","ESC-339"]
decisions: []
compared: null
---

# Les plugins : vue d'ensemble

Ce qui suit est la carte du sujet. La référence est `docs/PLUGINS.md`, la place de marché `docs/STORE.md`. Chaque point a sa page dans ce sujet ou dans `store/`.

## Ce qu'est un plugin

Un dossier avec un manifeste (`escouade-plugin.json`), de trois genres :

- **déclaratif** : rien ne s'exécute ; ses contributions sont lues et rendues par Escouade ;
- **programme en JS ou en TS** (`run.js`) : le store le compile en WASM, l'app le fait tourner dans son process, sans Node, le réseau et les fichiers appliqués (*Les programmes JS/TS des plugins, en WASM sans Node*) ;
- **programme natif** (`run.command`) : un process avec les droits de l'utilisateur, pour la première main et pour les binaires tiers attestés.

Tous parlent le même protocole, JSON-RPC 2.0, version 1. Aucun HTML de plugin n'entre dans la fenêtre.

## Ce qu'un plugin peut apporter aujourd'hui

- réagir aux 39 faits d'Escouade, par son programme, un webhook ou une commande déclarés ;
- décider des autorisations des agents (*Règles d'autorisation des plugins*) ;
- un thème ;
- un harness : décrit par ACP sans code (*Les harness ACP décrits*), ou, pour la première main, en process (*Le contrat des harness en process*) ;
- un genre de compte, une source du Kanban, un hébergeur de code, un service de voix, un environnement de lancement ;
- des réglages et des secrets, rendus par Escouade.

Le manifeste réserve `menus`, `statusBar`, `badges`, `projectTemplates` et `escouadeYml`, et le lecteur de workflow a son contrat, mais rien de l'app ne s'en sert encore. Les outils MCP apportés par un plugin, les textes rédigés, les langues, les statistiques, Ctrl+K, les canaux de notification déclarés et les vues libres ne sont pas construits.

## D'où ils viennent

- **Livrés avec l'app** (origine `builtin`, `plugins/<id-court>/`) : signés et mis à jour avec elle, coupables, jamais désinstallables (*Les plugins livrés avec Escouade*).
- **Du store** : store.escouade.dev, revue en local et sceau « Vérifié par Escouade », vérifié par l'app avec ses clés embarquées (*Le store dans l'app*). Les plugins de première main hors de l'app y passent comme ceux d'un tiers, signés seulement (*Les plugins d'exemple*).
- **D'un dossier ou d'un dépôt git** : « non vérifié », jamais un id `dev.escouade.*`.
- **En développement** : un dossier chargé en place, relu à chaque changement.

## Ce que garde l'utilisateur

Installé n'est pas activé : par défaut actif partout sauf dans les projets privés, où il s'active un par un. Avant l'installation, une fenêtre dit d'où il vient et ce qu'il pourra faire. Après : désactiver, retirer une permission, désinstaller, lire son journal. Une mise à jour est proposée, jamais installée seule. Un projet propose des plugins (`plugins.recommended`), il n'en installe aucun.

Le réseau déclaré est appliqué par Escouade pour un `run.js` et pour les webhooks d'un plugin déclaratif ; pour une commande, il est seulement montré, et la fenêtre d'installation le dit.

## L'essai réel

Fait le 2026-11-04 sur Windows, depuis store.escouade.dev (ESC-184) : voir `docs/PLUGINS.md` §7. macOS et Linux restent à faire à la release.
