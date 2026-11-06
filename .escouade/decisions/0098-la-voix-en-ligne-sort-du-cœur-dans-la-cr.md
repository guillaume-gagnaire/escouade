---
number: 98
id: "la-voix-en-ligne-sort-du-cœur-dans-la-cr"
title: "La voix en ligne sort du cœur dans la crate escouade-voice, sans repli sur un service du cœur"
date: "2026-11-06"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-350"
pages: ["plugins/tous-les-plugins-en-wasm","plugins/mistral-vibe-en-plugin","plugins/livres-avec-escouade","voix/dictee"]
tickets: ["ESC-350"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/mistral-vibe-en-plugin","added":3,"removed":3}]
accepted_at: "2026-11-06T17:31:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les cinq services de voix étaient des plugins depuis INT-13, mais leur code restait dans le crate de l'app et `escouade-plugins` le liait. P6 les porte en run.wasm. Un module ne compile qu'à partir de son dossier et de crates publiées, et le kit 0.1.0 n'a ni flux, ni socket, ni tâche qui survit à son appel. Le cœur gardait aussi un `CoreService` qui répondait quand un plugin ne démarrait pas : ce code ne peut plus y rester si la voix sort du cœur.

## Décision

Le code des services va dans une nouvelle crate publiée, `escouade-voice` 0.1.0, en MIT comme les crates du kit, sur un réacteur à elle tiré de celui d'OpenCode, augmenté de `ws/*`. Le kit ne change pas. L'app lie cette crate pour le protocole et les tables des services, mais n'exécute plus aucune requête vers un service. `CoreService` est retiré : un plugin qui ne démarre pas le dit à son premier appel. Un service ne démarre son module qu'à son premier appel, le registre le décrivant depuis la crate. `voice/transcribe` porte le son en PCM 16 bits (`pcm16`), et `ws/open` rend le corps d'un refus, sans les valeurs des en-têtes.

## Conséquences

Le chef publie une quatrième crate, après le kit ; d'ici là les cinq plugins n'ont ni Cargo.lock ni empreinte, et restent dans AWAITING_PUBLICATION. Une release sortie avant l'amorce n'a plus de voix en ligne du tout, alors qu'elle gardait le repli du cœur : Z2 passe avant la prochaine release. Limites nouvelles par rapport au natif : 120 s par requête, 4 flux en direct par plugin, une dictée annulée va à son terme chez le service. Le code de `host.rs` existe en deux copies (OpenCode, voix) jusqu'à ce qu'ESC-358 en décide. Le lancement de l'app ne compile plus aucun module de voix.
