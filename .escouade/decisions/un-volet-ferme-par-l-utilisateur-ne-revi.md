---
number: null
id: "un-volet-ferme-par-l-utilisateur-ne-revi"
title: "Un volet fermé par l'utilisateur ne revient pas avant son retour ; « Laisser piloter par… » prête un navigateur de l'utilisateur"
date: "2026-11-27"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-684"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-684"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":17,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

AB-9 (docs/AGENT-BROWSERS.md § 5.5) : un agent n'ouvre un volet que si l'utilisateur regarde sa conversation, un seul volet par agent, et si l'utilisateur ferme le volet l'agent ne le rouvre pas avant son retour, « et l'outil le dit ». Il restait à choisir ce que répond open_browser dans ce cas, ce qui compte comme une fermeture, et comment l'utilisateur donne un navigateur qu'il a ouvert seul.

## Décision

Fermer le volet (ou la ligne) d'un navigateur ou d'une page qu'un agent pilote pendant que sa conversation reste à l'écran retient l'agent : rien de lui ne s'ouvre à l'écran, ni n'attend, tant que l'utilisateur n'a pas quitté sa conversation puis n'y est pas revenu. open_browser répond alors une nouvelle place, `closed`, avec `why`, sans rien ouvrir ni demander ni compter ; `quiet` reste permis. Les déplacements de la fenêtre (un volet par agent), la fermeture par l'agent et le retour depuis un navigateur montré seul ne comptent pas. La fenêtre vérifie encore, au moment d'ouvrir, que l'utilisateur regarde la conversation (sinon la ligne attend), et laisse la conversation comme volet actif. « Laisser piloter par… », dans le menu d'un navigateur que l'utilisateur a ouvert seul, en fait piloter un agent actif du même projet (ou personne), dans la borne de 3, avec un état de pilotage neuf ; l'agent est prévenu par un message.

## Conséquences

La sortie d'open_browser gagne `closed` (docs/AGENT-BROWSERS.md § 3.2 listait waiting|quiet ; la page de spec le dit). Deux commandes de la fenêtre : devpage_dismiss et devpage_lend. Rien ne pousse l'écran de l'utilisateur ailleurs que la conversation qu'il regarde.
