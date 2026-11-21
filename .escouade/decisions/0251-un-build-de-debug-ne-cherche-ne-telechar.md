---
number: 251
id: "un-build-de-debug-ne-cherche-ne-telechar"
title: "Un build de debug ne cherche, ne télécharge ni n'installe jamais une vraie version d'Escouade"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-543"
pages: ["introduction/livraison"]
tickets: ["ESC-543"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/livraison","added":2,"removed":0}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Les apps lancées par les e2e (builds de debug) lisaient le vrai latest.json (l'adresse de tauri.conf.json, ESCOUADE_UPDATE_ENDPOINT n'étant donné que par les tests des mises à jour) et téléchargeaient la version publiée quand la branche était en retard (captures d'ESC-538 et d'ESC-539 : « Mise à jour 2.2.4 prête »). « Quitter » installe la mise à jour prête : un test ou un build de développement fermé proprement aurait lancé le vrai installeur NSIS en silencieux, dont le dossier est celui de l'app de l'utilisateur, fermée d'abord par le Restart Manager.

## Décision

Dans un build de debug, la recherche ne lit qu'un serveur de test : ESCOUADE_UPDATE_ENDPOINT, ou les adresses de la configuration quand elles sont toutes sur cette machine (localhost, 127.x.x.x, ::1 : l'app des tests Rust). Sans lui, elle est coupée : rien de neuf en automatique, la raison dite à « Rechercher une mise à jour », le journal le note. Un build de debug ne télécharge et n'installe qu'un installeur venu d'une adresse sur cette machine ; un autre est refusé au téléchargement et à install_ready (« Quitter » ferme alors l'app comme d'habitude). La fixture des e2e retire toute ESCOUADE_UPDATE_ENDPOINT héritée ; seuls les tests des mises à jour donnent la leur, et la fin de chaque test échoue si le journal de son app dit une recherche vers une autre adresse ou une installation. Un build de release ne change pas.

## Conséquences

Aucun e2e, test Rust ou build de développement ne peut plus installer une vraie version sur le poste. Pour essayer une vraie mise à jour, il faut un build de release (ou un serveur de test sur cette machine qui sert un installeur d'essai). Le journal de chaque recherche dit les adresses lues (« update check reads … »), ce que lit la garde des e2e. Pas de ligne de CHANGELOG : rien ne change pour un build publié.
