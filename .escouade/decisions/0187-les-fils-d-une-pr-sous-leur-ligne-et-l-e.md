---
number: 187
id: "les-fils-d-une-pr-sous-leur-ligne-et-l-e"
title: "Les fils d'une PR sous leur ligne, et l'écriture chez l'hébergeur au clic seulement"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-474"
pages: ["git-fichiers/pull-requests","git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-474"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":25,"removed":6},{"page":"git-fichiers/hebergeurs-de-code","added":7,"removed":0}]
accepted_at: "2026-11-16T11:20:30+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PR-4 des PR dans Escouade (design/pr-2-3, direction A). La vue d'une PR (décisions 171, 177) lisait les fils sans pouvoir y répondre, et l'onglet « PR et CI » disait « En lecture : Escouade ne publie rien en ton nom » (décision du 2026-10-16). Les plugins d'hébergeur savaient déjà commenter, répondre et (sauf GitHub) résoudre, sans capacité déclarée ni chemin de la fenêtre, et aucun ne savait commenter une ligne seule ni rouvrir.

## Décision

Les fils de l'hébergeur se posent sous leur ligne dans l'onglet Fichiers (PR entière), les résolus repliés avec leur compte, les dépassés ou dont la ligne n'est plus dans le diff à part en tête de leur fichier ; « Suivante » (N, P) va d'un fil non résolu à l'autre. La vue écrit chez l'hébergeur, sous le compte de l'utilisateur et à son clic seulement : commenter une ligne ou une plage (sur le commit lu), commenter la PR, répondre, résoudre, rouvrir. Une capacité Writing déclare ce que chaque hébergeur écrit (GitHub tout ; GitLab, Gitea/Forgejo, Bitbucket et Azure DevOps commenter, répondre, résoudre, sans essai réel avant PR-9) ; le protocole gagne forge/lineComment et forge/unresolve, resolve prend l'id du fil. Le cœur n'écrit que par la commande pr_write de la fenêtre, jamais par le serveur MCP ni pour un agent. Les brouillons sont gardés en mémoire par PR et par cible le temps de la session ; les refus (jeton, droit d'écrire, fil supprimé, limite, hors ligne) sont dits dans le fil, le brouillon gardé.

## Conséquences

La phrase « Escouade ne publie rien en ton nom » disparaît de la vue (« Rien ne part sur GitHub sans ton clic ») et devient, dans l'onglet « PR et CI », « En lecture ici : réponds et résous dans la vue de la PR ». Le texte d'un fil reste rendu assaini et balisé comme donnée d'un tiers pour les agents. Pas d'édition ni de suppression de commentaire, pas de suggestion ni de réaction ; un brouillon ne survit pas au redémarrage. Les autres hébergeurs n'offrent pour l'instant que « Commenter la PR » puisqu'ils ne lisent pas encore les fils (PR-9).
