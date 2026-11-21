---
number: 261
id: "l-entonnoir-du-premier-lancement-se-dit"
title: "L'entonnoir du premier lancement se dit à sa fin, lu dans son état gardé, avec l'accord donné à sa dernière station"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-507"
pages: ["fenetre/premier-lancement","fenetre/a-decouvrir"]
tickets: ["ESC-507"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/premier-lancement","added":17,"removed":0},{"page":"fenetre/a-decouvrir","added":5,"removed":0}]
accepted_at: "2026-11-21T19:34:24+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ON-4 (ESC-507) ajoute les sept événements de statistiques du § 9 de docs/ONBOARDING.md. Or la question des statistiques est la dernière station du parcours (choix O7) : un événement envoyé quand une station se ferme partirait avant l'accord, et la règle d'Escouade est de ne rien garder avant le oui (Telemetry::set_consent). Il fallait aussi dire ce que deviennent `trigger: update` (aucun parcours ne s'ouvre après une mise à jour : la bande ouvre la découverte) et la station « Ce que tu veux brancher », venue après la conception.

## Décision

Les événements du parcours (`onboarding_started`, `onboarding_step`, `onboarding_finished`, `first_reply`) ne sont pas mis en file au fil des stations : ils sont calculés, une fois le passage fini, à partir de l'état gardé du parcours dans state.json (début du passage, profil trouvé, choix recommandé gardé ou non par station, heure de la première réponse, `told`), envoyés si l'accord est donné, oubliés au non, gardés tant qu'il n'y a pas de réponse (un oui plus tard les envoie, une fois). `trigger` vaut `first` ou `redo` (pas `update`) ; `step` compte `hookups` ; `skipped` va de 0 à 7. Les événements de la découverte (`discover_done` avec `from: tip | other`, `discover_dismissed`, `discover_hidden`) partent au geste, pas quand la relecture de l'état réel coche une découverte. Le parcours dit « station » partout, jamais « étape ».

## Conséquences

Aucune donnée ne part avant le oui, et aucun événement n'est écrit sur le disque : seul l'état du parcours, déjà gardé, porte cinq champs de plus. Le taux d'arrivée au premier agent se lit `first_reply` ÷ `onboarding_started{trigger=first}` par semaine. Les événements arrivent tous à la fin du passage (leur heure est celle de la fin, pas de chaque station) ; un premier tour fini après le parcours n'est pas compté. La page de confidentialité du site et le registre des traitements comptent 48 événements.
