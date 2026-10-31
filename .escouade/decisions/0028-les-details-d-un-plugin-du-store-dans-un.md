---
number: 28
id: "les-details-d-un-plugin-du-store-dans-un"
title: "Les détails d'un plugin du store dans une modale ouverte par « ⋯ »"
date: "2026-10-31"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-318"
pages: ["store/espace-de-l-auteur"]
tickets: ["ESC-318"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/espace-de-l-auteur","added":16,"removed":0}]
accepted_at: "2026-11-01T00:33:00+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Sur /account, choisir un plugin rechargeait la page et sa gestion (versions, liste, publication auto, transfert, suppression) s'affichait sous la liste, ses installations dans la colonne de droite : loin de sa ligne, et le premier plugin était choisi sans qu'on le demande. L'utilisateur a demandé le 2026-10-31 d'ouvrir les détails dans une modale par les « ⋯ » et de revoir l'UX avec impeccable.

## Décision

Chaque ligne de « Mes plugins » a un « ⋯ » qui ouvre la modale du plugin (direction A « la fiche », choisie sur aperçu parmi trois : fiche, onglets, tiroir) : chiffres, versions, publication, puis à part « Céder ou supprimer ». L'adresse porte l'état (?plugin=<id>), sans plugin choisi par défaut ; les formulaires restent des POST suivis d'une redirection qui rouvre la modale, sans JavaScript. La colonne de droite ne garde que le compte, avec la place des « Applications connectées » d'ESC-317.

## Conséquences

La section #manage et la carte des installations de la colonne de droite disparaissent ; la colonne « Mises à jour » des versions quitte la modale (le total reste). Un composant StoreDialog (dialog natif, focus piégé et rendu, Échap) sert à d'autres modales du store. Les e2e author.spec et account.spec passent par la modale, au clavier, sur un téléphone et sans script.
