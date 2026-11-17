---
number: 200
id: "la-decouverte-des-fonctionnalites-une-ca"
title: "La découverte des fonctionnalités : une carte du cycle, cochée par l'état réel, trois bulles au bon moment"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-506"
pages: ["fenetre/a-decouvrir","fenetre/premier-lancement"]
tickets: ["ESC-506"]
documents: ["docs/ONBOARDING.md"]
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/a-decouvrir","added":64,"removed":3},{"page":"fenetre/premier-lancement","added":2,"removed":2}]
accepted_at: "2026-11-17T17:12:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Après le premier lancement (ON-1), l'utilisateur doit découvrir ce qu'Escouade sait faire (la voix, le Kanban et le pilote auto, les Docs, les PR, les terminaux, l'éditeur, plusieurs agents et comptes, les plugins, le serveur MCP, l'équipe) sans être noyé ni relancé. La direction C « La ligne » a été choisie le 2026-11-17 (docs/ONBOARDING.md, choix O1, O2 et O11 par défaut).

## Décision

Aide › À découvrir est une page de l'app : la ligne du premier lancement se prolonge en cinq voies (Planifier, Coder, Relire, Livrer, Piloter), une station par découverte, avec sa phrase, ce qu'elle demande avant (poids, quota, jeton, réseau) et « Essayer », qui ouvre le vrai écran et pose une bulle sur le vrai contrôle sans rien faire à la place de l'utilisateur. Une découverte se coche par l'état réel ou le moment que le cœur voit (les mêmes que les statistiques comptent), jamais par un clic sur sa carte ; la fenêtre ne dit que ce qu'elle seule voit (Docs, un fichier dans l'éditeur, une PR ouverte). Le pilote auto, allumé par défaut, est découvert quand il a lancé un ticket lui-même. Ce qui n'est pas possible ici (pas de micro, pas de serveur d'équipe, un réglage imposé) reste grisé avec pourquoi, sans compter ; ce que la version n'a pas (Escouade à distance) ne se montre pas. Écarter ne la propose plus jamais (« Remettre » sur la carte) ; « Tout masquer » coupe bulles et point. Trois bulles au bon moment seulement (micro survolé, premier ticket créé, PR ouverte), une par session, jamais deux fois, jamais pendant la frappe ni une permission. L'état vit dans state.json avec le premier lancement. La bande « Nouveau dans la 2.3 » se montre une fois à un poste réglé avant la 2.3 et ouvre la carte filtrée sur le nouveau. Accès : Aide (avec un point tant qu'il reste à découvrir) et Réglages › Général.

## Conséquences

Nouvelles commandes servies discover_view et discover_act (list.rs, docs/REMOTE.md). Les gestes qui comptent passent par Core::track, qui coche la découverte puis compte l'événement. Une bulle ne gêne jamais : elle laisse passer les clics hors de ses boutons et se ferme à tout appui ailleurs, à la frappe ou à l'appui sur son contrôle ; celle du micro attend un survol posé. Les événements de statistiques de la découverte restent à ON-4 (ESC-507), avec la page de confidentialité. Escouade à distance deviendra une découverte quand le mode serveur aura son réglage (REMOTE_IN_VERSION).
