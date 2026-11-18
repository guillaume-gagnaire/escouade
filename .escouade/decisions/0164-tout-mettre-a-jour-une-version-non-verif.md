---
number: 164
id: "tout-mettre-a-jour-une-version-non-verif"
title: "« Tout mettre à jour » : une version non vérifiée a sa propre fenêtre"
date: "2026-11-13"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-438"
pages: ["store/store-dans-l-app"]
tickets: ["ESC-438"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"store/store-dans-l-app","added":5,"removed":0}]
accepted_at: "2026-11-14T16:58:40+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-436 a groupé dans une fenêtre commune les mises à jour qui ne demandent ni permission ni contribution nouvelle. Le tri ne regardait pas le sceau : une version non vérifiée (perte du sceau comprise) ou la mise à jour d'un plugin d'un dépôt ou d'un dossier s'y confirmait d'un geste, sans l'avertissement de sa fenêtre seule, sous un texte qui rassure (audit 2026-11, SEC-29, essai B-16).

## Décision

Une version non vérifiée compte parmi ce qui « demande plus » : elle a sa propre fenêtre, celle d'une mise à jour seule, avec son avertissement. Cela couvre la perte du sceau et tout plugin venu d'un dépôt ou d'un dossier. La fenêtre commune ne garde que des versions vérifiées qui n'ajoutent rien, et le dit (« Toutes ces versions sont vérifiées par Escouade… »).

## Conséquences

Un plugin tiers non relu ne s'accepte plus jamais en groupe : un clic de plus par version non vérifiée. Les plugins de première main, toujours signés, restent groupés. L'étiquette « Non vérifié » disparaît de la fenêtre commune, qui n'en contient plus.
