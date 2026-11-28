---
number: 354
id: "cloisonnement-windows-w6-escouade-active"
title: "Cloisonnement Windows, W6 : Escouade active le bac à sable de Claude Code pour un agent clôturé de WSL"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-591"
pages: ["projets/cloisonnement-windows","harness/claude-code-wsl"]
tickets: ["ESC-591"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:58:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-591 (docs/WINDOWS-ISOLATION.md, § 4.6). Un Claude Code de WSL reçoit sandbox.filesystem.denyRead et denyWrite, qui ne valent que si l'utilisateur a activé son bac à sable. L'interop de WSL (lancer cmd.exe depuis Linux) pourrait ouvrir un processus Windows hors du bac à sable : à essayer.

## Décision

Choisi par l'utilisateur le 2026-11-27 (demande a-44, option a) : pour un agent clôturé dont le Claude Code tourne dans WSL, Escouade active son bac à sable dans le fichier de réglages de l'agent, et vérifie que l'interop ne le contourne pas ; « Ses fichiers » le dit. Pourquoi : c'est la barrière qui existe déjà ; sans elle, le denyRead d'un agent de WSL ne vaut rien.

## Conséquences

1 ticket (WB-8), ≈ 2 h, indépendant de W1. Le bac à sable filtre aussi le réseau par domaine : ce qu'il coupe pour un agent est à dire.
