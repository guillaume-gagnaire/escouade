---
number: 3
title: "Les plugins décident des autorisations avant l'utilisateur (PLG-8)"
date: "2026-10-30"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-180"
pages: ["plugins/regles-d-autorisation"]
tickets: ["ESC-180"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/regles-d-autorisation","added":17,"removed":0}]
accepted_at: "2026-10-30T01:45:30+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PLG-8 : les plugins de `permissions.decide` doivent répondre aux demandes d'autorisation avant qu'elles n'attendent l'utilisateur, dans un délai court, un refus l'emportant (décision 6 de l'utilisateur du 2026-10-25). Il restait à fixer le délai, ce qui va aux plugins, ce que voit l'utilisateur pendant la décision et le cas d'un plugin pas encore démarré.

## Décision

Toute nouvelle demande d'autorisation (pas un plan, pas une question) va en même temps à chaque plugin de `permissions.decide` allumé pour le projet, 500 ms chacun (3 s une seule fois pour un plugin pas encore démarré ; Escouade les démarre avec lui). Un refus l'emporte, puis une autorisation ; sinon l'utilisateur. La carte apparaît tout de suite mais la notification est retenue jusqu'à la fin de la décision ; une demande tranchée dit « Autorisé/Refusé par <plugin> via Escouade ». Chaque décision va au journal du plugin (type « Décision »). Les outils sont nommés comme Claude Code les nomme, avec une famille `kind` (command, edit, read, fetch, mcp, tool).

## Conséquences

Moins d'interruptions sans mode Bypass ; un plugin qui autorise tout reste visible (permission rouge, journal, carte signée). La carte peut s'afficher une fraction de seconde avant d'être tranchée. Rien dans `.escouade.yml` ne donne cette permission.
