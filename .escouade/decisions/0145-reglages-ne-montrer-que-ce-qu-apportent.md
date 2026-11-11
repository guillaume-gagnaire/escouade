---
number: 145
id: "reglages-ne-montrer-que-ce-qu-apportent"
title: "Réglages : ne montrer que ce qu'apportent les plugins installés"
date: "2026-11-11"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-407"
pages: ["fenetre/reglages"]
tickets: ["ESC-407"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages","added":28,"removed":13}]
accepted_at: "2026-11-11T14:38:25+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis ESC-373, Réglages › Comptes et harness listait, sous les harness installés, ceux de l'amorce dont le plugin n'était pas installé (« Non installé », « Installer <harness> »), et gardait le groupe d'un harness qui avait des comptes même sans son plugin. Services connectés, Tickets importés, Voix et Confidentialité listaient en dur toutes les sources, hébergeurs, services de voix et harness, installés ou non. L'utilisateur, le 2026-11-11, en voyant OpenCode « Non installé » : « Dans comptes et harness, t'es sensé retrouver que les harness des plugins installés. Et pareil pour tous les paramètres liés à des trucs installables par plugins. »

## Décision

Une page des réglages ne liste que ce qu'apporte un plugin installé : un harness (son plugin de première main, associé comme le fait le tri de l'amorce, ou un plugin qui le déclare dans contributes.harnesses ; le cœur, lui, garde ses harness déclarés sans leur plugin), une source de tickets (contributes.ticketSources), un hébergeur de code (contributes.forges), un service de voix en ligne (activation voice:<id>, plugin actif). Aucune offre « Non installé » ni bouton d'installation dans les réglages hors de la page Plugins : on installe dans Réglages › Plugins, dans la vue Plugins et à l'accueil, qui ne changent pas. Ce qui est gardé d'un plugin désinstallé (comptes, place dans l'ordre des harness, lien d'un projet à une source, service choisi, clés du trousseau) reste sur le disque, caché, et revient à sa réinstallation. Une partie qui n'a plus rien à montrer le dit en une ligne avec « Ouvrir Réglages › Plugins ». La recherche des réglages ne trouve pas ce qu'une page ne montre pas faute de plugin. La fenêtre relit les plugins installés à son ouverture.

## Conséquences

Comptes et harness perd ses offres de l'amorce ; engineOrder() ne garde que les harness dont le plugin est installé, ce qui cache aussi leurs comptes dans le panneau des quotas, la barre d'état et la passation tant que le plugin manque. Services connectés, Tickets importés, Voix, Agents et Confidentialité filtrent par les plugins installés (src/lib/installed.ts) ; tant que la liste des plugins n'est pas lue, tout est montré. La table des harness de première main et de leur plugin est tenue des deux côtés (plugins::sort::of_engine dans le cœur, installed.ts dans la fenêtre). Les tests de composants partent des plugins de première main installés (src/test/plugins.ts). Un plugin tiers qui sert les comptes d'une source déjà déclarée n'en fait pas apparaître une nouvelle ; une build de développement garde les plugins du dépôt, qui ne se désinstallent pas.
