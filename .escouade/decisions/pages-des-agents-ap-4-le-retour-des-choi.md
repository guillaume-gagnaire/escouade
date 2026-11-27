---
number: null
id: "pages-des-agents-ap-4-le-retour-des-choi"
title: "Pages des agents AP-4 : le retour des choix par la boîte, proposé par la page, envoyé par la bande d'Escouade"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-676"
pages: ["harness/pages-des-agents","kanban/pages-des-agents"]
tickets: ["ESC-676"]
documents: ["docs/AGENT-PAGES.md","docs/AGENT-BROWSERS-TICKETS.md"]
supersedes: []
superseded_by: null
changes: [{"page":"harness/pages-des-agents","added":19,"removed":7},{"page":"kanban/pages-des-agents","added":7,"removed":5}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Une page d'agent est du HTML écrit par un agent qui a pu lire un contenu hostile. AP-4 (ESC-676) lui donne un retour vers son agent (docs/AGENT-PAGES.md § 6, P3 tranché : un clic dans la page propose, la bande d'Escouade envoie) sans qu'une page puisse répondre seule, ni proposer un feu vert.

## Décision

1) `ask` de `show_page` (choice ou confirm, les champs d'ask_user sans fichiers) dépose la demande par le même appel ; retirée si la page n'est pas montrée ; une version avec un autre `ask` retire l'ancienne demande qui attendait. 2) `_/choose` est le seul canal : POST, corps fermé `{options, note}` (4 Ko, note de 2 000 caractères), une proposition par seconde, à la seule webview de la page ; il ne fait que proposer, vérifié par le cœur (demande ouverte, pas un feu vert, ses options seules, multi) ; tout le reste est un 404 muet. 3) La bande du choix, native et hors de la webview, dit la proposition avec les mots de la demande lus dans le cœur et l'envoie par la commande de la boîte (inbox_answer) sur le seul clic « Envoyer », armé 0,7 s après le dernier changement de la proposition. 4) Un feu vert n'est jamais dessiné dans la page ni proposé depuis elle : la bande a son propre Non et Oui. 5) La réponse est rendue à la page par un script fixe qui ne porte que des identifiants d'options (coche, fige, événement escouade:answered), par la webview elle-même en attendant le monde isolé d'AB-7. 6) « À toi » montre la page d'une demande (vignette dessinée par la fenêtre, jamais la page) avec « Ouvrir la page ». Pas encore : l'étiquette « choix » de la ligne, le pied de la carte, Claude hors d'Escouade avec une page (P9).

## Conséquences

Une page piège ne peut qu'allumer la bande avec une option de sa propre demande, dite dans les mots de la demande ; rien ne part sans le geste dans l'app. window.escouade gagne ask et choose. Le script de la réponse tourne dans le monde de la page (elle peut défaire son affichage, pas la réponse) jusqu'à AB-7.
