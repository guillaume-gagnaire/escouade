---
number: 319
id: "pages-des-agents-p4-p5-une-cloison-sans"
title: "Pages des agents P4, P5 : une cloison sans réseau, scripts permis sans eval"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-661"
pages: ["harness/pages-des-agents"]
tickets: ["ESC-661"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-26T00:19:17+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-661 : un HTML écrit par un agent (ou par ce qu'il a lu) n'est pas digne de confiance. Il faut décider ce qu'une page libre peut exécuter et joindre.

## Décision

Choisi par l'utilisateur le 2026-11-25 (demande a-36, recommandations acceptées). P4 : les scripts de la page sont permis dans la cloison (`script-src 'self' 'unsafe-inline'`), sans `unsafe-eval` ; écartés : HTML assaini sans script, scripts avec eval. P5 : aucun réseau, sans exception (`default-src 'none'`, `connect-src 'self'` vers son seul schéma) ; écartée : une liste d'origines par projet. La cloison : webview `agentpage-*` refusée par `refuses`, schéma `escouade-page://` servi à sa seule webview depuis la copie, CSP en en-tête et en meta, navigation seulement par une bande d'Escouade, incognito, permissions et téléchargements refusés, préchargements DNS et RTCPeerConnection retirés, ce qu'Escouade exécute dans un monde isolé.

## Conséquences

Une page ne charge rien du web (bibliothèques à embarquer dans ses fichiers). Sonde agentpage_probe en CI et essais réels sous macOS et Linux (docs/AGENT-PAGES.md § 8.4). AP-1.
