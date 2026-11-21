---
title: "L'audit vers un SIEM"
subject: "serveur-equipe"
sources: ["docs/TEAM-AUDIT.md (ESC-580, TS-21 : l'étude, les choix A1 à A8, le découpage AU-1 à AU-6)","Réponse de l'utilisateur du 2026-11-21 (demande a-23) : toutes les recommandations, A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A","docs/TEAM-SERVER.md §3.7, §3.8, §6, §9, §12, §13, §15","crates/escouade-team-server/README.md (Ce que le serveur voit et garde)","crates/escouade-team-crypto/src/beacon.rs (les balises de tête)","crates/escouade-team/README.md (Les coûts de l'équipe : la clé des administrateurs)","docs/registre-rgpd.md"]
tickets: ["ESC-580","ESC-588","ESC-608","ESC-609","ESC-610","ESC-612","ESC-614","ESC-617","ESC-606"]
decisions: [264,275,265,266,267,268]
compared: null
---

# L'audit vers un SIEM

> **Décidé, pas encore construit.** L'utilisateur a pris toutes les recommandations de l'étude d'ESC-580 (`docs/TEAM-AUDIT.md`) le 2026-11-21 (A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A). Cette page dit ce qui est décidé ; chaque partie se construit par un ticket, de AU-1 (ESC-608) à AU-6 (ESC-617). Une partie construite le dira ici.

## Ce que c'est

Un journal d'audit de l'équipe, envoyé à son outil de sécurité (Splunk, Elastic, Microsoft Sentinel, Datadog…). Il est éteint d'office ; l'équipe l'allume, sur son serveur et sur une machine de l'équipe.

## Ce qui sort, ce qui ne sort jamais (A3-B, A8-A)

- **Sort** (une liste fermée d'événements, chacun avec un nom stable et une sévérité de 1 à 5) : les membres, les appareils, les rôles, les époques et les invitations ; les accès au serveur ; les réglages imposés, les signataires et les accords des postes ; le coupe-circuit ; le store privé et les plugins installés ; les prises et les passages de tickets ; les validations ; les autorisations données aux agents (par qui) ; les consignes de l'Overseer ; la mémoire validée ou mise en quarantaine ; les budgets franchis ; les refus du cœur.
- **Les noms** : les événements nomment les membres, les appareils et les projets, et donnent les clés des tickets, **jamais leurs titres**, descriptions ou critères.
- **Ne sort jamais** : le contenu chiffré (titres, descriptions, spec, décisions, documents, souvenirs), les secrets (jetons, clés, mots de secours, secret d'une invitation, code d'appairage), le texte des conversations, des prompts, des sorties, des diffs et des messages de commit, les noms de fichiers, le code, les comptes des harness. Une commande autorisée sort par **son programme et son empreinte**, jamais son texte. Ni chaque tour d'agent, ni les renouvellements de sessions et de baux.
- Chaque événement est une structure fermée, et un test cherche des données piégées dans tout ce qui sort (AU-1, ESC-608).

## Les sources (A1-C, A2-A)

- **Le serveur seul** (AU-2, ESC-609) : ce que voient ses routes, en ids opaques, signé par une clé d'audit du serveur tirée à `init`.
- **Une machine de l'équipe qui déchiffre** (AU-5, ESC-614), sous le **rôle de service** (OE-1, ESC-606 : elle lit, ne signe rien d'administratif) ; un poste d'administrateur en recours, tant qu'il est ouvert. Elle nomme ce qui se passe dans le journal et les projets qu'elle tient, et dit ceux qu'elle ne suit pas.
- **Les postes** (AU-3, ESC-610 ; AU-4, ESC-612) : ce qui n'arrive que là (refus du cœur, autorisations, commandes approuvées, plugins, coupe-circuit appliqué), scellé pour une clé d'audit à part et déposé sur le serveur ; la machine l'ouvre. Un poste ne parle jamais au SIEM, et rien ne sort d'un poste en clair. Un poste modifié peut taire ses faits.

## Les formats (A4-A)

Un modèle JSON unique (`escouade.audit/v1`), écrit en JSON lines (fichier tourné ou sortie standard) et en syslog RFC 5424 sur TLS, CEF en option ; OpenTelemetry par un Collector qui lit le fichier ; pas de connecteur propre à un SIEM au départ. Options `--audit-file`, `--audit-stdout`, `--audit-syslog`, `--audit-cef` sur le serveur et la machine ; un tampon sur disque si le SIEM ne répond pas (7 jours ou 1 Gio, puis `audit.dropped`).

## L'intégrité (A5-B)

Chaque événement est signé par son émetteur, chaîné au précédent, et ancré à l'entrée du journal du serveur d'où il vient (son numéro, son époque, l'empreinte de son chiffré). La tête de la chaîne est publiée chaque heure dans le flux de l'équipe, comme les balises de tête. `escouade-audit verify` vérifie un export et recoupe ce que dit le serveur avec ce que dit la machine.

## La rétention et le RGPD (A6-A, A7-A)

- L'équipe qui allume l'audit est responsable du traitement : elle héberge son serveur. Le registre d'Escouade ne change pas, sauf un jour pour un serveur hébergé par l'éditeur.
- Escouade ne garde qu'un tampon d'émission (7 jours ou 1 Gio) ; la durée est celle du SIEM (de 6 mois à 1 an recommandé). Les adresses IP sont en entier dans les événements du serveur, réglables (`--audit-ip=full|truncated|off`).
- Allumer l'audit écrit une entrée au journal. Chaque membre voit dans Réglages › Équipe que l'audit est allumé, et ce qui part de son poste.
- Le guide du site (AU-6, ESC-617) donne le modèle de fiche de registre et l'information du CSE.

## Les tickets

| Ticket | Ce qu'il construit | Après |
| --- | --- | --- |
| AU-1 (ESC-608) | la crate `escouade-audit` : le modèle, les événements, ce qui ne sort jamais, les écritures, la signature, `verify` | ESC-588 |
| AU-2 (ESC-609) | le serveur émet seul | AU-1 |
| AU-3 (ESC-610) | la clé d'audit et le flux des postes | AU-1 |
| AU-4 (ESC-612) | les faits des postes, ce que voit le membre | AU-3, ESC-546, ESC-579 |
| AU-5 (ESC-614) | la machine d'audit, les noms, le recoupement | AU-2, AU-4, OE-1, ESC-546, ESC-579 |
| AU-6 (ESC-617) | les guides du site, l'essai contre Splunk et Elastic | AU-5 |

## Ce qui attend l'utilisateur

L'essai contre Sentinel et Datadog (un abonnement Azure et un compte Datadog), et le numéro d'entreprise IANA (PEN) des données structurées de syslog (`32473`, le numéro de documentation, d'ici là).
