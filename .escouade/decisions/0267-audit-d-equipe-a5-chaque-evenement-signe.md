---
number: 267
id: "audit-d-equipe-a5-chaque-evenement-signe"
title: "Audit d'équipe (A5) : chaque événement signé, chaîné, ancré au journal du serveur, sa tête publiée dans le flux de l'équipe"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/audit-siem"]
tickets: ["ESC-580","ESC-588","ESC-608","ESC-609","ESC-614"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-21 (ESC-580, docs/TEAM-AUDIT.md §5). Un SIEM garde ce qu'il reçoit mais ne vérifie pas de signature ; il faut pouvoir montrer qu'un événement vient d'Escouade, n'a été ni changé ni retiré, et correspond à une entrée réelle du journal signé (le groupe MLS, C9-B). Choix A5, tranché par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

A5-B, la recommandation. Quatre liens : la signature Ed25519 de l'émetteur sur le JSON canonique (RFC 8785) ; la chaîne de l'émetteur (seq, empreinte du précédent) ; l'ancre à l'entrée du serveur d'où vient l'événement (le flux, le numéro, l'époque, l'empreinte SHA-256 du chiffré), que le serveur émet aussi, ce qui permet de recouper le serveur et la machine ; un point de contrôle horaire de la tête, écrit dans la sortie et dans le flux de l'équipe, signé comme une balise de tête. Un outil escouade-audit verify vérifie un export (signatures, chaînes, points de contrôle, recoupement) ; le guide donne les règles de recoupement en SPL, KQL, ES|QL et Datadog. Écartés : A (sans la tête publiée ni l'outil), C (TLS seul).

## Conséquences

Environ 2 h 30 d'agent (AU-1, AU-5). Ne prouve ni le contenu d'un commit MLS face à un vérificateur sans clés, ni ce qu'un poste piraté tait ; une machine d'audit prise se trahit au recoupement pour tout ce qui passe par le serveur.
