---
number: 315
id: "un-ticket-qu-un-collegue-ecrit-et-m-assi"
title: "Un ticket qu'un collègue écrit et m'assigne attend mon accord avant tout lancement"
date: "2026-11-25"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-645"
pages: ["serveur-equipe/kanban-equipe","kanban/pilote-auto","harness/boite-a-toi"]
tickets: ["ESC-645","ESC-626"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/kanban-equipe","added":13,"removed":0},{"page":"kanban/pilote-auto","added":2,"removed":0},{"page":"harness/boite-a-toi","added":5,"removed":0}]
accepted_at: "2026-11-25T22:16:00+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

SR3-02 (haute) de la revue de sécurité de la 2.3 (ESC-626) : l'assignation d'un ticket d'équipe est un champ partagé que n'importe quel membre écrit (son client modifié met l'id de membre d'un collègue). Chez ce collègue, le ticket devenait « assigné à moi » et son pilote auto le lançait aussitôt : le titre et la description d'un tiers devenaient le premier message d'un agent du poste, avec ses droits, ses comptes et son mode de permissions, sans qu'il les ait vus. La preuve du rapport (deux postes contre le vrai serveur) le montrait.

## Décision

Un ticket de l'équipe assigné au membre de ce poste, dont le titre, la description ou les critères ont été écrits en dernier par un autre membre, est « à accepter » : ni le pilote auto, ni l'Overseer, ni `start_ticket` d'un agent, ni « Lancer », ni « Reprendre », ni la prise du serveur ne le lancent avant « Accepter » (sa carte, ou sa vue « Ticket à accepter » dans « À toi » ; jamais un outil MCP ni le chef d'escouade). Le membre d'une écriture se lit par l'appareil qui l'a signée et le registre de l'équipe, jamais par un champ. L'accord porte sur les écritures montrées (refusé si le texte a changé entre-temps) ; une nouvelle écriture d'un autre membre après l'accord remet le ticket « à accepter », une écriture de soi non. La carte dit « Texte de Bob Durand » et « Assigné à moi par Bob Durand » ; `get_ticket` dit `team.writtenBy`, `team.assignedBy`, `team.toAccept`. « Prendre aussi « À faire » de l'équipe » reste un accord donné d'avance pour les tickets non assignés.

## Conséquences

Un geste de plus pour un ticket confié par un collègue ; un ticket qu'on a écrit soi-même ne demande rien. L'état d'accord est de ce poste (gardé avec le ticket), pas partagé. Reste de confiance : un collègue peut encore changer les critères d'un ticket accepté pendant que son agent travaille (leurs notes passent dans les messages de boucle) ; « Prendre aussi « À faire » de l'équipe » coché, un ticket non assigné d'un collègue part sans accord, comme ce réglage le dit.
