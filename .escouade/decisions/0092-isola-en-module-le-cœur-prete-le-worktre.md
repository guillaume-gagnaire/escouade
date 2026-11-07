---
number: 92
id: "isola-en-module-le-cœur-prete-le-worktre"
title: "isola en module : le cœur prête le worktree, et un projet privé doit l'activer"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-351"
pages: ["plugins/tous-les-plugins-en-wasm","plugins/livres-avec-escouade","kanban/lancement-de-test"]
tickets: ["ESC-351"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":8,"removed":0},{"page":"plugins/livres-avec-escouade","added":2,"removed":0},{"page":"kanban/lancement-de-test","added":4,"removed":0}]
accepted_at: "2026-11-06T00:14:48+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

isola est le premier plugin de la première main porté sur proc/*. `dir` de proc/spawn n'est jamais un chemin, or launch/* nomme un worktree et isola n'est pas un harness : il n'a pas d'agent. D'autre part, Launch::decls ignorait la confidentialité : en process, isola atteignait tout projet, alors que le §3.3 et le §8.4.7 de docs/PLUGINS.md disent qu'un projet privé refuse un plugin non activé. Enfin, le cœur cherchait isola par which::find (qui prend un .cmd), et l'hôte par locate (qui le refuse).

## Décision

1. Chaque appel launch/* prête le worktree au plugin le temps de l'appel, sous un nom de l'appel donné dans `agent`, et le retire après : pas de nouvelle racine, pas de chemin dans les mains du module. Le prêt ne porte pas de projet : le cœur a jugé le projet dans Launch::for_dir, et un projet qu'on ferme doit encore voir ses services s'arrêter.
2. Un environnement de lancement suit la règle de tout plugin : dans un projet privé, il n'est actif qu'une fois activé là.
3. Le cœur cherche le programme d'un environnement comme l'hôte (locate) : ce que le plugin ne peut pas lancer n'est pas installé.
4. Le kit 0.1.0 ne change pas : le plugin gère lui-même proc/out et proc/exit, et demande un minuteur vide après proc/exit pour réveiller l'appel qui attend, le réacteur pollant les anciennes tâches d'abord.

## Conséquences

Un projet privé qui utilisait isola doit l'activer dans Réglages › Plugins ; sans cela, son ticket part comme sans isola, sur une recette et les ports réservés. Sous Windows, un isola en .cmd ne compte plus comme installé. Un plugin qui n'est pas un harness et qui veut joindre un worktree a maintenant un chemin : le prêt du cœur. Un changement du kit n'atteint pas ce module avant une publication et un nouveau plugin-lock.sh, ce qui explique le contournement du point 4 ; une prochaine version du kit pourra le rendre inutile. Le vrai isola n'a pas été joué avec le module (pas de release Windows) : à faire sous Linux ou macOS avant Z1.
