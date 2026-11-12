---
number: null
id: "la-page-d-un-lancement-va-dans-un-volet"
title: "La page d'un lancement va dans un volet de l'écran partagé, et sa barre prend les contrôles de l'app"
date: "2026-11-12"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-419"
pages: ["fenetre/ecran-partage","kanban/page-du-lancement"]
tickets: ["ESC-419"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/ecran-partage","added":9,"removed":6},{"page":"kanban/page-du-lancement","added":11,"removed":5}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur (2026-11-12) : « Pour le mode navigateur, les boutons sont moches, corrige. Et permets de placer le navigateur à droite, comme on peut faire pour les terminaux ou d'autres agents. » La barre de la page (ESC-311) avait ses propres boutons (cadre natif visible), et la page ne s'affichait qu'en remplaçant la conversation (vue « Page ») ou dans la fenêtre « Pages ».

## Décision

1) La page est un contenu de l'écran partagé à part entière (`page`, nommé par l'agent) : à droite ou en dessous de la conversation, par « Ouvrir à côté/en dessous » de son « … », « Ouvrir la page à côté/en dessous » du menu de l'agent, ou en glissant le bouton « Page » de l'en-tête (ou son étiquette). Le volet se redimensionne et se ferme comme les autres ; la page n'est pas rechargée et garde adresse, largeur et devtools. Sa conversation montre alors ses messages ; sa vue « Page » rend actif le volet. La webview suit la zone de son volet et se masque sous un menu, un dialogue ou pendant un glisser. 2) La barre (et celle de la fenêtre « Pages ») utilise les contrôles communs : `icon-btn` pour la navigation, `btn`/`btn icon-only` pour Devtools, fenêtre à part, navigateur et « … », `segmented small` pour les largeurs, `field` pour l'adresse, le menu global de l'app pour « … », toujours présent. 3) La fenêtre « Pages » reste : y passer une page ferme son volet.

## Conséquences

Une page fermée ou passée dans la fenêtre « Pages » ferme son volet ; restée seule, c'est son agent sur la vue « Page ». Avec deux pages à l'écran, F12 et Ctrl+Maj+M vont au volet actif. Le menu « … » de la barre est désormais toujours là (« Ouvrir à côté/en dessous », ou « Fermer le volet »). Aucun changement côté Rust : la disposition reste un JSON opaque.
