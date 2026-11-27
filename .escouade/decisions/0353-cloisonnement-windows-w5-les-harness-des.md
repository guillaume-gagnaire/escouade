---
number: 353
id: "cloisonnement-windows-w5-les-harness-des"
title: "Cloisonnement Windows, W5 : les harness des plugins tiers restent refusés tant qu'un projet privé existe, réexaminés quand la barrière est éprouvée"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-591"
pages: ["projets/cloisonnement-windows","projets/projets-prives"]
tickets: ["ESC-591","ESC-590","ESC-623"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:58:34+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-591 (docs/WINDOWS-ISOLATION.md, § 6). Depuis ESC-590 et ESC-623, un harness d'un plugin tiers n'est lancé dans aucun projet tant qu'un projet privé existe, faute de pouvoir prouver sa clôture. Lancé sous la barrière d'Escouade, sa clôture deviendrait prouvable sous Windows natif, mais pas sous macOS ni Linux.

## Décision

Choisi par l'utilisateur le 2026-11-27 (demande a-44, option a) : rien ne change pour les harness tiers avec la barrière Windows ; un ticket à part réexamine leur cas après les essais de la barrière (WB-7). Pourquoi : sous la barrière, leur clôture deviendrait prouvable sous Windows natif, mais pas sous macOS ni Linux ; la rouvrir avant que la barrière soit éprouvée ferait revenir le risque de W-04.

## Conséquences

Le refus d'ESC-590 et d'ESC-623 reste la règle sur tous les systèmes. Un ticket de plus (≈ 2 h) si l'utilisateur veut les rouvrir sous Windows.
