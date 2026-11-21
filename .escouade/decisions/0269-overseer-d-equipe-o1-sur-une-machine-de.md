---
number: 269
id: "overseer-d-equipe-o1-sur-une-machine-de"
title: "Overseer d'équipe (O1) : sur une machine de l'équipe en mode serveur, sur les comptes de la machine"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/overseer-equipe"]
tickets: ["ESC-580","ESC-588","ESC-606","ESC-613"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/overseer-equipe","added":34,"removed":0}]
accepted_at: "2026-11-21T20:49:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-22 (ESC-580, docs/TEAM-OVERSEER.md §3). L'Overseer d'aujourd'hui (ESC-462, ESC-468) est celui d'une personne, sur son poste et son compte. Un Overseer qui suit les projets d'une équipe doit tourner jour et nuit, sans dépenser le quota d'une personne pour le travail de tous ; les comptes des harness ne se partagent jamais. Choix O1, tranché par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

O1-A, la recommandation. L'Overseer d'équipe tourne sur une machine de l'équipe en mode serveur (escouade-server team overseer), sous le rôle de service, sur les comptes de harness connectés à la machine (un siège d'équipe, une clé d'API), jamais le compte d'un membre ; son modèle se règle sur la machine (Haiku par défaut), imposable par overseer.model. Il voit les projets de l'équipe dont la machine a la clé, leur Kanban, leurs docs, ses propres agents, le fil de l'équipe et le résumé des coûts ; jamais les projets personnels ni privés, ni ce que fait un agent sur un poste. L'Overseer de chaque personne ne change pas. Écartés : B (le poste d'un administrateur, qui s'arrête avec lui et dépense son quota), C (pas d'Overseer d'équipe, ce qui reste le cas sans machine).

## Conséquences

Environ 3 h d'agent (OE-3), sur le mode serveur et le passage de TS-17. L'essai réel demande une machine (le VPS d'essai) et un compte de harness connecté par l'utilisateur.
