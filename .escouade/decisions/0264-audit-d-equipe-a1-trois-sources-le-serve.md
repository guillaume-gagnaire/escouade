---
number: 264
id: "audit-d-equipe-a1-trois-sources-le-serve"
title: "Audit d'équipe (A1) : trois sources, le serveur, une machine qui déchiffre et les postes scellés pour une clé d'audit"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/audit-siem"]
tickets: ["ESC-580","ESC-588","ESC-608","ESC-609","ESC-610","ESC-612","ESC-614"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/audit-siem","added":33,"removed":0}]
accepted_at: "2026-11-21T20:49:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-21 (ESC-580, docs/TEAM-AUDIT.md §3). Le serveur d'équipe est aveugle (C10-A) : il ne voit que des métadonnées et des ids opaques, et les commits MLS partent en messages privés. Ce qu'un RSSI veut suivre (qui est entré, qui a changé une règle, qui a autorisé quoi à un agent) est en partie dans le journal chiffré, en partie seulement sur les postes. Choix A1, tranché par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

A1-C, la recommandation. L'audit a trois sources : S1, le serveur seul (ses routes : inscriptions, sessions, révocations, rôles du serveur, commits, droits des projets, prises, limites ; en ids opaques, signé par une clé d'audit du serveur tirée à init) ; S2, une machine de l'équipe qui déchiffre le journal des membres, le fichier d'équipe, le coupe-circuit, le store privé et les flux des projets qu'elle tient, et nomme les faits ; S3, les faits qui n'arrivent que sur un poste (refus du cœur, autorisations données, commandes approuvées, plugins), scellés par chaque poste pour une clé d'audit HPKE à part (comme la clé des coûts) et déposés dans un flux escouade/audit, que la machine ouvre et émet. Un poste ne parle jamais au SIEM, et rien ne sort d'un poste en clair. En vagues, S1 d'abord. Écartés : A (S1 seul, tout en ids opaques), B (S1 et S2, sans les faits des postes).

## Conséquences

Environ 12 h d'agent (AU-1 à AU-5) ; une clé d'audit du serveur et une clé d'audit HPKE de l'équipe à tenir ; les faits des postes attendent hors ligne dans une file. Un poste modifié peut taire ses faits : l'audit des postes ne prouve pas leur absence. Le serveur reste aveugle. Les tickets AU-1 à AU-6 sont écrits par ESC-588.
