---
number: null
id: "une-commande-de-lancement-ouvre-son-adre"
title: "Une commande de lancement ouvre son adresse quand elle répond, par l'attente de « ▶ Tester »"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-560"
pages: ["fenetre/barre-laterale","kanban/page-du-lancement","projets/escouade-yml"]
tickets: ["ESC-560"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/barre-laterale","added":10,"removed":0},{"page":"kanban/page-du-lancement","added":5,"removed":0},{"page":"projets/escouade-yml","added":11,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur veut configurer une URL à ouvrir automatiquement au lancement d'une commande de lancement (2026-11-20). Les lancements de test savaient déjà attendre qu'une adresse réponde et l'ouvrir dans une page de lancement, mais une page était toujours celle d'un agent ; une commande lancée dans le projet n'en a pas. Il fallait aussi qu'une adresse suive les ports d'un worktree, sans syntaxe existante pour cela.

## Décision

Chaque commande a une adresse facultative (http ou https, vérifiée à la saisie et par le cœur) et « Où l'ouvrir » : page de lancement (par défaut) ou navigateur. Elle s'ouvre quand l'adresse répond, par le même code d'attente que « ▶ Tester » (waitUntilUp : 500 ms, 2 s par requête, 3 min au plus), jamais après un délai fixe ; une commande qui s'arrête d'elle-même avant n'ouvre rien et son log le dit. Dans un worktree, la page est celle de l'agent ; dans le projet, un onglet par commande de la fenêtre « Pages » du projet (clé run:<projet>:<commande>, sans « Remettre dans l'agent »), rechargé plutôt que doublé. L'adresse nomme les ports comme un shell : ${ESCOUADE_PORT_BASE}, ${ESCOUADE_PORT_END}, avec une valeur pour le projet après :- ; aucune autre variable. .escouade.yml la porte par url et open_in, approuvées avec la commande.

## Conséquences

RunCommand gagne url et openIn (défauts : vide, page) ; devpage_open prend un projet facultatif pour une page sans agent ; le schéma de .escouade.yml décrit url et open_in. L'API distante (port relayé d'ESC-538) n'est pas sur la 2.2 : à reprendre au reversement dans release/2.3.
