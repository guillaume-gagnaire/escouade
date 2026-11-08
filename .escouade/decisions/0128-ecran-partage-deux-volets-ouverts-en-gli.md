---
number: 128
id: "ecran-partage-deux-volets-ouverts-en-gli"
title: "Écran partagé : jusqu'à quatre volets, ouverts en glissant depuis la barre latérale (direction B)"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-392"
pages: ["fenetre/ecran-partage","fenetre/raccourcis"]
tickets: ["ESC-392"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/ecran-partage","added":46,"removed":17},{"page":"fenetre/raccourcis","added":4,"removed":2}]
accepted_at: "2026-11-08T21:51:14+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un utilisateur veut voir un agent et un terminal, ou deux agents, côte à côte, en glissant un agent ou un terminal depuis la barre latérale. La zone de travail ne montrait qu'un contenu à la fois (drapeaux par projet qui s'excluent dans state.svelte.ts, chaîne de {#if} dans App.svelte) ; notifications « vues », focus du composer, dictée et réponses au clavier supposaient un seul agent à l'écran. L'étude ESC-392 (docs/SPLIT.md) a rendu trois directions en aperçu (design/split-view/) ; l'utilisateur a choisi B le 2026-11-08, voulu plus de deux volets, l'éditeur en plus des agents, terminaux et lancements, et la disposition retenue par projet, puis demandé la construction pour la 2.2 beta.

## Décision

La zone de travail d'un projet se partage en volets, côte à côte ou l'un sur l'autre, quatre au plus (un arbre de séparations binaires, lib/panes.ts), chaque volet gardant 320 px de large ou 200 px de haut. Un volet prend un agent, un terminal, le journal d'un lancement ou l'éditeur ; le Kanban, les PR et les Docs prennent toute la zone sans défaire la disposition. On ouvre à côté en glissant depuis la barre latérale (ou le bouton Éditeur) : une boussole de cinq cases (haut, gauche, droite, bas, milieu = remplacer), cases barrées sans la place, Échap annule ; aussi par le menu contextuel (« Ouvrir à côté », « Ouvrir en dessous »), Ctrl+clic, et au clavier (Ctrl+\ partager ou revenir à un volet, Ctrl+Maj+\ tourner, Ctrl+Alt+flèches changer de volet, Ctrl+Maj+W fermer), qui passent aussi par les terminaux. Chaque volet est un panneau sur le cadre ; l'actif porte un anneau de la couleur du projet et reçoit la saisie, la dictée, les réponses au clavier, Ctrl+Tab, le tiroir et la barre de statut ; tout agent montré est vu. La disposition est retenue par projet (ui.panes) et revient au redémarrage, sans ce qui n'existe plus (terminaux compris).

## Conséquences

app.agent, app.term, app.runCommand restent « le contenu du volet actif », ce qui garde le reste du code tel quel ; agentShown suit le volet actif et shownAgents liste les agents vus. Le nom « split » restant au « Côte à côte » existant (ui.layout), le code dit « panes » ; dans un volet, le tiroir des fichiers s'ouvre par-dessus. Une vue dans un volet inactif ne prend pas le focus (mountTerminal, Composer). Sans écran partagé, rien ne change. Restent pour plus tard : la boussole pilotée au clavier depuis une carte, le Kanban, les PR et les Docs dans un volet, le téléphone (qui ne montrera que le volet actif).
