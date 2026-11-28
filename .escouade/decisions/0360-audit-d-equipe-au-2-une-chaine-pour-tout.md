---
number: 360
id: "audit-d-equipe-au-2-une-chaine-pour-tout"
title: "Audit d'équipe AU-2 : une chaîne pour tout le serveur, ce que dit stream.written, request.too_large"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-609"
pages: ["serveur-equipe/audit-siem","serveur-equipe/serveur"]
tickets: ["ESC-609"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/audit-siem","added":22,"removed":6},{"page":"serveur-equipe/serveur","added":7,"removed":0}]
accepted_at: "2026-11-28T02:35:37+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

AU-2 (ESC-609) branche l'audit sur le serveur d'équipe (S1, docs/TEAM-AUDIT.md §3.1). L'étude laissait ouverts quelques points que le code impose de trancher : un serveur porte plusieurs équipes alors qu'un émetteur d'escouade-audit avait une équipe fixe ; certains refus n'ont pas d'équipe ; une entrée d'un message n'a pas d'époque au serveur ; écrire un événement par opération d'un projet ferait sortir le travail de chacun (§1.6, §6.2) ; les tailles refusées (citées au §3.1) n'avaient pas d'événement ; role.changed ne savait nommer qu'un membre, que le serveur ne connaît pas.

## Décision

1. Une seule chaîne (une clé, un émetteur `server-<16 hex>`) pour tout le serveur ; chaque événement dit son équipe (son id en hexadécimal), et `server` quand le serveur n'en connaît aucune (adresse au-delà de sa limite, client trop vieux, appareil inconnu). escouade-audit gagne `Emitter::seal_for`.
2. `stream.written` sort, avec son ancre, pour chaque entrée du flux de l'équipe et de ses flux propres sauf les totaux des coûts, et pour les commits et annulations d'un projet ; jamais pour les opérations d'un projet. L'ancre : `stream` = `team`, le nom d'un flux propre (`escouade.setting`) ou l'id du projet ; `epoch` = celle d'un commit, ou pour un message le nombre de commits de son flux ; `ct` = SHA-256 des octets chiffrés servis.
3. Nouvel événement `request.too_large` (S1, sévérité 2, la taille la plus grande de la route), ajouté au §1.2.
4. La cible de `role.changed` prend un membre (S2) ou un appareil (S1).
5. Le bruit : session.opened une fois par appareil et par adresse puis par jour ; session.refused et request.too_large une fois par minute par lieu ; rate.limited une fois par minute avec le compte depuis ; client.too_old une fois par heure ; claim.expired à la reprise d'un bail fini ; recovery.used quand le jeton de récupération sert (pas quand recovery-token l'affiche, autre processus).
6. Un jeton servi deux fois sort sous un id opaque (`t-` + 32 hex d'un SHA-256 de son hachage), jamais le jeton ni son hachage.
7. Les sorties (fichier, sortie standard, syslog) se cumulent, chacune avec son tampon ; un fil à part scelle et écrit, une requête n'attend jamais.

## Conséquences

La machine d'audit (AU-5) doit répéter la même ancre (mêmes noms de flux, même époque d'un message) pour le recoupement du §5.3, et ne peut recouper avec S1 que les entrées que S1 dit (pas les opérations des projets : ses événements de tickets se recoupent par l'id de prise et le jeton). Un SIEM filtre les événements hors équipe par `team.id = server`. Le volume reste celui du §4.3 : ni renouvellements ni opérations.
