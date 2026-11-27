---
number: 351
id: "cloisonnement-windows-w3-quand-la-barrie"
title: "Cloisonnement Windows, W3 : quand la barrière ne peut pas tenir, l'agent n'est pas lancé, et la raison est dite"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-591"
pages: ["projets/cloisonnement-windows"]
tickets: ["ESC-591"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:58:34+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Étude ESC-591 (docs/WINDOWS-ISOLATION.md, § 4.3 et § 6). Un volume sans ACL (FAT32, exFAT, un partage réseau) ne porte ni ACE de refus ni étiquette d'intégrité ; une ACE peut être refusée (un dossier d'un autre propriétaire). L'agent peut alors être refusé, ou lancé en garde-fou.

## Décision

Choisi par l'utilisateur le 2026-11-27 (demande a-44, option a) : sous Windows natif, une fois la barrière construite, un agent clôturé dont la barrière ne peut pas être posée (volume sans ACL, ACE refusée) n'est pas lancé ; le refus dit le dossier, la raison et le geste possible (déplacer le projet sur un volume NTFS, retirer « Projet privé »). Pourquoi : un projet privé qui se croit cloisonné et ne l'est pas est pire qu'un refus ; « Barrière » doit rester vrai partout où l'interface le dit.

## Conséquences

« Barrière » reste vrai partout où l'interface le dit. Un projet privé sur une clé USB ou un partage réseau ne fait plus tourner d'agent sous Windows natif.
