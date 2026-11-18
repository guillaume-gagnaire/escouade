---
number: 206
id: "le-pilote-acp-reprend-sans-session-en-le"
title: "Le pilote ACP reprend sans session en le disant et redonne les consignes ; le manifeste gagne acp.resume, turnEnd, efforts, effortOption et usage"
date: "2026-11-16"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-491"
pages: ["plugins/harness-acp-decrits"]
tickets: ["ESC-491","ESC-484","ESC-485"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/harness-acp-decrits","added":12,"removed":0}]
accepted_at: "2026-11-18T12:15:06+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les essais d'Unharnessed (ESC-484) et de Prime Agent (ESC-485) ont montré ce que le pilote ACP ne tenait pas : après un redémarrage, une session que le harness ne recharge pas était remplacée sans le dire et sans redonner les consignes du ticket ; un tour lancé par le harness s'affichait hors tour et se mêlait à la réponse suivante ; thought_level n'était pas posé ; un modèle à valeur composée était refusé ; le contexte dit dans un format à lui n'était pas lu ; le code entier d'un appel long ne se lisait pas.

## Décision

Pour tout harness décrit par ACP sans extension : une session non reprise (loadSession faux, ou session perdue) est dite, et la session neuve reçoit avec le prochain message les consignes de l'agent et le résumé de ce qui a été fait (le dossier de passation d'ESC-88 tourné en reprise, LaunchParams.recap). Pour un manifeste qui a acp.turnEnd, un contenu hors de tout session/prompt ouvre un tour du harness, fini par sa marque, sinon par le message suivant ou l'arrêt ; sans acp.turnEnd, rien ne change (les flux enregistrés des autres harness restent lus à l'identique). Le manifeste gagne, facultatifs : acp.resume (args avec {session}, session : pointeur JSON vers l'id gardé), acp.turnEnd, acp.efforts et acp.effortOption (l'effort par l'option de réflexion, thought_level par défaut, posée seulement quand la session l'offre), acp.usage (context, window). Une valeur de modèle faite de parties est trouvée par sa dernière partie ou ses parties jointes par /. Le détail d'un appel d'outil inconnu montre son entrée longue en entier, repliée. Les nouveaux champs sont jugés par le lecteur du manifeste (codes pointer et session).

## Conséquences

Le contrat reste compatible : un manifeste d'aujourd'hui marche sans changement, une app plus ancienne ignore les nouveaux champs. escouade-plugin-protocol passe en 0.1.3, escouade-acp en 0.1.7, le plugin dev.escouade.acp en 2.2.5, en AWAITING_PUBLICATION jusqu'à la publication des deux crates (son Cargo.lock à réécrire ensuite) ; le résumé de reprise demande l'app 2.2.3. Unharnessed reprend sa session par --resume ; Prime Agent pose son effort par thought_level et dit sa session neuve. Un plugin ACP recréé en plein travail d'Unharnessed ne retrouve pas l'id vivant de sa session (il ne garde que l'id enregistré) : cas rare, non traité.
