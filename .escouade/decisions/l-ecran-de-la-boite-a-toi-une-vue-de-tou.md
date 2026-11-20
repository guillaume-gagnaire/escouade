---
number: null
id: "l-ecran-de-la-boite-a-toi-une-vue-de-tou"
title: "L'écran de la boîte « À toi » : une vue de tous les projets ouverte du rail, seulement serveur MCP allumé"
date: "2026-11-17"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-510"
pages: ["harness/boite-a-toi","fenetre/barre-des-projets","fenetre/raccourcis"]
tickets: ["ESC-510","ESC-509","ESC-513"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/boite-a-toi","added":35,"removed":9},{"page":"fenetre/barre-des-projets","added":10,"removed":3},{"page":"fenetre/raccourcis","added":12,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

IN-1 (ESC-509) a livré le cœur de la boîte « À toi » sans écran. L'utilisateur a choisi le 2026-11-17 la direction B « La file » (avec les vignettes de D dans la liste), une boîte qui n'existe qu'avec le serveur MCP, et sa sortie dans la ligne 2.2 depuis main, où ni l'aperçu PDF de la vue VS Code, ni mammoth, ni l'API distante n'existent.

## Décision

La boîte est une vue de tous les projets, ouverte par une tuile de la barre des projets sous « Vue d'ensemble » et par Ctrl+Maj+J, la file à gauche (groupes, jauge, vignettes des choix) et une demande à la fois en grand à droite, réponse en bas, tout au clavier (J, K, L, 1-6, Entrée). Sa tuile, son compteur, son raccourci et sa vue n'existent que serveur MCP allumé, et suivent l'interrupteur à chaud ; éteindre avec des demandes ouvertes le demande d'abord. Le compteur de la tuile compte les demandes ouvertes plus les agents arrêtés que compte « N en attente », jamais une demande reportée. Le PDF est lu par un module à part (src/lib/previews/pdf.ts, pdfjs-dist 6.4.299 comme release/2.3), une page sur un canevas, sans script ni chargement distant, la CSP inchangée ; Word et CSV restent des fichiers à ouvrir sur la 2.2. Les fichiers d'une demande close se gardent 7, 30 ou 90 jours (réglage inboxKeepDays, 30 par défaut). Six événements de statistiques s'ajoutent (inbox_opened, ask_answered, ask_snoozed, ask_reopened, inbox_compare_opened, inbox_attachment_opened).

## Conséquences

La boîte se voit et se traite dans la 2.2 sans attendre la 2.3. Le reversement dans release/2.3 doit réunir les deux aperçus PDF, classer inbox_save_file et inbox_track dans l'API distante et décider si les réglages imposés couvrent la boîte (docs/INBOX.md § 18). IN-3 (ESC-513) remplace le groupe « Tes agents se sont arrêtés » par ses vues réglées sur place, et y branche son total.
