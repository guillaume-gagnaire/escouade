---
number: 131
id: "les-menus-harness-n-offrent-que-les-harn"
title: "Les menus « Harness » n'offrent que les harness utilisables"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-391"
pages: ["harness/harness"]
tickets: ["ESC-391"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/harness","added":17,"removed":8}]
accepted_at: "2026-11-09T07:18:24+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Sur la 2.2.0-beta.2, « Harness par défaut » proposait tous les harness dont le programme est trouvé sur le poste, « même ceux pas installés ». En 2.2 un harness passe par son plugin, et l'amorce n'installe que les plugins de ce qui est configuré (ESC-373) : un harness pouvait être offert avec son plugin coupé ou absent, ou sans aucun compte, et ses agents ne démarraient pas.

## Décision

Un harness est utilisable quand son programme est trouvé, que son plugin est installé et actif, et qu'il a au moins un compte dans Escouade. Les menus « Harness » (harness par défaut du projet et de l'assistant de nouveau projet, Nouvel agent, ticket, nouvelle discussion, modèle des textes rédigés qui sert aussi aux relectures de PR) n'offrent que ceux-là. Le harness déjà choisi qui ne l'est plus reste listé avec sa raison : plugin coupé, aucun compte, programme introuvable. Réglages › Comptes et harness et « Ajouter un compte » montrent toujours chaque harness trouvé, pour qu'on puisse le configurer. Le cœur suit la même règle pour choisir qui rédige les textes, et la passation ne va que vers un harness au plugin actif.

## Conséquences

Le cœur donne à la fenêtre les harness dont le plugin est coupé ou absent (enginesOff, engines_off_now), relus à l'ouverture des menus et après un changement de plugin. Un harness sans compte n'est plus proposé dans les menus : on l'ajoute d'abord dans Comptes et harness. Le menu d'un projet ou d'un ticket se montre aussi quand le harness choisi n'est plus utilisable, même s'il ne reste qu'un autre harness.
