---
number: null
id: "drive-browser-atteint-le-cadre-d-une-aut"
title: "drive_browser atteint le cadre d'une autre origine par sa session sous Windows, le cadre entier sous WebKit"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-682"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-682"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":12,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Un artefact de claude.ai s'affiche dans un cadre d'une autre origine et d'un autre site. scroll_to et highlight ne visaient que le document principal ; l'étude (§ 2 bis.4) laissait AB-11 trancher comment atteindre ce cadre sans ouvrir son contenu à la lecture.

## Décision

drive_browser prend frame (l'<iframe>, par un sélecteur) pour scroll_to et highlight seuls, au palier montrer. Sous Windows, le geste se fait dans le monde isolé du cadre : un cadre d'un autre site (son propre processus) est atteint par sa session du protocole DevTools, connue par Target.setAutoAttach (iframes seules, à plat, jamais en pause : seuls paramètres permis) ; dans cette session, seuls Page.getFrameTree, Page.createIsolatedWorld et Runtime.evaluate des scripts d'Escouade. Sans target, sous WebKit, ou quand le cadre n'est pas atteint : le cadre entier (frame: whole). Le contenu du cadre reste hors de read_browser et de browser_console ; l'agent n'en reçoit que « trouvé » ou non.

## Conséquences

La liste fermée gagne Target.setAutoAttach (ni getTargets ni attachToTarget, qui atteindraient d'autres webviews). Chaque navigateur Windows auto-attache ses iframes dès sa création ; les événements des sessions de cadres sont écartés des tampons. Le cas phare de l'artefact peut montrer un passage précis sous Windows ; sous macOS et Linux, le cadre entier, à vérifier par AB-12 et la CI.
