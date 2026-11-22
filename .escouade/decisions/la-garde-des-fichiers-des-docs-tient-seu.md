---
number: null
id: "la-garde-des-fichiers-des-docs-tient-seu"
title: "La garde des fichiers des docs tient seule, avant tout accès au disque"
date: "2026-11-22"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-643"
pages: ["docs/docs-dans-le-depot"]
tickets: []
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: ""
accepted_by: ""
---

## Contexte

La revue de sécurité de la 2.3 a montré que la garde des fichiers de la spec, des décisions et des documents comptait sur ses appelants : un segment absolu remplaçait tout le chemin, `..` remontait, et un dossier parent absent sautait la comparaison au dossier avant que l'écriture ne crée les dossiers. Sur main, les appelants validaient déjà le chemin ; en 2.3, un nouvel appelant ne le faisait pas.

## Décision

La garde refuse d'elle-même, avant tout accès au disque, tout segment vide, `.`, `..`, absolu, de lecteur, UNC ou de périphérique, contenant `\`, `:` ou un caractère de contrôle, finissant par un point ou une espace, ou nom réservé de Windows ; le refus dit « chemin hors du dossier » et sa raison. Le chemin est ensuite vérifié lexicalement sous le dossier, et sur le disque pour la partie qui existe ; l'écriture refait ce contrôle après avoir créé les dossiers.

## Conséquences

Défense en profondeur : un futur appelant qui oublierait de valider ne peut plus écrire hors de `.escouade`. Un fichier d'un dépôt cloné au nom refusé (deux-points, nom réservé, point final) n'est plus lu ni écrit par les docs, ce qui est voulu.
