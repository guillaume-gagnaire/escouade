---
number: 250
id: "le-cache-des-modules-wasm-suit-la-versio"
title: "Le cache des modules WASM suit la version exacte de wasmtime, et la sortie d'un plugin est bornée par tour"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-542"
pages: ["plugins/programmes-js-ts-en-wasm"]
tickets: ["ESC-542"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/programmes-js-ts-en-wasm","added":4,"removed":0}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

RUSTSEC-2026-0566 (wasmtime-wasi : mémoire de l'hôte sans borne par des écritures vides dans un AsyncWriteStream) et RUSTSEC-2026-0567 (abort sans la fonction threads) visent wasmtime 50.0.1, l'hôte des plugins. La montée en 50.0.2 pose deux questions : un module précompilé par 50.0.1 peut-il être chargé par 50.0.2 (wasmtime ne compare que sa version majeure, et la clé du cache n'a que la version de l'app) ; et Escouade a-t-elle, dans ses propres flux, l'équivalent du défaut de wasmtime-wasi (ses sorties standard sont à elle, pas un AsyncWriteStream, mais elles s'accumulent sans borne pendant un tour).

## Décision

wasmtime et wasmtime-wasi passent en 50.0.2. Le moteur des plugins exige des modules précompilés la version exacte de wasmtime (ModuleVersionStrategy::Custom(« wasmtime x.y.z »), lue de l'épinglage par le build.rs d'escouade-plugins) : un module d'une autre version est refusé par wasmtime et précompilé à nouveau, sans erreur visible. Ce qu'un plugin écrit sur sa sortie standard ou d'erreur est borné à 256 Mio par sortie et par tour (wasm::OUTPUT_MAX, sa plus grande mémoire) : au-delà, l'instance est arrêtée avec la raison au journal et refaite avec le budget de redémarrages.

## Conséquences

Après la mise à jour, chaque module de plugin est précompilé une fois de plus à sa première activation (une fois par version de wasmtime). Un plugin qui écrirait plus de 256 Mio en un tour est arrêté : aucun plugin connu n'en approche. Monter wasmtime ne demande que l'épinglage : la clé suit.
