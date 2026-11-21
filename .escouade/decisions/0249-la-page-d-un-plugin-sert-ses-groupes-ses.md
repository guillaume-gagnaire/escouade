---
number: 249
id: "la-page-d-un-plugin-sert-ses-groupes-ses"
title: "La page d'un plugin sert ses groupes, ses secrets sur plusieurs lignes et l'import d'un fichier, sans jamais garder le chemin"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-526"
pages: ["plugins/contrat-du-manifeste"]
tickets: ["ESC-526"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/contrat-du-manifeste","added":19,"removed":4}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Issue #40 : un plugin VPN veut ranger ses réglages, décrire ses secrets et recevoir une configuration OpenVPN entière. ESC-525 lit `groups`, `description`, `multiline`, `lines` et `import`, mais la page du plugin ne les montrait pas encore. L'étude ESC-501 (§4.1, §4.3, choix n° 6) recommande l'import du contenu plutôt qu'un chemin avec un droit de lecture.

## Décision

La page du plugin montre d'abord les groupes, dans l'ordre. Chacun a un titre, une description et ses rangées sous un filet, sans boîte, réglages et secrets mêlés dans l'ordre de `fields`. Un groupe `folded` est replié comme « Avancé ». Ce qui n'est dans aucun groupe vient après, sous « Réglages » puis « Secrets » ; un plugin sans groupes se rend comme avant. Un secret montre sa description. Un champ `multiline` est une zone numérotée à chasse fixe de `lines` lignes, 64 Kio au plus : au-delà, il est refusé avec un message, sans rien tronquer. Un secret multi-ligne gardé ne se relit jamais : l'app ne rend que son nombre de lignes et d'octets (« Gardé · N lignes · taille »), avec « Remplacer… » et « Oublier ». « Importer un fichier… » est un champ fichier de la fenêtre, filtré sur les extensions. Il lit le fichier une fois (extension vérifiée, 64 Kio, UTF-8 sans caractère nul) puis se vide aussitôt. Ni le chemin ni le nom n'atteignent l'app, ses réglages ou un journal, et le plugin ne gagne aucun droit sur les fichiers.

## Conséquences

L'import passe par un champ fichier de la WebView : le chemin reste inconnu du code de la fenêtre comme du cœur. Aucune commande Tauri ni aucun droit n'est ajouté. `plugin_secrets` rend `size` (lignes et octets) pour un secret multi-ligne gardé. La limite de 64 Kio et le masquage ligne par ligne au journal sont ceux d'ESC-527, partagés et non doublés. Avec le découpage d'ESC-527, une configuration de 6 Ko se garde depuis la page sous Windows. L'e2e `plugin-groups.spec.ts` et un Vitest tiennent le contrat.
