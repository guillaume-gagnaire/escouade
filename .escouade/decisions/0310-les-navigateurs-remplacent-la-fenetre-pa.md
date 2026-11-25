---
number: 310
id: "les-navigateurs-remplacent-la-fenetre-pa"
title: "Les navigateurs remplacent la fenêtre Pages"
date: "2026-11-23"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-657"
pages: ["kanban/page-du-lancement","fenetre/barre-laterale","fenetre/ecran-partage","fenetre/fenetre","kanban/lancement-de-test"]
tickets: ["ESC-657","ESC-560","ESC-311","ESC-419","ESC-651"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-24T02:31:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La page d'un lancement s'ouvrait dans la vue « Page » de son agent ou dans une fenêtre « Pages » du projet, à part (ESC-311, ESC-560). L'utilisateur a demandé le 2026-11-23 « une gestion de navigateurs comme les terminaux, dans la sidebar, avec une ligne par navigateur, qu'on peut ensuite ouvrir en plein écran ou en demi écran exactement comme les agents et les terminaux » ; ses choix : un navigateur peut exister seul, une ligne = un onglet, la fenêtre « Pages » retirée, les largeurs et les devtools à côté de l'adresse, les devtools intégrés en bas ou à droite là où le système le permet.

## Décision

Une section « Navigateurs » sous « Terminaux », sur son modèle exact : une ligne par onglet (point d'état, nom, menu, croix, « + » qui demande une adresse), affichée seule ou dans un volet de l'écran partagé. Un navigateur s'ouvre seul par « + », ou de lui-même pour le test d'un agent et pour une commande de lancement qui a une adresse. La vue « Page » dans la conversation d'un agent et la fenêtre « Pages » sont retirées : son bouton « Page » affiche sa ligne, et le réglage « Dans une fenêtre à part » (ou `window`) ouvre la ligne. Les adresses suivent les règles des pages de lancement (http ou https, jamais Escouade, l'origine de départ ; le reste au navigateur du système). Les devtools ne s'ouvrent que sur un geste de l'utilisateur (le raccourci injecté dans la page, qu'un script pouvait déclencher, est retiré) et se logent dans la page là où la webview le permet sans élargir la sécurité : WebKitGTK oui (en bas, à droite par l'inspecteur) ; WebView2 non (il faudrait un port de débogage distant) et WKWebView non (attaché, l'inspecteur casse les largeurs), qui gardent leur fenêtre, ce que la vue dit. Un navigateur vit tant qu'Escouade tourne, comme un terminal. À distance, rien n'est exposé : une webview est de la fenêtre (commandes « du poste seulement »).

## Conséquences

Un seul endroit pour toutes les pages, aux gestes déjà connus des terminaux, et une page ouverte seule à côté d'un agent. On perd la fenêtre à part sur un second écran (le navigateur du système reste) et, sous macOS et Linux, F12 tapé dans la page (le clic droit « Inspecter » le remplace). Les navigateurs ne reviennent pas au redémarrage ; le téléphone ne les voit pas.
