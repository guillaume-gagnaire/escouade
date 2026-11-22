---
number: 284
id: "fichiers-recus-d-un-serveur-une-liste-de"
title: "Fichiers reçus d'un serveur : une liste de ce qui s'ouvre"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-629"
pages: ["a-distance/ce-qui-change-de-machine"]
tickets: ["ESC-629","ESC-624"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/ce-qui-change-de-machine","added":15,"removed":7}]
accepted_at: "2026-11-22T13:13:41+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-624, constat SR1-06) a montré qu'un fichier reçu d'un serveur s'ouvrait d'un clic dès qu'il n'était pas sur une liste noire de programmes et de scripts : une console, une image disque, un raccourci ou un lien d'un autre système passait, et sous macOS la copie n'était pas mise en quarantaine. Un serveur, ou un agent égaré par le contenu d'un dépôt, peut envoyer n'importe quel fichier.

## Décision

Le poste n'ouvre un fichier reçu d'un serveur que s'il est sur une liste blanche, le nom et les octets disant le même type : images PNG, JPEG, GIF, WebP et PDF, ouverts par le système ; texte brut, Markdown, CSV et JSON, ouverts dans l'éditeur d'Escouade (jamais confiés au système). Tout le reste n'a que « Télécharger sur ce poste », montré dans son dossier, et la fenêtre dit pourquoi. La copie est marquée comme téléchargée : Zone.Identifier sous Windows, com.apple.quarantine sous macOS, jamais exécutable sous macOS et Linux.

## Conséquences

Un document bureautique, une archive ou une page web reçus d'un serveur ne s'ouvrent plus d'un clic : l'utilisateur les ouvre lui-même depuis leur dossier, Gatekeeper ou SmartScreen avertissant alors. La liste noire d'ESC-628 reste en plus pour le poste. Un type à ajouter à la liste passe par une nouvelle décision.
