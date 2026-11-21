---
number: 273
id: "overseer-d-equipe-o7-o8-ses-lancements-c"
title: "Overseer d'équipe (O7, O8) : ses lancements comptent comme le pilote auto, l'équipe le borne, le coupe-circuit l'arrête"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/overseer-equipe"]
tickets: ["ESC-580","ESC-588","ESC-546","ESC-616"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-22 (ESC-580, docs/TEAM-OVERSEER.md §8, §9). Un Overseer qui tourne la nuit sur les comptes de la machine doit rester sous les budgets de TS-16 et sous le coupe-circuit (TS-15, ESC-546), et ne doit jamais pouvoir lever ce qui l'arrête. Choix O7 et O8, tranchés par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

O7-A et O8-A, les recommandations. Ses lancements comptent comme le pilote auto : un budget franchi avec stopAutopilot les arrête (celui d'un projet dans ce projet, celui de l'équipe partout), il le dit dans le fil et le demande à l'administrateur du budget. Ses coûts sont comptés par TS-16 sous « machine de l'équipe » et « Overseer d'équipe », jamais sous un membre réel. L'équipe lui impose des clés overseer.* (sorte restreindre) : enabled, projects, maxAgents (3 par défaut), budget, model, engines, mayApprove. Le coupe-circuit « arrêter tous les agents » l'arrête avec ses agents ; une portée « Overseer d'équipe » l'arrête lui seul ; il ne peut ni le lancer ni le lever ; sans contact avec le serveur depuis 1 h, la machine ne lance plus rien ; escouade-server team overseer stop l'arrête en local ; levé, il ne relance rien seul de ce que l'arrêt a interrompu. Écartés : O7-B (les budgets de TS-16 seuls), O7-C (un budget à lui seul), O8-B (le coupe-circuit d'aujourd'hui seulement).

## Conséquences

Environ 2 h d'agent (OE-5), après le coupe-circuit d'ESC-546 : OE-5 ajoute lui-même la portée « Overseer d'équipe » au coupe-circuit (environ 30 min), puisque ESC-546 est écrit et en vérification sans elle.
