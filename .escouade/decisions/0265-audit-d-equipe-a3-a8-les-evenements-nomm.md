---
number: 265
id: "audit-d-equipe-a3-a8-les-evenements-nomm"
title: "Audit d'équipe (A3, A8) : les événements nomment qui et où, jamais quoi ; une commande autorisée par son programme et son empreinte"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/audit-siem"]
tickets: ["ESC-580","ESC-588","ESC-608","ESC-612","ESC-614"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-21 (ESC-580, docs/TEAM-AUDIT.md §1, §2, §7). Un événement d'audit sans noms est illisible pour un RSSI ; avec les titres des tickets, le contenu chiffré sortirait vers un outil tiers. Le texte d'une commande autorisée porte souvent un secret. Choix A3 et A8, tranchés par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

A3-B et A8-A, les recommandations. Les événements nomment les membres, les appareils et les projets (nom court) et donnent les clés des tickets, jamais leurs titres, descriptions ou critères. Une liste écrite en dur de ce qui ne sort jamais : le contenu chiffré, les secrets (jetons, clés, mots de secours, graine du séquestre, secret d'invitation, code d'appairage), le texte des conversations, prompts, sorties, diffs et messages de commit, les noms de fichiers, le code, les comptes des harness. Une commande autorisée sort par son programme et son empreinte, jamais son texte. Chaque événement est une structure fermée (deny_unknown_fields), les chaînes passent par les motifs de jetons de la mémoire d'équipe, et un test cherche des données piégées dans tout ce qui sort. Écartés : A3-A (ids opaques seuls), A3-C (les titres aussi), A8-B (le texte masqué par motifs), A8-C (rien).

## Conséquences

Compris dans AU-1 (la liste fermée et son test) et AU-5 (les noms). Les noms ne viennent que de la machine qui déchiffre ; le serveur seul reste en ids opaques.
