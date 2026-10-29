---
title: "Escouade"
subject: "Introduction"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
order: 1
---

## Ce que c'est

Escouade est une application desktop pour piloter plusieurs agents de code en local, organisés par projet, sur le harness de son choix (voir *Harness*). Elle s'appelait « Claude Code Manager ».

## Comportement

### L'ancien nom

Au lancement, ce qui existe encore sous l'ancien nom est déplacé :

- le dossier `~/.claude-code-manager/` devient `~/.escouade/` ;
- les dossiers de l'identifiant `dev.gagnaire.claude-code-manager` (WebView, taille de fenêtre) passent à `dev.gagnaire.escouade` ;
- les préférences `ccm.*` deviennent `escouade.*`.

La variable `CCM_DATA_DIR` reste acceptée à côté d'`ESCOUADE_DATA_DIR`. Les nouveaux agents travaillent sur des branches `escouade/` ; les agents d'avant gardent leurs branches `ccm/`. La mise à jour réapplique le nouvel identifiant aux raccourcis de l'app, dont Windows se sert pour les notifications.

### Référence visuelle

`design/Claude Code Manager.dc.html` est la source de vérité du look & feel.
