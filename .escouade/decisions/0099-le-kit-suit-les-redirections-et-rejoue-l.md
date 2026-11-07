---
number: 99
id: "le-kit-suit-les-redirections-et-rejoue-l"
title: "Le kit suit les redirections et rejoue les tâches réveillées : 0.1.1, tous les plugins en run.wasm sur le publié"
date: "2026-11-06"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-358"
pages: ["plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-358"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/tous-les-plugins-en-wasm","added":7,"removed":0}]
accepted_at: "2026-11-06T19:28:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le kit 0.1.0 laissait une tâche réveillée endormie sans ligne de plus, ne suivait pas les 3xx en module, et OpenCode et la voix avaient chacun leur réacteur. Les plugins le contournaient, et un tiers tombait dessus sans message.

## Décision

escouade-plugin-kit 0.1.1 : vrai waker et tour borné ; send suit les redirections comme reqwest, n'emporte hors du service que Accept, Accept-Language et User-Agent, et s'arrête si un appel a encore son corps ; http::redirect pour un client qui décide à chaque saut ; host::Host pour un plugin qui sert ses lignes lui-même. Pas d'API proc/* : seul isola s'en sert. escouade-voice 0.1.0 est publiée en MIT, avec l'accord de l'utilisateur.

## Conséquences

Jira, Linear, Trello et ClickUp suivent maintenant une redirection au lieu d'échouer. Les copies de redirect.rs, le minuteur vide d'isola, host.rs d'OpenCode et AWAITING_PUBLICATION disparaissent. Chaque module change d'empreinte : la version signée doit partir de celles du §8.29.
