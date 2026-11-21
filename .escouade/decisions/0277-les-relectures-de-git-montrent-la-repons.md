---
number: 277
id: "les-relectures-de-git-montrent-la-repons"
title: "Les relectures de git montrent la réponse la plus récente arrivée"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-522"
pages: ["git-fichiers/git-fichiers"]
tickets: ["ESC-522"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/git-fichiers","added":3,"removed":0}]
accepted_at: "2026-11-21T23:37:34+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'historique, les fichiers non commités et le diff d'un fichier se relisent à chaque changement du dépôt, et chaque réponse était jetée dès qu'une relecture plus récente était partie (GitGraph.svelte, FilesPanel.svelte, FileDiff.svelte). Sous charge, le dépôt change plus vite que git ne répond : l'historique restait sur « Chargement… », la liste gardait un fichier supprimé (e2e app.spec.ts:303, :84, :898, ESC-522).

## Décision

Une réponse s'affiche dès qu'elle est plus récente que celle qui est à l'écran et qu'elle vaut pour ce qui est montré (même projet, agent, dépôt, fichier) ; une réponse plus ancienne n'en remplace jamais une plus récente (src/lib/reads.ts, newestReads).

## Conséquences

Le panneau Git suit le dépôt même quand il change sans cesse, sans attendre qu'il se calme. Rien d'autre ne change : le premier affichage et les relectures gardent leurs délais.
