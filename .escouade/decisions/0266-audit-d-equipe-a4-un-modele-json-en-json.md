---
number: 266
id: "audit-d-equipe-a4-un-modele-json-en-json"
title: "Audit d'équipe (A4) : un modèle JSON, en JSON lines et en syslog RFC 5424 sur TLS, CEF en option, OpenTelemetry par le Collector"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/audit-siem"]
tickets: ["ESC-580","ESC-588","ESC-608","ESC-609","ESC-617"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:52+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-21 (ESC-580, docs/TEAM-AUDIT.md §4). Les SIEM visés (Splunk, Elastic, Microsoft Sentinel, Datadog) lisent des fichiers par leurs agents et du syslog par leurs collecteurs ; OTLP monte mais demande une pile de plus ; un connecteur par SIEM met quatre jetons sur la machine. Choix A4, tranché par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

A4-A, la recommandation. Un modèle d'événement unique en JSON (schema escouade.audit/v1, une correspondance OCSF documentée, non émise), écrit en JSON lines (fichier tourné, ou sortie standard) et en syslog RFC 5424 sur TCP et TLS (RFC 5425), CEF en option de la sortie syslog ; OpenTelemetry par un Collector qui lit le fichier, documenté ; aucun connecteur propre à un SIEM au départ (Splunk HEC si on le demande) ; pas de webhook comme voie de SIEM. Options du serveur et de la machine : --audit-file, --audit-stdout, --audit-syslog, --audit-cef ; un tampon sur disque si le SIEM ne répond pas (7 jours ou 1 Gio, audit.dropped au-delà). Écartés : B (OTLP seul), C (un connecteur par SIEM).

## Conséquences

Environ 2 h 30 d'agent dans AU-1 et AU-2. Un numéro d'entreprise IANA (PEN) pour les données structurées de syslog, à demander par l'utilisateur (32473, le numéro de documentation de la RFC 5612, d'ici là). Les essais contre Sentinel et Datadog demandent un abonnement Azure et un compte Datadog, à l'utilisateur ; Splunk et Elastic s'essaient en conteneurs (AU-6).
