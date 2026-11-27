---
number: null
id: "capture-browser-une-image-mcp-bornee-un"
title: "capture_browser : une image MCP bornée, un fichier de repli, 20 par minute et l'accord de la lecture"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-670"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-670"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":11,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-3 (ESC-670) donne aux agents la capture des navigateurs (docs/AGENT-BROWSERS.md § 3 bis). Il fallait fixer comment l'image arrive à un harness qui ne lit pas les images MCP, comment se prend une autre largeur selon le système, ce que compte la limite de débit, et quel accord vaut hors du lancement (C4 tranché : A, a-42).

## Décision

capture_browser rend une image MCP JPEG (qualité 70, 1 600 px et 600 Ko au plus : la qualité baisse puis l'image rapetisse ; un JPEG qui tient reste tel quel), puis, sous l'avis de donnée de tiers, url, width, height, scale, pixels, selector et file : la même image dans captures/<agent>/<n>.jpg du dossier de données, les 50 dernières gardées, supprimées avec l'agent. La vue, un élément (ce qu'il en montre dans la vue, rien ne défile) ou le document (pas sous macOS). Tablette 768 px et téléphone 390 px : émulés par le protocole DevTools sous Windows, sans toucher à la vue ; sinon (WebKit, et le retour à la pleine largeur), la fenêtre montre la page à cette largeur de la barre le temps de la capture, sans l'enregistrer. 20 captures par minute glissante et par agent ; une demande d'accord ne compte pas. Hors du lancement, le même accord que read_browser (même bande, même réponse asked) : un accord vaut pour lire et capturer, un refus refuse les deux. La consigne du serveur MCP dit qu'une capture d'une page connectée montre aussi ce qui l'entoure et demande de n'y capturer que le nécessaire.

## Conséquences

Un agent voit ce que voit l'utilisateur, en image, avec un repli pour Codex, OpenCode ou Copilot (à vérifier par un essai réel de chaque harness). La largeur par la barre sous WebKit n'est essayée que par AB-5. Les images restent sur le poste, dans le dossier de données, jusqu'à la suppression de l'agent. Nouvelle dépendance : image (jpeg, png).
