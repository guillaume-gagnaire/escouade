---
number: null
id: "drive-browser-pilote-les-onglets-et-les"
title: "drive_browser pilote les onglets et les pages des agents (AB-7) : paliers, refus, règle de la main relevée par sondage"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-679"
pages: ["kanban/navigateurs-pilotes","harness/pages-des-agents"]
tickets: ["ESC-679"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":43,"removed":11},{"page":"harness/pages-des-agents","added":21,"removed":13}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-7 (avec AP-6) construit le pilotage commun d'ESC-666 : un seul outil drive_browser pour toutes les lignes de « Navigateurs ». L'étude laissait ouverts quelques points d'exécution : quel accord vaut pour agir hors du lancement, ce que fait navigate vers une autre origine, comment la borne de 4 gestes par seconde se tient, ce que deviennent les gestes de l'utilisateur pendant qu'il a la main, et comment montrer avant la bande d'AB-8.

## Décision

drive_browser est construit sur tous les onglets et pages des agents, aux paliers de § 9 (montrer sans demander partout ; agir sans demander sur le lancement, avec accord ailleurs, tab seul sur une page ; version sur une page seulement). Choix d'exécution : (1) hors du lancement, l'accord d'agir est celui de read_browser (« lire et agir », une fois par agent, origine et navigateur, la même bande) ; (2) navigate ne va qu'à l'origine du navigateur ou au lancement, une autre origine passe par open_browser qui la demande ; (3) refus avant tout geste : fichier, téléchargement (download, blob:, data:), champ ou bouton d'un formulaire dont l'action va vers une autre origine, secret d'Escouade (valeurs connues et formes de jetons), mot de passe hors du lancement toujours, sur le lancement avec l'accord C2 tenu par navigateur et agent (commande browser_drive_password) ; (4) sous Windows, vraies entrées CDP (clic, insertText, touches) vérifiées par la garde de la liste fermée, qui n'a pas grandi ; sous WebKit, entrées synthétiques du monde isolé, dites synthetic ; (5) règle de la main : gestes isTrusted comptés hors de la fenêtre des gestes de l'agent, relevés à chaque appel et toutes les 500 ms : sous Windows par le monde isolé, qui reste d'un appel à l'autre ; sous WebKit, dont le monde isolé ne garde rien d'un appel à l'autre (ESC-692), par un relais posé dans le monde de la page (browserdrive-gestures.js, onglets et pages d'agent) que le monde isolé relève par un événement, comme le relais de console ; la page peut fausser ce relais (un geste inventé ne fait que donner la main à l'utilisateur, un geste tu lui laisse « Reprendre la main ») ; ce que l'utilisateur fait pendant qu'il a la main est oublié quand il la rend ; (6) 4 gestes par seconde : un geste attend son tour dans la seconde (1 s au plus) plutôt que d'être refusé, l'heure pleine est refusée ; (7) en attendant AB-8, le contour de highlight est un élément d'Escouade posé par le monde isolé sous tous les systèmes, placé dans les coordonnées du document et retrouvé par sa marque (rien n'attend dans le monde isolé), et la fenêtre reçoit les phases drive et mode (une largeur choisie par l'agent est gardée comme celle de l'utilisateur) ; (8) version sur une page d'agent fait ce que fait « v2 ▾ » de la vue d'AP-3 (page_show_version), la fenêtre prévenue par la phase version.

## Conséquences

AB-8 branche la bande (Reprendre la main, La rendre, Continuer, la demande de mot de passe) sur browser_drive_state et browser_drive_hand, et passe le surlignage de Windows à Overlay.highlightNode (DOM.enable et Overlay.enable à ajouter alors à la liste fermée). capture_browser et browser_console refusent encore les lignes page: (look y capture). Les entrées WebKit restent à essayer sur Linux et macOS (AB-5, AB-12). Le cadre d'un artefact reste à AB-11. Avec AP-4 (ESC-676) : une page que son agent pilote ne propose aucun choix pendant un geste de l'agent ni dans les 2 s qui suivent (le cœur refuse ce _/choose) ; « Envoyer » et tout feu vert restent les gestes de l'utilisateur.
