---
number: 272
id: "overseer-d-equipe-o6-un-seul-par-equipe"
title: "Overseer d'équipe (O6) : un seul par équipe, par un bail d'équipe au serveur ; la prise atomique pour le reste"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/overseer-equipe"]
tickets: ["ESC-580","ESC-588","ESC-611","ESC-618"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-22 (ESC-580, docs/TEAM-OVERSEER.md §7). Sur un Kanban d'équipe, il y a l'Overseer personnel de chaque membre, le pilote auto de chacun, et peut-être deux machines. La prise atomique (TS-6) empêche deux lancements d'un même ticket, mais pas deux Overseers d'équipe qui répondent au même message ou se contredisent. Choix O6, tranché par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

O6-A, la recommandation. Un bail d'équipe au serveur (/v1/team/leases/overseer), comme une prise de ticket : 10 min renouvelées toutes les 3 min, un jeton qui croît à chaque prise et que portent ses écritures (refusées sous un jeton dépassé). Une seule machine fait tourner l'Overseer d'équipe, les autres attendent et le disent ; celle qui reprend le bail relit le fil depuis le dernier message traité et ne relance rien qui tourne ailleurs. Pour le reste, la prise atomique tranche (un ticket, un preneur) ; l'Overseer personnel d'un membre agit au nom de son membre, voit l'Overseer d'équipe dans le fil, et aucun Overseer ne reçoit d'ordres d'un autre. Écartés : B (un Overseer par projet, plus tard par le même bail nommé par projet), C (aucun garde).

## Conséquences

Environ 1 h 30 d'agent (OE-2) et 2 h pour la cohabitation et l'e2e (OE-6). Le serveur ne voit du bail qu'un nom, une machine et une heure. Les tâches programmées de l'équipe, tenues par la machine du bail, sont une suite possible (environ 2 h), hors de ce découpage.
