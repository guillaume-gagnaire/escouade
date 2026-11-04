---
number: 69
id: "un-plugin-qui-decide-demarre-des-qu-il-d"
title: "Un plugin qui décide démarre dès qu'il devient prêt et activé"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-329"
pages: ["plugins/regles-d-autorisation"]
tickets: ["ESC-329"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/regles-d-autorisation","added":4,"removed":1}]
accepted_at: "2026-11-04T20:11:55+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'essai réel du store (ESC-184, docs/PLUGINS.md §7.1) a montré qu'un plugin qui décide des autorisations ne démarrait, hors lancement de l'app, qu'avec la première demande : team-rules 1.1.1 par Node a pris plus de 3 s, et la règle de l'équipe ne s'appliquait pas à cette demande, partie à l'utilisateur.

## Décision

Escouade démarre un plugin qui décide avec l'app et aussitôt qu'il devient prêt et activé : installation (store, dossier, dépôt git), mise à jour, réactivation, permission decide rendue, dossier en développement relu. Le plugin en marche est arrêté avant que sa nouvelle version ne remplace son dossier. Le délai de décision (500 ms, 3 s à froid) et le repli vers l'utilisateur ne changent pas.

## Conséquences

La règle s'applique dès la première demande. Un plugin qui ne décide pas reste démarré à sa première utilité. La mise à jour d'un plugin en marche ne bute plus sous Windows sur le dossier tenu par son process. Reste un cas hors du ticket : une révocation levée par le store ne redémarre pas le plugin, qui repart au prochain lancement de l'app ou à la première demande.
