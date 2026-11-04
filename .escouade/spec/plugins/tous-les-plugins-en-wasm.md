---
title: "Tous les plugins en WASM (la cible de la 2.2)"
subject: "plugins"
sources: ["docs/PLUGINS.md §8 (ESC-330)","prototypes/wasm-all/","Décision de l'utilisateur du 2026-11-04 : « en 2.2, tous les plugins, rust ou TS, seront compilés en WASM, c'est acté »","Précision de l'utilisateur du 2026-11-04 : « tous les plugins builtins soient dans /plugins/*, tous publiés sur le store, vérifiés et signés »"]
tickets: ["ESC-330"]
decisions: [73,71,70,72]
compared: null
---

**Cette page dit la cible, pas ce qui tourne.** Rien de ce qu'elle décrit n'est encore dans l'app : ESC-330 a fait l'étude et le prototype, le portage suit le plan de `docs/PLUGINS.md` §8.10. Tant qu'il n'est pas fait, *Les plugins livrés avec Escouade* et *Les programmes JS/TS des plugins, en WASM sans Node* disent ce qui est vrai aujourd'hui.

## L'état de fin

- Chaque plugin est un module WASM, le même pour tous les systèmes, compilé par le store depuis ses sources et signé : en JS/TS par Javy (`run.js`), en Rust par `run.wasm`.
- Il ne reste aucun programme natif par système : ni `escouade-plugins`, ni `run.command`.
- La première main suit le même chemin qu'un tiers. Chaque plugin vit dans `plugins/<id-court>/` (`plugins/claude`, `plugins/codex`, `plugins/github`…), avec son manifeste, son README, sa LICENSE, son CHANGELOG et ses tests. Ses identifiants `dev.escouade.*` ne changent pas.
- Chacun passe par une étiquette `plugins/<id-court>/vX.Y.Z`, la soumission, la compilation, la revue, la signature du chef et la publication.

## Ce que l'utilisateur voit

- L'app marche dès son installation et hors ligne : elle embarque une amorce, les versions publiées et signées de sa première main, installée sans réseau par le même chemin qu'une installation du store.
- Une version du store plus récente reste en place à la mise à jour de l'app. On ne redescend jamais de version.
- Une mise à jour de la première main signée se fait sans clic si ses permissions ne grandissent pas ; sinon elle attend l'accord. Un agent en cours garde sa version jusqu'à son prochain lancement.
- Un plugin livré se coupe, mais ne se désinstalle pas.
- Avant de lancer un plugin, l'app vérifie sa signature, les empreintes de son module et de son archive, sa révocation (la liste du store, sinon celle de l'amorce) et sa compatibilité. Un `dev.escouade.*` non signé est refusé.

## Les capacités

Ce qu'un plugin fait hors du protocole se déclare au manifeste, est appliqué par Escouade et se dit dans la fenêtre d'installation : le réseau, y compris les instances que l'utilisateur donne, les flux et le WebSocket ; le serveur local d'un harness lancé par le cœur ; les fichiers de racines nommées, appelés de façon synchrone ; les commandes nommées. Les minuteurs sont un droit de base. Détail : `docs/PLUGINS.md` §8.4.

## Le modèle

Une instance par plugin, sur un fil, réveillée par les événements de l'hôte, un tour à la fois. `wasm32-wasip1`, un module cœur en réacteur : le même hôte que `run.js`.

## Les binaires natifs

Le store refuse `run.command` à la soumission et l'app ne le lance plus, quelle que soit l'origine. Aucun n'est publié aujourd'hui.
