---
number: 280
id: "fonctionnalites-l-assistant-fait-le-choi"
title: "Fonctionnalités : l'assistant fait le choix (NEW_INSTALL coupé), station après le premier agent, « Passer » ne change rien"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-559"
pages: ["fenetre/fonctionnalites","fenetre/premier-lancement"]
tickets: ["ESC-559","ESC-550"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/fonctionnalites","added":30,"removed":8},{"page":"fenetre/premier-lancement","added":11,"removed":0}]
accepted_at: "2026-11-22T05:10:14+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

F-1 (ESC-550, décision 226) laissait toutes les fonctionnalités allumées sur une installation neuve « jusqu'à ce que l'assistant de F-2 fasse le choix », en prévoyant que seule la constante NEW_INSTALL changerait. F-2 (ESC-559) apporte cet assistant (docs/FEATURES.md § 1, décidé avec l'utilisateur le 2026-10-28 : une fonctionnalité par écran, « Passer » ne garde que l'essentiel, ce qu'on utilise reste allumé après une mise à jour). Ses maquettes datent d'avant la direction C de l'onboarding : il fallait lui trouver sa place dans la ligne du premier lancement, dire comment il s'ouvre à la mise à jour, et ce que devient la valeur d'une installation neuve.

## Décision

1. NEW_INSTALL passe de true à false : une installation neuve a toutes les fonctionnalités coupées, l'essentiel seul ; une installation mise à jour garde allumé ce qu'elle utilise (le relevé de F-1, inchangé) et coupe le reste. Rien d'autre ne change dans le cœur pour cette valeur. 2. L'assistant est la 7e station des 8 de la ligne du premier lancement, après le premier agent et avant les statistiques : le premier agent (le « aha ») n'arrive pas plus tard, l'utilisateur choisit ce que l'app montre après l'avoir vue au travail, les statistiques restent la dernière question. 3. À la première fenêtre de la version qui apporte le choix, sur une installation mise à jour, l'assistant s'ouvre de lui-même (état featuresDue dans state.json), jamais sur le premier lancement ni pour un client distant ; fini ou passé, plus jamais de lui-même. 4. Ses interrupteurs sont ceux des réglages, enregistrés au geste ; « Passer : garder l'essentiel » n'en change aucun, ce qui, avec NEW_INSTALL coupé, ne garde que l'essentiel et ce qui est utilisé. 5. Ce qui est utilisé = le relevé de la mise à jour plus ce qui l'est quand l'assistant s'ouvre ; une source de tickets connectée compte pour le Kanban. 6. Choisir une source de tickets ou la voix à « Ce que tu veux brancher » allume le Kanban ou la Voix (le geste vaut choix). 7. « Revoir la présentation » dans Réglages › Fonctionnalités ferme les réglages et rouvre l'assistant. 8. La fenêtre des nouveautés (mise à jour installée) donne son écran et son interrupteur à une fonctionnalité qu'une mise à jour apporte et que l'installation n'a pas rencontrée (featuresKnown) ; elle y arrive allumée. 9. Les captures de l'assistant sont refaites par un script sur la vraie app (tests/site-shots/features-tour.spec.ts), commitées dans public/features/.

## Conséquences

Qui installe Escouade ne voit que l'essentiel tant qu'il n'allume rien : la station le lui présente fonctionnalité par fonctionnalité, et « Tout passer » du premier lancement laisse l'app réduite à l'essentiel (Réglages › Fonctionnalités rallume tout). Qui met à jour vers la 2.3 perd de vue ce qu'il n'utilisait pas (le coût par exemple, faute de budget ou d'économies affichées), jusqu'à son geste dans la fenêtre qui s'ouvre d'elle-même. Un serveur (escouade-server) neuf n'a pas d'assistant : ses fonctionnalités s'allument dans Réglages › Fonctionnalités depuis la fenêtre qui s'y connecte. Les tests (cœur, e2e) partent de réglages qui ont tout allumé pour ne pas dépendre de l'assistant. Le parcours compte 8 stations, environ 3 minutes ; l'entonnoir a une valeur de plus (step: features). Coût : 36 captures JPEG (≈ 1 à 2 Mo) dans l'app.
