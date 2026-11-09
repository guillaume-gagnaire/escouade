---
number: 135
id: "un-geste-qui-ouvre-une-pr-dans-le-naviga"
title: "Un geste qui ouvre une PR dans le navigateur dit toujours pourquoi il échoue"
date: "2026-11-09"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-402"
pages: ["git-fichiers/pull-requests"]
tickets: ["ESC-402"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":4,"removed":0}]
accepted_at: "2026-11-09T16:02:25+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Retour de bêta 2.2 (2026-11-09) : « Le bouton pour ouvrir une PR dans le navigateur ne fonctionne pas. » Les boutons passaient par openOutside, qui avalait tout refus de open_url (catch muet) et ignorait sans rien dire une adresse vide ou qui n'est pas une page web ; la relecture faisait un `void openUrl`. Un échec ne se voyait donc nulle part : le bouton « ne faisait rien ».

## Décision

Tous les gestes qui ouvrent une PR (numéro sous la carte, en-tête, barre d'état, « Voir la PR #n », icône de « PR et CI », de la vue « Pull requests », de la relecture, PR d'un ticket « En revue ») passent par un seul chemin, openPrPage. Il ouvre la page web que donne l'hébergeur ; une adresse vide ou qui n'est pas http(s) n'est pas envoyée au système ; tout échec se dit dans un toast d'erreur « La PR #n ne s'ouvre pas dans le navigateur : <raison> ».

## Conséquences

Plus de catch muet sur ces gestes. Les adresses des cinq hébergeurs sont vérifiées sur fixtures, listes comprises ; aucun plugin n'avait d'adresse fautive, donc aucun plugin n'est changé. Les autres liens (conversation, terminaux) gardent leur comportement.
