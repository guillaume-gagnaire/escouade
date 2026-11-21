---
number: 275
id: "un-role-de-service-pour-les-machines-de"
title: "Un rôle de service pour les machines de l'équipe (A2, O2) : elles lisent et travaillent, ne signent rien d'administratif"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/audit-siem","serveur-equipe/overseer-equipe"]
tickets: ["ESC-580","ESC-588","ESC-606","ESC-614"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-21 et TS-22 (ESC-580, docs/TEAM-AUDIT.md §3.2, docs/TEAM-OVERSEER.md §3.2). La machine d'audit et l'Overseer d'équipe tournent jour et nuit sur une machine en mode serveur. Le registre n'a aujourd'hui que Admin et Member, et une machine y entre comme l'appareil d'un membre. Une machine d'administrateur exposée, pilotée par un modèle qui lit des textes de tiers, tiendrait une clé qui signe des membres et des réglages. Choix A2 et O2, tranchés par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

A2-A et O2-A, les recommandations. Un rôle de service, nouveau, au registre de l'équipe : une machine inscrite comme machine de l'équipe (rattachée à l'équipe, pas à un membre), qui lit et écrit le contenu des projets qu'on lui ouvre, prend des tickets et émet l'audit, mais ne signe jamais de membre, de rôle, de réglage d'équipe, de coupe-circuit ni de plugin privé, et ne répond pas aux invitations ; le serveur la refuse sur ses routes d'administrateur. Pour l'audit, un poste d'administrateur reste un recours sans machine (A2-B en recours). Écartés : B (une machine d'administrateur, qui pourrait aussi répondre aux invitations la nuit) et C (la machine d'un membre, dont le départ l'emporte et dont les coûts sont les siens).

## Conséquences

Environ 1 h 30 d'agent (OE-1, partagé avec AU-5, qui s'en sert). Les règles du groupe MLS (policy.rs) changent : un point de plus pour l'audit externe de la cryptographie (TS-20). Les invitations de nuit, si on les veut, passent par une autre machine d'administrateur, sans Overseer.
