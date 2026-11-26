---
number: null
id: "le-relais-de-console-de-webkit-ne-rend-s"
title: "Le relais de console de WebKit ne rend son lot que sur demande, dans l'appel du cœur"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-692"
pages: ["kanban/page-du-lancement","kanban/navigateurs-pilotes"]
tickets: ["ESC-692","ESC-668","ESC-672"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/page-du-lancement","added":3,"removed":3},{"page":"kanban/navigateurs-pilotes","added":4,"removed":4}]
accepted_at: ""
accepted_by: ""
---

## Contexte

ESC-672 a mis la sonde agentbrowser_probe en CI ; sous macOS (WKWebView, CI de main 45870498799), l'étape console ne gardait ni l'exception non rattrapée ni les requêtes en échec (le 404, le port refusé). Le relais de la page disait ses entrées au collecteur du monde isolé escouade-agent par un événement, une fois ce collecteur « prêt ». Or sous WebKit, le monde isolé ne garde rien d'une évaluation à l'autre (la sonde le mesure : worldKept faux sous WKWebView et sous WebKitGTK, vrai sous WebView2) : seul ce que la première relève vidait arrivait, tout le reste se perdait. Sous Linux, cela ne passait que parce que tout arrivait avant la première relève (sous WSL, une fois, le 404 s'est perdu aussi).

## Décision

Le relais garde ce qu'il voit (500 entrées, les plus anciennes perdues et comptées) et ne pousse rien. Chaque seconde, le cœur le lui demande depuis le monde isolé par un événement, et le relais rend son lot dans ce même appel, puis l'oublie. Aucun script fixe ne compte sur ce qu'un appel d'avant a laissé dans le monde isolé. Windows (WebView2, événements du protocole) ne change pas.

## Conséquences

Sous macOS, la console d'un navigateur garde l'exception, le 404 et la requête refusée (run 45881504697 de la branche d'étude study/esc-692-macos). Sous Linux, elle garde aussi ce que la page dit après son chargement. La sonde exige une entrée et un 404 tardifs (2,5 s) et mesure worldKept ; un test Vitest rejoue le relais et la relève, chaque relève dans un monde neuf. Aucune limite ne reste : rien à dire sur le site.
