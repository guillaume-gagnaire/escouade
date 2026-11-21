---
number: 231
id: "app-mobile-m15-unifiedpush-par-un-ntfy-a"
title: "App mobile, M15 : UnifiedPush par un ntfy auto-hébergé, en option après MO-6 (proposé)"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-549"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-549"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-20T13:56:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un Android sans services Google ne reçoit pas FCM, et l'utilisateur veut tout héberger lui-même (2026-11-20). UnifiedPush laisse l'app d'Escouade se faire réveiller par l'app ntfy, branchée sur un serveur ntfy auto-hébergé (Coolify), et le cœur y envoie lui-même, sans l'éditeur ni Google. Le plugin Capacitor qui existe (capacitor-unifiedpush 0.1.2, du 2026-11-10, une seule personne, 94 téléchargements par semaine) n'est pas tenu ; la bibliothèque Android (org.unifiedpush.android:connector 3.3.6, 2026-11-18) l'est (docs/MOBILE.md M15).

## Décision

À trancher par l'utilisateur. Proposé : oui, en option, après MO-6 (MO-9, environ 3 h d'agent) : un plugin à nous autour de la bibliothèque d'UnifiedPush (Kotlin, de l'ordre de 200 lignes), l'envoi scellé du cœur vers l'adresse du téléphone, le choix dans les réglages de l'app (« Notifications par : le relais d'Escouade · UnifiedPush ») ; l'utilisateur installe ntfy sur son Coolify et l'app ntfy sur son Android. Pas pour la première sortie : il faut une seconde app et un serveur, il sert une minorité d'Android, et la batterie est à mesurer. Sous iOS, UnifiedPush n'existe pas. Autres options : dès la première sortie ; jamais.

## Conséquences

La seule voie de push sans Google ni l'éditeur sous Android ; un plugin natif de plus à tenir ; un ntfy dans les essais.
