---
number: 380
id: "la-publication-de-l-app-mobile-un-workfl"
title: "La publication de l'app mobile : un workflow lancé à la main, sous l'environnement mobile-release, qui n'envoie qu'au test fermé et à TestFlight"
date: "2026-11-28"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-576"
pages: ["a-distance/app-mobile"]
tickets: ["ESC-576"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/app-mobile","added":14,"removed":0}]
accepted_at: "2026-11-28T18:45:16+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

MO-8 (ESC-576) prépare la publication de l'app mobile sans compte ni clé : l'utilisateur a ses comptes d'entreprise (vérifiés D-U-N-S) chez Google et Apple et suit un guide qui lui fait poser des secrets aux noms fixés. Il fallait dire comment la CI signe et envoie, ce qu'elle n'envoie jamais, et comment répondre aux questions des stores (sécurité des données, confidentialité, chiffrement à l'export, règle 4.2).

## Décision

Un workflow « Mobile release » lancé à la main seulement (entrées : la plateforme, android par défaut ; « construire seulement », cochée par défaut), chaque job sous l'environnement GitHub mobile-release, seul détenteur des secrets, lus par leur nom ; un secret manquant arrête le job avant toute construction en le nommant. fastlane envoie l'AAB, signé par la clé de téléversement, au test fermé (alpha) de Google Play, et l'IPA, signée dans un trousseau jetable retiré même en échec, à TestFlight sans attendre son traitement. Jamais de revue d'Apple, de testeurs externes, de production ni d'envoi des fiches : ce sont les gestes de l'utilisateur. Les versions viennent du numéro du run. Les fiches (fr, en) et les captures (tirées de l'app sur des données fictives) vivent dans apps/mobile/store ; la politique de confidentialité de l'app est sur le site (/mobile/privacy/). ITSAppUsesNonExemptEncryption reste hors de l'Info.plist jusqu'au choix de l'utilisateur, la voie recommandée déclarant des algorithmes standards et la déclaration de l'ANSSI pour la France.

## Conséquences

L'utilisateur pose les secrets, envoie le premier AAB à la main, lance et approuve chaque publication, tranche le chiffrement à l'export et soumet à Apple ; un agent ne publie rien. Le guide (docs/MOBILE-PUBLISH.md) et le workflow partagent les mêmes noms, tenus par un test. Hors de ce workflow, l'AAB reste non signé. Tant que le chiffrement n'est pas tranché, App Store Connect pose la question à chaque build.
