---
number: null
id: "les-agents-lisent-les-navigateurs-qui-le"
title: "Les agents lisent les navigateurs : qui, le lancement, l'accord par agent, origine et navigateur"
date: "2026-11-26"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-669"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-669"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":15,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-2 (ESC-669) construit les deux premiers outils de lecture des navigateurs, list_browsers et read_browser, sur le pont d'AB-1, pour la 2.2 (de main). L'étude ESC-663 (docs/AGENT-BROWSERS.md § 3 bis, § 5) fixe le principe (lancement sans demander, autre origine avec accord, C4 : A) ; restaient à trancher dans le code : ce qu'est « le lancement du projet », à qui l'accord est donné, ce que devient un refus, ce que voit Claude hors d'Escouade, et où vit l'interrupteur tant que main n'a pas Fonctionnalités.

## Décision

1. L'interrupteur « Agents et navigateurs » (clé agentBrowsers, allumé par défaut) est dans Réglages › Agents, sous « Les agents peuvent piloter Escouade » ; coupé, les deux outils refusent et la consigne du serveur MCP ne parle plus des navigateurs (elle est donnée par le serveur selon le réglage). 2. Qui : un agent lit les navigateurs de son projet seul ; une discussion ceux qu'elle a ouverts ; Claude hors d'Escouade liste ceux d'un projet non privé et n'en lit aucun ; un projet privé n'existe que pour ses agents (même réponse qu'une clé inconnue). 3. Le lancement du projet, lu sans demander : les origines des adresses des commandes de lancement (de l'utilisateur, aux ports du projet et des worktrees), et localhost / 127.0.0.1 / [::1] aux ports que déclarent ces commandes, les lancements de test des agents et leurs blocs réservés ; une adresse écrite par un agent ne compte jamais pour un site d'internet. 4. L'accord hors du lancement est donné à un agent, pour une origine et un navigateur, jusqu'à la fermeture du navigateur ; un refus tient aussi longtemps ; 5 demandes au plus par navigateur ; la réponse revient à l'agent par un message ; « Retirer les accords » (menu de la ligne) fait tout tomber et prévient un agent qui attendait. 5. Un navigateur sans placement à l'écran (la fenêtre dit full, beside, below ou hidden) se liste mais ne se lit pas.

## Conséquences

Un agent peut voir ce que voit l'utilisateur sur l'app qu'il développe sans rien demander, et jamais une page de site tiers connectée sans un geste de l'utilisateur dans une bande native que la page ne peut pas imiter. L'accord par agent (plutôt que par navigateur seul) est plus strict que l'étude ne l'écrit : un autre agent du même projet redemande. AB-3 (capture_browser) et AB-4 (browser_console) reprennent le même accord (devpage::Pages::leave). En 2.3 (AB-R), l'interrupteur passe dans Réglages › Fonctionnalités sous la même clé, et les outils dans la partie « Lancement et navigateurs ».
