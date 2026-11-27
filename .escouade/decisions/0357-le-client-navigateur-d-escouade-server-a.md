---
number: 357
id: "le-client-navigateur-d-escouade-server-a"
title: "Le client navigateur d'escouade-server : à faire, éteint par défaut, allumé par --browser"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-655"
pages: ["a-distance/mode-serveur","a-distance/api-distante"]
tickets: ["ESC-655"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-27T23:59:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'étude ESC-655 (docs/BROWSER-CLIENT-STUDY.md), faite d'après le mode serveur du fork d'OshOEz, propose que escouade-server serve l'interface dans un navigateur. Ce client sert qui ne peut pas installer Escouade : un poste de travail verrouillé, une tablette, un Chromebook. En échange, il ajoute au serveur une surface : une page servie et une origine web acceptée par l'API distante. Question B1 (a-45) : le faire, et comment l'allumer.

## Décision

Le client navigateur est fait (tickets BC-1 à BC-4 et BC-7 à BC-9 de l'étude). Il est éteint par défaut. On l'allume par escouade-server --browser, par la variable ESCOUADE_SERVER_BROWSER=1, ou par install.sh --browser. Éteint, le serveur ne sert aucune page et refuse toute Origin web, comme aujourd'hui. Allumé, il sert la page embarquée dans le binaire sur le port de son API, et accepte l'Origin http://<Host> quand le Host est de la boucle locale. Choisi par l'utilisateur le 2026-11-26, avec la recommandation de l'étude.

## Conséquences

Les serveurs pilotés par le bureau ou le téléphone n'exposent rien de plus. Ceux qui allument le navigateur ont une surface de plus, couverte par BC-9 (revue de sécurité avant la bêta). Il faut ajouter l'option et sa variable à escouade-server, à install.sh et à l'image (BC-1, BC-6), puis les documenter sur le site et dans la spec (BC-8). Coût du chantier : environ 14 h d'agent (15 h 45 avec la clé WebCrypto, décision B2).
