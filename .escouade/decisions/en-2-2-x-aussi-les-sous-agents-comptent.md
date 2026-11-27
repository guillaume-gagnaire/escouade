---
number: null
id: "en-2-2-x-aussi-les-sous-agents-comptent"
title: "En 2.2.x aussi, les sous-agents comptent une fois, pour le modèle sur lequel ils ont tourné"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-691"
pages: ["fenetre/stats"]
tickets: ["ESC-691","ESC-662"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/stats","added":8,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-662 a trouvé en 2.3 des écarts présents aussi sur la ligne 2.2 (main) : chez Copilot CLI, l'usage d'un sous-agent dont le plugin n'avait pas vu le départ était perdu ; chez Copilot CLI, Codex et OpenCode, ce que consomme un sous-agent sur un autre modèle était rangé sous le modèle de l'agent (et, chez Codex, estimé au tarif de ce modèle). ESC-691 porte la correction sur main pour une 2.2.x.

## Décision

Le cœur de la 2.2 lit l'élément `turnModels` (prochain protocole, dans le cœur seulement) envoyé juste avant `turn` : la part de chaque modèle dans le tour, dont la somme est l'usage du tour. Il écrit une ligne de stats par modèle, un seul tour dans « Par compte » et « Par harness » ; sans `turnModels` (un plugin plus ancien), le tour reste sur le modèle de l'agent. Copilot CLI 2.2.10 (qui garde les protocoles 3 et 4 de Copilot CLI), Codex 2.2.6 et OpenCode 2.2.6 l'envoient en restant sur la crate publiée du protocole : ils écrivent l'élément en JSON à leur sortie (`turnmodels.rs`). Copilot CLI compte aussi les appels d'un sous-agent dont il n'a pas vu le départ.

## Conséquences

Les stats par modèle et le coût estimé de Codex deviennent justes pour les sous-agents ; les totaux de l'agent ne changent pas, sauf chez Copilot CLI où l'usage autrefois perdu s'y ajoute. Un cœur 2.2 plus ancien ignore l'élément. La crate du protocole n'est pas publiée ni montée de version sur main ; les trois plugins sont à revoir, signer et publier au store. Le test de parité des plugins de harness retire `turnModels` de la comparaison avec les anciens natifs, en vérifiant que sa somme vaut l'usage du tour.
