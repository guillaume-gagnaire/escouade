---
number: 226
id: "fonctionnalites-tout-allume-sur-une-inst"
title: "Fonctionnalités : tout allumé sur une installation neuve, ce qu'on utilise relevé et gardé allumé à la mise à jour"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-550"
pages: ["fenetre/fonctionnalites","fenetre/reglages","fenetre/reglages-imposes","fenetre/a-decouvrir","harness/serveur-mcp"]
tickets: ["ESC-550"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/fonctionnalites","added":56,"removed":0},{"page":"fenetre/reglages","added":4,"removed":0},{"page":"fenetre/reglages-imposes","added":6,"removed":0},{"page":"fenetre/a-decouvrir","added":2,"removed":0},{"page":"harness/serveur-mcp","added":2,"removed":0}]
accepted_at: "2026-11-20T13:36:48+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

F-1 (ESC-550) pose le registre des fonctionnalités entières (Kanban, Éditeur, Espace Docs, Coût, Statistiques, Workflows, Tâches programmées, Overseer, Voix), chacune coupable d'un interrupteur qui emporte ses vues, ses outils MCP, ses découvertes et ce qu'elle fait en fond. Il faut décider de leur valeur pour une installation neuve et pour une installation qui passe à la version qui les apporte, avant que l'assistant du premier lancement (F-2) n'existe. Décidé avec l'utilisateur le 2026-10-28 (docs/FEATURES.md § 1) : après une mise à jour, ce qu'on utilise reste allumé ; « Passer » de l'assistant ne garde que l'essentiel.

## Décision

1. Une installation neuve a toutes les fonctionnalités allumées (NEW_INSTALL = true) jusqu'à ce que l'assistant de F-2 fasse le choix : rien n'est caché à qui ne voit pas l'assistant (parcours passé, poste géré, client d'un serveur). 2. Une installation mise à jour règle ses interrupteurs une fois, à son premier démarrage : elle relève ce qu'elle utilise (projets avec des tickets, une spec, des décisions ou des documents, une branche des docs ou une discussion ; l'éditeur ouvert, ses extensions ou serveurs de langage ; des workflows lus ; des tâches programmées ; l'Overseer allumé ou parlé ; la voix réglée ou un modèle téléchargé ; la vue des statistiques ouverte ; un budget par jour ou les économies du cache dans la barre), l'allume et garde ce relevé (featuresUsed, avec le nombre de projets) ; le reste suit la valeur d'une installation neuve, donc allumé aussi. 3. Le relevé ne se refait jamais. Réglages › Fonctionnalités le montre (« Utilisée sur 2 projets »), et l'assistant de F-2 s'en servira : il dira « Tu l'utilises déjà… : il reste allumé » et « Passer » ne coupera jamais une fonctionnalité utilisée. 4. Les Tâches programmées partent avec le Kanban (leurs passages sont des tickets). 5. Chaque fonctionnalité est imposable par le poste géré et par l'équipe, dans les deux sens (features.<id>, sorte valeur).

## Conséquences

Personne ne perd rien en mettant à jour ni en installant : l'app reste celle d'avant tant que l'utilisateur ne coupe rien ou que F-2 ne lui demande pas. Le relevé, aujourd'hui sans effet visible sur les interrupteurs (tout est allumé), prépare l'assistant de F-2. Si F-2 change la valeur d'une installation neuve (couper ce qui n'est pas choisi), seule la constante NEW_INSTALL change : le relevé garde alors allumé ce qui est utilisé, ce que le test de la reprise vérifie déjà avec une valeur coupée. Coût : neuf stratégies de plus dans l'ADMX, les profils, le schéma de /etc et la page du site.
