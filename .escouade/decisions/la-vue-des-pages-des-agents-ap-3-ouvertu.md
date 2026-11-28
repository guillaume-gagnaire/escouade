---
number: null
id: "la-vue-des-pages-des-agents-ap-3-ouvertu"
title: "La vue des pages des agents (AP-3) : ouverture à côté sans voler la conversation, une bande de pilotage commune, une carte par appel"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-675"
pages: ["harness/pages-des-agents"]
tickets: ["ESC-675"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/pages-des-agents","added":21,"removed":13}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AP-3 (ESC-675) construit la vue de la direction A choisie le 2026-11-25 (a-36) : la ligne page:, l'ouverture à côté de la conversation, la barre, la bande de pilotage commune aux pages et aux onglets, la carte dans la conversation. Il restait à trancher comment l'agent apprend où sa page est allée (show_page rendait toujours opened: waiting), ce que la barre d'une copie offre, ce que devient la version regardée, et la forme de la carte dans ses états.

## Décision

1) La fenêtre décide d'ouvrir la page à côté (l'utilisateur regarde la conversation de l'agent, ou un onglet ou une page que cet agent pilote, et l'agent n'a pas demandé quiet) ; la conversation reste le volet actif. show_page attend la fenêtre 1,5 s au plus et rend opened: beside si la page est à l'écran, waiting sinon. 2) La barre d'une page n'a ni adresse, ni retour, ni rechargement (c'est une copie) : titre, agent, ticket, modèle, « v2 ▾ » (les versions gardées), les trois largeurs des navigateurs (gardées le temps du lancement, pas par nom comme un lancement) et « … ». Regarder une version plus ancienne est dit par une bande ; la version suivante de l'agent ramène à la dernière. 3) La bande de pilotage est un composant à part (DriveBand) pour les pages et les onglets ; sur une page elle porte toujours la mention de confiance, et AB-8 y branche le pilotage. 4) Chaque appel show_page réussi devient une carte (un appel refusé garde sa rangée), avec ses états : ouverture, montrée ou mise à jour, à côté ou en attente, « vN depuis », fermée (« Rouvrir » rouvre sa dernière version à côté de la conversation), plus gardée, coupée. 5) Le nom montré de l'agent est son nom d'aujourd'hui, sinon celui qu'il avait. 6) Un volet par agent (règle d'ESC-678, étendue aux pages) : la page d'un agent est pilotée par lui (driver) ; en s'ouvrant à côté, elle prend le volet d'un onglet ou d'une autre page que cet agent pilotait, et un onglet qu'il ouvre ensuite prend le sien.

## Conséquences

Un agent sait si l'utilisateur voit déjà sa page (et peut la piloter, AB-7) ou si elle attend. Cinq commandes du poste s'ajoutent (agentpage_rows, agentpage_info, agentpage_reopen, agentpage_version, agentpage_close_row), du poste seulement en 2.3 (AB-R). AB-8 réutilise DriveBand sans la redessiner ; AP-4 ajoute à la carte le pied de la demande et l'étiquette « choix » à la ligne.
