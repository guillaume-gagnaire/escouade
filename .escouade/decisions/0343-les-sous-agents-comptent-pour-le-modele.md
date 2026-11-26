---
number: 343
id: "les-sous-agents-comptent-pour-le-modele"
title: "Les sous-agents comptent pour le modèle sur lequel ils ont tourné"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-662"
pages: ["fenetre/stats"]
tickets: ["ESC-662"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/stats","added":8,"removed":0}]
accepted_at: "2026-11-26T17:59:10+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-662 : un sous-agent de Copilot CLI (`task`) tourne souvent sur un autre modèle que son agent (dans le flux réel `full-turn.jsonl`, `mai-code-1.1-flash` pour un agent sur `gpt-6-luna`). Son usage entrait bien une fois dans le tour, mais tout le tour était rangé sous le modèle de l'agent. Même chose chez Codex (où le coût estimé prenait en plus le tarif du modèle de l'agent) et chez OpenCode (un sous-agent d'`opencode.json` peut prendre un autre fournisseur). Chez Copilot, l'usage d'un sous-agent dont le plugin n'avait pas vu le départ était perdu.

## Décision

Le protocole des plugins gagne l'élément `turnModels` (protocole 0.1.7 dans le cœur), envoyé juste avant `turn` : la part de chaque modèle dans le tour, dont la somme est l'usage du tour. Le cœur écrit une ligne de stats par modèle, à l'horodatage du tour (un seul tour dans « Par compte » et « Par harness ») ; sans `turnModels` (un plugin plus ancien), rien ne change. Copilot CLI 2.2.9, Codex 2.2.6 et OpenCode 2.2.6 l'envoient, en restant sur la crate du protocole publiée (0.1.2) : ils écrivent l'élément en JSON à leur sortie (`turnmodels.rs`), le type n'étant qu'au cœur tant que la crate n'est pas publiée. Copilot compte aussi l'usage d'un sous-agent inconnu de sa session. Le compte reste celui de l'agent : un sous-agent tourne dans son process.

## Conséquences

Les stats par modèle et le coût estimé de Codex deviennent justes pour les sous-agents ; les totaux ne changent pas. Un élément nouveau plutôt qu'un champ de `turn` : un cœur plus ancien l'ignore (`Unknown`), les plugins plus anciens restent tels quels. On ne publie pas la crate du protocole pendant la 2.3 : les trois plugins se construisent sur la 0.1.2 publiée, et pourront prendre le type quand la crate sera publiée. Ils sont à revoir, signer et publier ; l'amorce suit. Les mêmes écarts existent sur la ligne 2.2 (main) : un correctif 2.2.x suivra.
