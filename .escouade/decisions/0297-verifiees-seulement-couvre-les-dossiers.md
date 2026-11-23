---
number: 297
id: "verifiees-seulement-couvre-les-dossiers"
title: "« Vérifiées seulement » couvre les dossiers et les dépôts, et le harness d'un tiers demande l'accord à son premier lancement"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-589"
pages: ["plugins/harness-acp-decrits","store/store-dans-l-app","fenetre/reglages"]
tickets: ["ESC-589"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":12,"removed":0},{"page":"store/store-dans-l-app","added":8,"removed":0},{"page":"fenetre/reglages","added":3,"removed":0}]
accepted_at: "2026-11-23T20:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit de sécurité de la 2.2.0 (fiche SEC-01, gravité moyenne) a montré deux trous qui se combinent. « N'installer que des versions vérifiées » ne jugeait que le store : un plugin tiers s'installait toujours depuis un dossier ou un dépôt git, et une installation mise à jour gardait le réglage éteint. Et un plugin sans code peut décrire un harness, un programme nu du PATH lancé hors du bac à sable, gardé seulement par une liste de refus de 28 shells et interpréteurs que `npx`, `uvx`, `py`, `dotnet`, `java` ou `make` passent : un « harness pour l'outil X » qui déclare `npx un-paquet` faisait tourner le paquet avec les droits de l'utilisateur au premier agent. Deux options ont été présentées à l'utilisateur ; il a choisi le 2026-11-21 (a-3) « A · Étendre le réglage, accord au premier lancement ».

## Décision

1. Réglage allumé, un dossier ou un dépôt git n'est accepté que s'il est une version vérifiée du store : même id, même version, sceau tenu, mêmes fichiers. Sinon il est refusé avec sa raison et le geste qui l'autorise (couper le réglage). Le mode développement reste un geste à part, dit comme tel dans sa fenêtre, que le réglage ne couvre pas.
2. Le réglage s'allume une fois, à la mise à jour, pour une installation qui n'a aucun plugin tiers installé ; il ne change pas sinon. Une ligne des Nouveautés le dit.
3. Le harness qu'un plugin tiers décrit demande l'accord à son premier lancement, sa ligne de commande entière redite (programme trouvé, arguments, reprise, version, variables ajoutées et retirées, celles qu'Escouade pose). L'accord est gardé par l'empreinte de la ligne : une ligne changée par une mise à jour redemande l'accord. Refusé, l'agent ne démarre pas, et la conversation le dit. Les harness de la première main (`dev.escouade.*`) ne demandent rien. « Harness des plugins tiers : Refuser » n'en lance aucun ; c'est ce qu'imposera un poste géré en 2.3.
4. L'accord se demande au client qui lance : la ligne et l'accord sont des commandes de la liste unique.
5. La liste de refus des shells et interpréteurs reste un premier filtre du manifeste, plus la garde.

Pourquoi : fermer l'installation hors du store et le trou de la liste de refus sans retirer les harness tiers ; un accord relit la commande au moment où elle va tourner, là où la liste ne jugeait qu'un nom.

## Conséquences

Un utilisateur qui installait des plugins tiers depuis un dossier ou un dépôt, réglage allumé, doit couper le réglage ou passer à la version du store. Le premier agent d'un harness tiers ouvre une fenêtre d'accord ; chaque mise à jour qui change sa ligne la rouvre. Le cœur relit la ligne au lancement et refuse un lancement que le plugin décrirait autrement. Le réglage « Harness des plugins tiers » est le point d'appui de la stratégie d'un poste géré de la 2.3. Le rapport d'audit public dit SEC-01 corrigé dans la 2.2.6.
