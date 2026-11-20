---
number: 228
id: "passer-un-agent-la-prise-passee-par-le-s"
title: "Passer un agent : la prise passée par le serveur, l'offre au journal, le dépôt chiffré, la machine qui prend seule"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-548"
pages: []
tickets: ["ESC-548"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-20T20:03:02+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-17 (ESC-548) : passer l'agent d'un ticket de l'équipe à un collègue ou à une machine en mode serveur, en C16-A (la passation d'ESC-88 chez qui reprend). La conception (§11 de docs/TEAM-SERVER.md) ne disait pas comment la prise change de main sans qu'un tiers la prenne entre deux, où vit la proposition, ni ce que devient le worktree de qui passe la main.

## Décision

- La prise change de main **au serveur** : `hand` la donne à un appareil du projet sous un nouveau jeton, pour un bail de 14 jours ; la colonne `handed_by` dit qui l'a passée, et seul celui-là la reprend (`withdraw`) tant qu'elle n'est pas prise. Le serveur refuse (`not_in_project`) un appareil révoqué, sans accès, ou soi-même.
- La proposition est un objet `handoff` du journal du projet (chiffré, signé) ; la conversation et le patch, un fichier du serveur scellé par la clé du projet. Les deux s'effacent à la reprise et à l'abandon. Chez qui reçoit, l'offre n'est crue qu'avec la prise que le serveur dit passée à cet appareil par celui qu'elle nomme.
- Le travail non commité part en **patch chiffré par défaut** ; le commit « en cours » est l'autre choix. Un hébergeur qui ne répond pas défait le commit : rien ne passe.
- Le worktree de qui passe la main est **détaché de la branche** (ses fichiers restent) et rattaché à l'annulation, pour que la branche se reprenne ailleurs, même plus tard sur ce poste.
- Une machine en mode serveur le dit dans chaque projet (`machine` sur l'objet `person` de son appareil) et **prend seule** ce qu'on lui passe, sur le harness de l'agent ou le premier qui sert le projet.
- Le refus par qui reçoit et un projet de plusieurs dépôts restent pour plus tard.

## Conséquences

Le passage ne dépend que du serveur d'équipe déjà là (pas de canal direct entre postes, pas d'API distante d'un espace). Un collègue qui ne prend pas laisse la prise passée 14 jours ; qui a passé la main l'annule. Le patch par défaut ne touche pas l'historique de la branche.
