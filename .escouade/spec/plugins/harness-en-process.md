---
title: "Le contrat des harness en process"
subject: "plugins"
sources: ["docs/PLUGINS.md §4.5 et §4.6","docs/TODO-2.2.md (INT-9)","crates/escouade-plugin-protocol/src/engine.rs","src-tauri/src/plugins/harness.rs","src-tauri/src/plugins/shared.rs"]
tickets: ["ESC-221","ESC-223"]
decisions: [34]
compared: null
---

# Le contrat des harness en process

Le harness d'un plugin (`contributes.harnesses`) est servi par le process du plugin, comme tout plugin, de première main ou tiers. Il n'y en a encore aucun dans une release : le premier est le harness « Minimal » du plugin de démonstration, dans les builds de développement et les tests. Les harness d'aujourd'hui y passent un par un (INT-10).

## Ce qui reste au cœur

- **Le lancement.** Le plugin décrit le lancement (arguments, variables, fichiers, premières lignes) et ne lance rien. Le cœur lance le programme qu'il a trouvé lui-même, dans son Job object, sous WSL au besoin. Le harness survit à un plantage de son plugin.
- **Ce que le cœur vérifie avant de lancer**, quel que soit le plugin, sinon l'agent ne démarre pas et la raison est dite :
  - le jeton du serveur d'Escouade n'est jamais dans les arguments (« … met le jeton d'Escouade dans les arguments de son harness… ») ;
  - pour un projet privé, la clôture est appliquée et chaque chemin refusé figure dans le lancement (« … n'applique pas la clôture des projets privés… ») ;
  - une discussion de l'onglet Docs est tenue en lecture seule (« … ne tient pas son harness en lecture seule : la discussion ne démarre pas. ») ;
  - les fichiers ne sont écrits que dans le dossier de l'agent.
- **L'état de l'agent.** Le plugin traduit ce que dit le harness en éléments (texte, raisonnement, outil, demande, question, fin de tour, note), et le cœur les applique à la conversation. Ce que le plugin écrit au harness passe par le cœur.
- **Le terminal.** Les octets du PTY et les frappes ne passent jamais par le plugin : il décrit le lancement de la vue et lit ses hooks, et il met la réponse du cœur dans la forme de son harness.
- **Ce qui est déclaré** se lit sans lancer le plugin : le nom, le programme, les versions, les capacités (lecture seule, comptes par clé), les modèles, le dossier où le harness garde la mémoire d'un projet (proposée une fois à la mémoire partagée).

## Un plugin qui plante

- En plein tour : le plugin redémarre (trois fois en cinq minutes au plus) et le cœur lui redonne l'agent et ce qu'il n'avait pas traduit. Rien n'est montré deux fois. La conversation dit « Le plugin de <harness> a redémarré. ». Un outil en cours au moment du plantage est fermé à la fin du tour.
- Après le dernier plantage permis, l'agent s'arrête et la conversation dit « Le plugin de <harness> s'est arrêté après N plantages : l'agent s'arrête, sa session est gardée. Relance le plugin dans Réglages › Plugins, puis envoie un message. ».

## Les seuils

Mesuré sur le harness « Minimal », contre les chiffres d'INT-0 : au plus 5 ms de plus par ligne au rythme réel (0,66 ms au 99e centile), au moins 2 200 lignes par seconde (41 119), au plus 30 Mo par process de plugin (14 Mo). Chaque harness qui passe en process refait ces mesures.

## Un harness partagé par des agents

Un harness peut servir plusieurs agents d'un seul process (`opencode serve`, voir *OpenCode en plugin*). Le contrat le dit pour tous (`Launch::shared`) :

- **Le lancement reste au cœur.** Le plugin décrit le process et le marque partagé ; le cœur le lance une fois pour les agents du même plugin, du même programme et des mêmes variables, avec un agent par dossier si le plugin le demande, et l'arrête avec son dernier agent. Il survit donc au plantage de son plugin, comme tout harness.
- **Ce que le plugin lit seul**, le cœur ne le voit pas passer : les éléments arrivent sans être demandés (`agent/items`), dans leur ordre. Ce qu'un tel harness ne dit pas sur une ligne, la clôture et la lecture seule comprises, le plugin le tient ; seul un plugin livré avec l'app est cru sur la clôture.
- **Un plantage ou un arrêt se voit sans appel en cours** : chaque agent regarde l'état de son plugin. Les messages sont les mêmes que plus haut ; coupé pendant qu'un agent tourne, l'agent s'arrête en le disant, sa session gardée.

Une fin de tour et une question alertent comme pour les autres harness, quel que soit le plugin : c'est le cœur qui applique les éléments.
