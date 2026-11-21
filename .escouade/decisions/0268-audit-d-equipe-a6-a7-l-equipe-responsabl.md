---
number: 268
id: "audit-d-equipe-a6-a7-l-equipe-responsabl"
title: "Audit d'équipe (A6, A7) : l'équipe responsable du traitement, la durée du SIEM, les adresses IP en entier et réglables"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/audit-siem"]
tickets: ["ESC-580","ESC-588","ESC-609","ESC-612","ESC-617"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-21 (ESC-580, docs/TEAM-AUDIT.md §6). L'audit est un contrôle de l'activité des salariés (RGPD, information du CSE, L. 2312-38). Le serveur s'héberge par l'équipe (§15.1 de TEAM-SERVER.md) ; le registre d'Escouade (docs/registre-rgpd.md) ne couvre que les traitements de l'éditeur. Choix A6 et A7, tranchés par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

A6-A et A7-A, les recommandations. L'équipe qui allume l'audit en est responsable du traitement ; le registre d'Escouade ne change pas (il gagnera une section de sous-traitant le jour d'un serveur hébergé par l'éditeur). Escouade ne garde que son tampon d'émission (7 jours ou 1 Gio) ; la durée est celle du SIEM, 6 mois à 1 an recommandés dans le guide, comme la recommandation de la CNIL sur la journalisation. Les adresses IP sont en entier dans les événements du serveur, réglables par --audit-ip=full|truncated|off. Chaque membre voit dans Réglages › Équipe que l'audit est allumé, ce qui sort et ce qui est parti de son poste ; allumer l'audit écrit une entrée au journal. Le guide livre un modèle de fiche de registre et l'information du CSE. Écartés : A6-B (tronquées d'office), A6-C (jamais), A7-B (une archive signée de N mois).

## Conséquences

Compris dans AU-2 (--audit-ip, le tampon), AU-4 (ce que voit chaque membre) et AU-6 (le guide, la fiche, le CSE). Pas de changement du registre d'Escouade.
