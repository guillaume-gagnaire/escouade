---
number: 347
id: "passer-un-agent-ts-17b-la-session-de-cla"
title: "Passer un agent, TS-17b : la session de Claude Code reprise en fork, réécrite par chaîne JSON sous trois formes, la passation au moindre échec jusqu'à la fin du premier tour"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-607"
pages: ["serveur-equipe/passer-un-agent"]
tickets: ["ESC-607"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/passer-un-agent","added":22,"removed":14}]
accepted_at: "2026-11-26T21:55:21+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La décision 274 (C16) choisit la reprise native d'une session de Claude Code passée à un collègue, avec la passation d'ESC-88 en repli. Sa construction (ESC-607) laissait des choix de détail : quelles formes du chemin réécrire, ce qui compte comme un échec, ce que devient la copie, ce que dit l'écran.

## Décision

- La session part dans le dépôt chiffré seulement pour un agent de Claude Code, entre deux tours, quand son fichier est trouvé dans le dossier de son compte ; sa version est la plus récente de ses lignes, sinon celle du `claude` du compte. Au-delà de la borne du dépôt (20 Mo), elle ne part pas. L'offre le dit (`session`).
- Chez qui reprend : le worktree se réécrit dans chaque chaîne et chaque clé de chaque ligne JSON, sous la forme de son système, sous l'autre séparateur et tel que l'écrit Git Bash (`/c/…`), sans casse sous Windows ; un dossier voisin (`dem-41-old`) n'est pas un chemin du worktree ; une ligne sans le worktree reste octet pour octet ; une ligne qui n'est pas du JSON fait refuser. La vérification relit chaînes, clés et texte brut (`\` doublé ou non).
- La copie n'écrase jamais une session du même identifiant ; un identifiant qui n'est pas celui d'une session (lettres, chiffres, `-`, `_`) fait refuser. Un process de l'agent démarré d'avance est arrêté avant le `--resume <id> --fork-session`.
- Est un échec du premier tour : une erreur (session non trouvée, API, sortie en erreur) ou un refus d'un garde-fou ; le process s'arrête, la copie est retirée, puis la passation sur une nouvelle session, sans bloquer le ticket. Un tour fini, interrompu, arrêté par la limite ou par la connexion garde la session reprise, et sa copie reste dans le dossier du compte.
- L'écran : le séparateur « Passé par Alice · reprise de la session, suite sur Claude Code » ou « Passé par Alice · passation, suite sur … » ; le premier message replié « Reprise de la session » ; chez qui passe la main, « Sa session de Claude Code part avec lui, chiffrée ; s'il ne peut pas la reprendre, il repart avec la passation. »

## Conséquences

L'agent qui reprend garde tout son contexte au lieu d'un résumé, sur Claude Code seulement ; tout doute revient à la passation d'avant. L'essai réel `real_session_copy` (claude 2.1.332) a montré la reprise en fork d'une copie réécrite. Reste l'essai à deux comptes ou organisations (a-24). Un changement du format des sessions de Claude Code se rattrape par la vérification (refus, puis passation), pas par une reprise fausse.
