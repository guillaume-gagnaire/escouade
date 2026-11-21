---
number: 235
id: "app-mobile-m7-un-relais-de-notifications"
title: "App mobile, M7 : un relais de notifications auto-hébergé par l'éditeur sur son Coolify, FCM seul de Firebase, APNs pour iOS (proposé)"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-549"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-549"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":4,"removed":3}]
accepted_at: "2026-11-20T13:56:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

APNs et FCM n'acceptent un envoi que de qui détient les clés du projet de l'app : sans relais de l'éditeur, une app fermée ne reçoit rien, même en auto-hébergé. C'est la décision 6 de REMOTE.md, ouverte. L'utilisateur, le 2026-11-20 : « Firebase obligatoire ? J'aurais bien tout hosté moi-même sur Coolify. » (docs/MOBILE.md § 3.11, M7).

## Décision

À trancher par l'utilisateur. Proposé : un relais auto-hébergé par l'éditeur sur son Coolify, sous push.escouade.dev (un service à lui, son image construite par la CI) ; ses deux clés (le compte de service de FCM, la clé .p8 d'APNs) en secrets de Coolify, jamais dans le dépôt ; il ne voit qu'un réveil scellé (XChaCha20-Poly1305, la clé de l'appairage), le téléphone déchiffre ; il ne garde que l'identifiant au hasard de chaque appairage, sa plateforme et son jeton de push ; borné en débit ; refusable à l'appairage. Offert à tous, auto-hébergés compris : un utilisateur ne peut pas en monter un pour l'app des stores, seul le détenteur des clés du projet de l'app envoie à ses téléphones. De Firebase, seulement FCM (ni Analytics, ni Crashlytics, ni base, ni Hosting), l'inscription automatique du SDK coupée jusqu'à l'accord de l'utilisateur ; APNs pour iOS, sans SDK Firebase dans l'app iOS. Autres options : pas de relais ; un service de push d'un tiers (OneSignal, Expo).

## Conséquences

Un service de plus à tenir sur le Coolify de l'éditeur (MO-6) ; un projet Firebase réduit à FCM et une clé APNs, créés par l'utilisateur. Ce qui part chez Google (le FID, le jeton, l'app et sa version, les métadonnées des messages, jamais leur texte) est vérifié sur l'APK par MO-6 et dit dans la page de confidentialité. Un Android sans Google passe par UnifiedPush (M15).
