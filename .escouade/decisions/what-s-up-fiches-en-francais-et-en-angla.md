---
number: null
id: "what-s-up-fiches-en-francais-et-en-angla"
title: "What's up ? : fiches en français et en anglais, la dernière seule après des versions sautées, fiche facultative"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-699"
pages: ["introduction/livraison"]
tickets: ["ESC-699"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"introduction/livraison","added":8,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La PR 51 (2.2.17) montre une fois une fiche des nouveautés après une mise à jour. Ses fiches n'existaient qu'en français (une interface en anglais n'en voyait aucune), le composant montrait toutes les fiches plus récentes que la dernière vue (jusqu'à 5 écrans chacune), le premier lancement n'enregistrait la fiche vue qu'après un fetch (un rechargement rapide pouvait l'ouvrir), et le README imposait une fiche et un hook pre-push venus d'un plugin tiers.

## Décision

Chaque fiche existe en français et en anglais (public/whats-up/ et public/whats-up/en/) ; la fenêtre entre dans la page une fois la langue des réglages connue, avec src et lang de cette langue, sans modifier whats-up.js. Les ids des fiches sont connus au build : au premier lancement la dernière est enregistrée aussitôt ; après des versions sautées l'avant-dernière est donnée comme vue, donc seule la dernière s'ouvre. Trois fiches au plus dans le dépôt, tenu par un test. La fiche est facultative (seulement pour des nouveautés visibles), faite avec l'atelier et .whatsup/capturer.mjs (Playwright) ; le plugin osho-whats-up et son hook restent possibles, non exigés.

## Conséquences

Une fiche coûte deux jeux de captures (fr, en, clair et sombre) ; la plus ancienne sort quand une quatrième arrive. Le binaire garde au plus trois fiches d'images. Une interface en anglais voit désormais les nouveautés.
