---
number: 283
id: "la-coque-de-l-app-mobile-l-interface-par"
title: "La coque de l'app mobile : l'interface partagée par alias, une connexion par espace, des projets natifs fermés"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-569"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-569"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":17,"removed":5}]
accepted_at: "2026-11-22T10:15:38+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

MO-1 (ESC-569) construit la coque de l'app mobile en Capacitor 8, en direction B (M1), sur l'interface Svelte d'Escouade « partagée, jamais copiée ». Trois choix techniques n'étaient pas tranchés par la conception (docs/MOBILE.md) : comment la page du téléphone reprend src/ sans le copier ni tirer tout le bureau ; comment elle tient plusieurs espaces sans le cœur local du bureau (spaces.svelte.ts lit les autres espaces par server_link, une commande du poste) ; et comment l'éprouver contre un vrai escouade-server tant que l'appairage et le tunnel du téléphone (MO-4) manquent, le cœur refusant sur la boucle locale toute origine de page web autre que sa fenêtre (MO-2 n'admet celle de la WebView qu'à travers le tunnel).

## Décision

1. apps/mobile importe src/ là où il est, par l'alias $shared (Vite, Vitest, tsconfig) : jetons, icônes, textes (une zone « mobile » du catalogue commun), la plaque d'un espace, transport.ts. Ce qui n'existe que sur le téléphone vit dans apps/mobile/src. plateOf sort de servers.svelte.ts vers src/lib/plate.ts (réexporté) pour que la plaque ne tire pas l'état du bureau.
2. Le téléphone a ses propres espaces (apps/mobile/src/lib/phone-spaces.svelte.ts) : un RemoteTransport par espace appairé, son coup d'œil (space_glance) toutes les 3 s, fermés en arrière-plan, rouverts au retour, le dernier coup d'œil gardé affiché. Une commande desk est refusée sur le téléphone. Jusqu'à MO-4, un espace n'est donné qu'à la page des tests (window.__ESCOUADE_SPACES__) ou du développement (VITE_ESCOUADE_SPACES).
3. Le cœur n'est pas changé par MO-1 : l'origine de la WebView (https://localhost, capacitor://localhost) n'est admise qu'à travers le tunnel du téléphone (MO-2, PHONE_ORIGINS). Les e2e et `npm run dev:server` passent par un relais local qui tient la place du tunnel et retire l'origine de la page de navigateur, comme un client sans origine ; le jeton d'appareil est demandé comme toujours.
4. Les projets natifs : R8 et shrinkResources en release, aucune sauvegarde des données (allowBackup, règles d'extraction), HTTP en clair vers 127.0.0.1 seul, portrait, iPhone seul (M10), Capacitor par SwiftPM ; identifiant dev.escouade.mobile. Le retour du système : la feuille, puis « À toi », puis l'app en arrière-plan.

## Conséquences

MO-4 ajoute l'appairage et le stockage des espaces au Keystore et au Trousseau à la place des deux sources de test, et branche le plugin du Rust (le proxy du tunnel donne l'URL de chaque espace). Les e2e de MO-1 n'éprouvent pas la garde d'origine du tunnel, que les tests de MO-2 tiennent. La page du téléphone embarque les catalogues de textes complets du bureau (environ 800 Ko de JS, lus depuis le disque). La CI a deux jobs de plus (Linux pour l'APK et l'AAB, un Mac pour le simulateur) quand apps/mobile ou src/ change.
