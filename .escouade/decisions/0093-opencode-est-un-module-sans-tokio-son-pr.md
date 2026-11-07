---
number: 93
id: "opencode-est-un-module-sans-tokio-son-pr"
title: "OpenCode est un module sans tokio : son propre réacteur sur le protocole, sa crate liée au cœur jusqu'à P8"
date: "2026-11-05"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-349"
pages: ["plugins/opencode-en-plugin","plugins/tous-les-plugins-en-wasm","plugins/livres-avec-escouade"]
tickets: ["ESC-349"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/opencode-en-plugin","added":8,"removed":1},{"page":"plugins/tous-les-plugins-en-wasm","added":4,"removed":0},{"page":"plugins/livres-avec-escouade","added":2,"removed":0}]
accepted_at: "2026-11-06T01:42:33+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

P5 (ESC-349) porte OpenCode en run.wasm. Ses trois tâches tokio (lire le flux SSE, les requêtes qui attendent, le minuteur) doivent devenir des événements de l'hôte. Le réacteur du kit ne lance une tâche que pour un appel du cœur : rien n'y lance la lecture du flux, et il n'a pas de client net/stream. Le kit 0.1.0 est publié : le changer demande une publication par le chef, sans quoi le module du store ne se compile pas. Le cœur lie par ailleurs la crate en bibliothèque (règles, traduction) jusqu'à P8.

## Décision

Le plugin a son propre réacteur (plugins/opencode/src/host.rs) sur escouade-plugin-protocol, publié, sans escouade-plugin-kit : tâches, net/fetch, net/stream et son crédit, timer/*. Le code async du client reste tel quel, sans tokio ni reqwest sur aucune cible, et se teste nativement ligne à ligne. La crate est en crate-type ["cdylib", "rlib"], membre du workspace plugins/, dépendance par chemin de src-tauri. Le manifeste déclare local:shared et limits (60 s, 16 Mio par requête).

## Conséquences

Aucune publication n'est nécessaire pour ce port. Un autre plugin à flux (la voix, P6) devra reprendre ce code ou faire monter spawn et net/stream dans le kit, avec une publication. Une session dont les messages pèsent plus de 16 Mio ne se copie plus, ce qui échoue en le disant ; 64 Mio de mémoire par instance empêchent d'aller au plafond de 32 Mio. Au-delà de 8 serveurs OpenCode lancés à la fois, le suivant n'a pas de flux. Le flux est maintenant lâché avec le dernier agent. Le crash d'une instance sans process se simule dans les tests (Runtime::crash_instance).
