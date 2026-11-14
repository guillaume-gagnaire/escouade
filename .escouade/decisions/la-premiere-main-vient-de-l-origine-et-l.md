---
number: null
id: "la-premiere-main-vient-de-l-origine-et-l"
title: "La première main vient de l'origine, et les exclusions du proxy n'ont qu'une lecture"
date: "2026-11-13"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-435"
pages: ["plugins/livres-avec-escouade","fenetre/reseau"]
tickets: ["ESC-435"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":8,"removed":2},{"page":"fenetre/reseau","added":5,"removed":2}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le contre-audit de la 2.2.0-beta.12 (ESC-418) relève deux constats faibles. SEC-27 : un dossier chargé « en développement » sous un identifiant `dev.escouade.*` recevait les droits de la première main, parce que la main se décidait sur le préfixe de l'identifiant (essai B-14). SEC-28 : la case « Ignorer la vérification des certificats TLS » valait encore hors du proxy pour un hôte exclu par une plage d'adresses, parce que la route (la bibliothèque HTTP) et la vérification (`proxy_for`) lisaient les exclusions chacune à sa façon (essai W-28). L'utilisateur a décidé le 2026-11-13 de les corriger avant la 2.2 finale, sur main et sur release/2.2.

## Décision

1. Un plugin est de la première main quand il porte un identifiant `dev.escouade.*` **et** vient de l'amorce ou d'une version du store signée (dans une build de développement, aussi des modules du dépôt et de l'amorce non signée). Un dossier en développement, un dossier copié ou un dépôt cloné sont lus comme des tiers, quel que soit leur identifiant : `dev.escouade.*` leur est refusé. Une version vérifiée du store d'un autre auteur est lue comme un tiers. 2. Une seule fonction lit les exclusions du proxy (`tls::proxy_for`), et elle décide de la route de chaque client des réglages et de la vérification du certificat. Elle comprend `*`, les noms (avec `.`, `*.`, un port, un point final), les adresses (IPv6 entre crochets ou non, avec un port) et les plages d'adresses.

## Conséquences

Un auteur de plugin de la première main ne peut plus charger son dossier en développement sous `dev.escouade.*` dans une release : il passe par une build de développement (origine `workspace`) ou par le banc (`--first-party`). Une entrée déjà installée de cette façon ne se charge plus. Les exclusions écrites en plage d'adresses, en joker ou avec un port sont maintenant respectées pour la route aussi : un hôte de `10.0.0.0/8` est joint directement, son certificat vérifié.
