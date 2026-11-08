---
number: 118
id: "reglages-la-barre-des-categories-la-tein"
title: "Réglages : la barre des catégories, la teinte par portée et « Avancé »"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-379"
pages: ["fenetre/reglages"]
tickets: ["ESC-379"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":45,"removed":39}]
accepted_at: "2026-11-08T02:07:46+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La fenêtre des réglages de la 2.0 rangeait 13 onglets en trois groupes et choisissait le projet par des pastilles au-dessus de certains onglets : on ne savait pas toujours si un réglage valait pour l'app ou pour un projet, et le rare encombrait les pages. L'étude ESC-375 a proposé trois directions ; l'utilisateur a choisi la direction A le 2026-11-07. R1 (ESC-378) a posé le registre des pages ; R2 pose la coque.

## Décision

La fenêtre des réglages (1120 × 780, ou la place disponible) a une barre des catégories à gauche : la recherche (inerte jusqu'à R8), les pages d'Escouade, « Projets » avec le projet choisi déplié sur ses sept pages, « Tous les projets », puis « À propos » et la version en bas. Les flèches, Origine et Fin parcourent la barre ; passer sur un projet l'ouvre. La fenêtre prend le teal d'Escouade sur les pages de l'app et la couleur du projet sur les siennes. « Avancé » est un groupe replié en bas d'une page, qui nomme ce qu'il cache, lu dans le registre. Le pied « Annuler / Enregistrer » et le point de changement par page restent. Jusqu'à R3–R7, chaque page montre le contenu de l'ancien onglet qui la recevra.

## Conséquences

Aucune clé enregistrée ne change. Les renvois de l'app passent encore par les anciens ids d'onglet (legacyTab) jusqu'à R6 ; les projets fermés vivent sur « Tous les projets », et leur retour y mène. Le point d'une page suit le registre : tant que les pages montrent les anciens onglets, un réglage peut marquer une page qui ne l'affiche pas encore. Les e2e ouvrent une page par openSettingsPage() (tests/e2e/fixture.ts) ; settings-look.spec.ts compare la vraie fenêtre aux aperçus design/settings-2-2/index.html#a-*, dans les deux thèmes, à 1440 × 900 et 1028 × 779.
