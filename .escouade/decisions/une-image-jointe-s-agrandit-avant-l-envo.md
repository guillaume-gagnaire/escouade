---
number: null
id: "une-image-jointe-s-agrandit-avant-l-envo"
title: "Une image jointe s'agrandit avant l'envoi, dans la visionneuse de la conversation"
date: "2026-11-20"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-555"
pages: ["conversation/conversation"]
tickets: ["ESC-555"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"conversation/conversation","added":8,"removed":1}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Demande de l'utilisateur du 2026-11-20 : « Quand dans le prompt on met une image, ça serait cool de pouvoir cliquer dessus pour l'agrandir. » La vignette d'une image jointe au champ de saisie (et au formulaire d'un ticket) ne réagissait pas au clic ; la visionneuse d'ESC-171 ne servait qu'aux images déjà dans la conversation et à la boîte « À toi ».

## Décision

La vignette d'une image jointe et son nom deviennent un bouton « Agrandir <nom> » qui ouvre la même visionneuse (ImageViewer, aucune seconde) sur elle, les autres images jointes du message en précédente et suivante dans l'ordre du champ. Le « × » retire sans ouvrir ; fermer rend le focus à la vignette. Elle montre l'URL data: que le champ tient déjà, sans source : rien n'est relu sur le disque, « Ouvrir » et « Afficher dans le dossier » sont cachés (rien à ouvrir avant l'envoi), Copier et Enregistrer sous restent. Pareil dans le formulaire d'un ticket ; la réponse de la boîte « À toi » (puces sans vignette) et le champ d'un sous-agent (texte seul) n'ont rien à agrandir.

## Conséquences

Une puce d'image partagée (ImageChip) remplace les deux copies du champ et du formulaire du ticket ; son infobulle dit « Agrandir <nom> » au lieu du nom seul. Si la boîte « À toi » montre un jour des vignettes dans sa réponse, elle se branche de même.
