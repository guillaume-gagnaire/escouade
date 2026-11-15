---
number: 181
id: "l-ecran-de-l-overseer-orbite-et-discussi"
title: "L'écran de l'Overseer (orbite et discussion), sa voix en « appuyer pour parler », son raccourci global, et une négation près d'une action qui ne donne jamais de consigne"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-468"
pages: ["overseer/overseer","fenetre/raccourcis","fenetre/reglages"]
tickets: ["ESC-468"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"overseer/overseer","added":48,"removed":11},{"page":"fenetre/raccourcis","added":5,"removed":1},{"page":"fenetre/reglages","added":5,"removed":3}]
accepted_at: "2026-11-15T19:20:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

OV-1 (décision 174) laissait à OV-2 l'écran de l'Overseer retenu sur maquettes (design/overseer : l'orbite à gauche, la discussion à droite, voix-tu-parles et voix-il-repond), sa voix, son raccourci global, et une limite : check_source vérifie qu'une consigne cite les mots de l'utilisateur et nomme l'action, pas le sens de la phrase, si bien que « ne merge surtout pas » nommait « merge ». Il fallait aussi trancher les faux amis (« n'hésite pas à merger »), le raccourci par défaut et sa forme sur les trois systèmes (Wayland n'en a pas), et comment la voix s'appuie sur la conversation vocale d'ESC-97 et se coupe par la stratégie de dictée d'ESC-465.

## Décision

1) Son écran remplace la discussion minimale, sur toute la fenêtre : l'orbite (orbe teal au centre ; tickets et agents sans ticket de tous les projets non privés, à la couleur de leur projet ; en cours en haut, 6 au plus ; ce qui attend l'utilisateur à droite, en ambre et plus près, 4 au plus ; le terminé des dernières 24 h en bas puis à gauche, plus loin et plus pâle avec l'âge, 4 au plus ; une tâche dont la nouvelle a moins de 8 s fond vers l'orbe et s'illumine ; un clic ouvre son agent, sinon le Kanban) et la discussion de verre (messages « dit » ou « tapé » avec l'heure, réponses, actions avec « Annuler » ou « Retirer », nouvelles en cartes — gardées par le cœur, la dernière par ticket, 30 au plus — avec « Valide-le », « Renvoie-le », « Regarde ce qui bloque » envoyés comme messages et « Montre-moi le diff » qui ouvre le diff dans la fenêtre ; « Consignes · n »). Rien ne bouge avec prefers-reduced-motion. 2) Sa voix est la conversation vocale d'ESC-97 menée en « appuyer pour parler » : Espace maintenu (champ vide) ou le micro ; relâcher envoie sans attendre le silence (nouvelle commande conversation_end), Échap jette ; le message part « dit » avec la consigne de finir par un passage lu ; sa réponse est lue avec sous-titres ; parler ou taper l'interrompt ; le micro n'entend rien entre deux appuis. La dictée coupée par la stratégie du poste coupe aussi sa voix (overseer_voice_start et le message dit passent par check_dictation). Sans voix ici, le champ seul et un lien vers Plugins. 3) Son raccourci global, par tauri-plugin-global-shortcut tant qu'il est allumé : Alt+Maj+O sous Windows et Linux (Ctrl+Alt est AltGr), ⌥⌘O sous macOS ; réglable dans Réglages › Overseer (il n'y a pas de page Raccourcis des réglages) ; le nouveau est enregistré avant de rendre l'ancien, refusé s'il est pris ; il ouvre l'écran par le lien escouade://overseer, que l'on lie sous Wayland à un raccourci du bureau (xdg-open). 4) Une consigne est refusée quand une négation est dans les cinq mots avant ou les deux après un mot de l'action, dans le message de l'utilisateur et non la seule citation ; les tournures qui disent oui par une négation (n'hésite pas, n'oublie pas, pas de souci, sans attendre, sans me demander, don't hesitate, don't forget, feel free, without asking, no problem…) n'en sont pas, « ne… que » non plus ; une négation qui reste une fois la tournure ôtée nie encore ; l'Overseer demande alors une confirmation en clair.

## Conséquences

Une phrase niée ne donne plus de consigne, prouvé par les tests Rust (français, anglais, faux amis, outil add_directive) et l'e2e ; en échange, un oui proche d'une négation (« Non, merge-le », « Merge-le, ne pousse pas ») demande une confirmation, et une négation plus loin ou une tournure inconnue n'est pas vue (dit dans les Limites). La voix de l'Overseer n'écoute que pendant l'appui. Le raccourci global n'existe pas sous Wayland ; les e2e l'enregistrent dans un bac à sable et le pressent par overseer_hotkey_press, l'appel au système n'étant couvert que par les tests Rust (faux clavier) et à la main. Une nouvelle dépendance, tauri-plugin-global-shortcut (cargo deny passe).
