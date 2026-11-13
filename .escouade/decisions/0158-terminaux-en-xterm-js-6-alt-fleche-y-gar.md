---
number: 158
id: "terminaux-en-xterm-js-6-alt-fleche-y-gar"
title: "Terminaux en xterm.js 6 ; Alt+flèche y garde le déplacement par mot"
date: "2026-11-13"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-432"
pages: ["fenetre/raccourcis","introduction/stack"]
tickets: ["ESC-432"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/raccourcis","added":1,"removed":0},{"page":"introduction/stack","added":1,"removed":1}]
accepted_at: "2026-11-13T16:33:04+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-427 avait monté xterm.js en 6 et ses addons (fit 0.11, search 0.16, unicode11 0.9, web-links 0.12, webgl 0.19) ; trois e2e ont échoué et le chef est revenu en 5.x. xterm 6 refait son défilement (barre de VS Code) : le fond du thème est posé sur `.xterm-scrollable-element`, plus sur `.xterm-viewport`. Il retire aussi la conversion qu'il faisait d'Alt+flèche en Ctrl+flèche (xterm.js #5346), laissée aux applications qui l'embarquent.

## Décision

Les terminaux passent en xterm.js 6. Escouade garde lui-même ce que xterm.js 5 faisait d'Alt+flèche : Alt+←/→ envoie Ctrl+←/→ (Échap b / Échap f sous macOS), et hors macOS Alt+↑/↓ envoie Ctrl+↑/↓. Un autre modificateur en plus, ou un log, laisse la touche à xterm.js.

## Conséquences

Rien ne change pour l'utilisateur : le déplacement par mot reste dans PowerShell, bash et les TUI. Les vues mettent le fond du nouveau conteneur de défilement en transparent, comme elles le faisaient pour le viewport, et les e2e lisent le fond du thème sur ce conteneur et dans un pixel de ce que dessine WebGL.
