---
number: null
id: "les-corrections-de-l-audit-dans-la-2-2-v"
title: "Les corrections de l'audit dans la 2.2 : variables et mode Bypass appliqués aux chemins propres à la 2.2"
date: "2026-11-12"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-428"
pages: ["harness/comptes","plugins/tous-les-plugins-en-wasm","plugins/harness-acp-decrits"]
tickets: ["ESC-428","ESC-424","ESC-425"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":4,"removed":1},{"page":"plugins/tous-les-plugins-en-wasm","added":2,"removed":2},{"page":"plugins/harness-acp-decrits","added":2,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur présente la 2.2 finale à l'équipe sécurité de sa mission. Les corrections de l'audit (ESC-420 à 426) ont été faites sur main, qui porte la 2.3 (ESC-359 : terminal et comptes lancés par les plugins). Dans la 2.2, c'est encore le cœur qui écrit les arguments de la vue terminal, qui lit les comptes, et qui embarque des plugins de première main d'avant permissions.env.

## Décision

1. La 2.2 garde la règle de main pour la première main : un plugin dev.escouade.* sans permissions.env garde ses lancements comme avant. Aucune liste de variables n'est ajoutée. Dès que le seed de la 2.2 porte une version qui déclare ses variables, ce plugin y est tenu. 2. Les variables jugées en 2.2 sont celles de proc/spawn, d'engine/launch, de la relance, de terminal/launch, d'engine/oneShot, et aussi de l'attach que rend agent/open (le terminal qui rejoint un process partagé). account/read n'existe pas en 2.2. 3. Sans « Autoriser le mode Bypass », --allow-dangerously-skip-permissions et --dangerously-skip-permissions sont retirés des arguments de vue terminal que le cœur écrit (tui.rs) et du programme suivant d'un agent (start_again), en plus du lancement décrit par un plugin.

## Conséquences

Les plugins embarqués par la 2.2 marchent sans changement jusqu'à la mise à jour du seed par le chef. L'attach d'agent/open et la relance sont jugés en 2.2 mais pas encore sur main : un ticket pour main est à prévoir.
