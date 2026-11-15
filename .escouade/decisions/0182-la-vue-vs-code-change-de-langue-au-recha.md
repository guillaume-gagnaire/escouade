---
number: 182
id: "la-vue-vs-code-change-de-langue-au-recha"
title: "La vue VS Code change de langue au rechargement de la fenêtre"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-472"
pages: ["editeur/editeur"]
tickets: ["ESC-472"]
documents: ["docs/EDITOR-STUDY.md"]
supersedes: []
superseded_by: null
changes: [{"page":"editeur/editeur","added":19,"removed":10}]
accepted_at: "2026-11-15T19:43:13+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La vue VS Code (ESC-447) doit parler la langue d'Escouade (ESC-472, A4). VS Code lit ses textes quand ses modules se chargent (titres de commandes, menus, réglages) : le pack de langue de monaco-vscode-api doit être là avant, et le plan de travail ne se démonte ni ne se recharge seul dans la page. Changer ses textes à chaud donnerait une interface à moitié traduite. Il faut donc trancher ce que fait la vue quand la langue d'Escouade change alors qu'elle est déjà chargée.

## Décision

La vue se charge dans la langue d'Escouade du moment, une fois par fenêtre (français : le pack de langue de monaco-vscode-api ; anglais : VS Code tel quel). Quand la langue change ensuite, la vue ne se recharge pas d'elle-même : sa barre dit « VS Code change de langue au rechargement de la fenêtre. » avec « Recharger ». « Recharger » demande d'abord pour les fichiers modifiés dans la vue (Enregistrer, Ne pas enregistrer, Annuler, comme toute fermeture), puis recharge la fenêtre, qui rouvre l'éditeur du même projet avec ses onglets, celui qui était montré au premier plan. Le thème, la couleur du projet, le thème d'un plugin, les polices et la taille du texte, eux, se suivent à chaud, sans recharger.

## Conséquences

Rien de la vue ne se perd en changeant de langue : ni une modification (demandée avant), ni les onglets du projet montré (rouverts) ; ceux d'un autre projet ne sont pas rouverts. Celui qui change de langue dans les réglages n'est pas sorti de sa page par un rechargement qu'il n'a pas demandé. Le pack français pèse 1,9 Mo (0,35 Mo en brotli), chargé seulement à la première ouverture de la vue en français.
