---
number: 274
id: "passer-un-agent-c16-la-reprise-native-de"
title: "Passer un agent (C16) : la reprise native de la session copiée pour Claude Code, gardée par des contrôles, la passation d'ESC-88 en repli et pour les autres harness"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/passer-un-agent"]
tickets: ["ESC-548","ESC-581","ESC-588","ESC-607"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/passer-un-agent","added":9,"removed":1}]
accepted_at: "2026-11-21T20:49:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-17 (ESC-548) a livré le passage d'un agent en C16-A : chez qui le prend, une nouvelle session dont le premier message est le dossier de passation d'ESC-88. Le §15.2 de docs/TEAM-SERVER.md disait « A, puis B pour Claude Code » si l'essai TS-0c montrait la reprise fiable. L'essai (ESC-581, docs/TEAM-SESSION-COPY.md, 2026-11-21) : une session de Claude Code copiée, ses chemins réécrits par chaîne JSON, se reprend par --resume dans un autre dossier, avec son contexte, ses outils et le bon cwd ; Codex retrouve sa copie mais sa réponse n'a pas pu être obtenue. Choix C16, tranché par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » L'option « a » est la recommandation du §4 de docs/TEAM-SESSION-COPY.md. Reprise par ESC-588.

## Décision

C16-B pour Claude Code, C16-A pour les autres harness. Quand l'agent passé est sur Claude Code, entre deux tours, et que le collègue le prend avec Claude Code : la session (son transcript_path) part dans le dépôt chiffré du passage avec le chemin du worktree et la version de claude ; chez qui reprend, la reprise native n'est tentée que si sa version de claude est égale ou plus récente, que la réécriture du worktree de qui passe la main vers le sien se fait par chaîne d'objet JSON (les deux séparateurs) et qu'il ne reste ensuite aucune occurrence de l'ancien chemin ; la copie est déposée dans projects/<worktree encodé>/<id>.jsonl du dossier de configuration du compte choisi, et l'agent est lancé par --resume <id> --fork-session (un nouvel identifiant, jamais deux histoires sous le même). À tout échec (version plus ancienne, chemin restant, session non trouvée, erreur au premier tour), la passation d'ESC-88, comme aujourd'hui. La copie est effacée du serveur une fois reprise, comme le journal. La fenêtre du passage dit « Reprise de la session » ou « Passation ». Codex, Copilot, Cursor, OpenCode et les autres restent en C16-A ; Codex se reprendra peut-être si son essai se finit (§2 de l'étude). Écarté : C16-A seul pour tous, qui fait repartir l'agent de Claude Code avec un résumé.

## Conséquences

Un ticket, TS-17b (environ 2 h d'agent), et un essai à deux comptes ou deux organisations de Claude Code, à l'utilisateur (ce que font session-owner et hybrid-organization à la reprise, et les blocs de réflexion signés). La session copiée porte plus que le journal (le prompt système, les identifiants du compte et de l'organisation, ce que les outils ont lu) : elle part chiffrée de bout en bout, ne va que dans le dossier du compte de qui reprend, et s'efface du serveur. Le format de session de Claude Code change sans prévenir : les contrôles et le repli le gardent.
