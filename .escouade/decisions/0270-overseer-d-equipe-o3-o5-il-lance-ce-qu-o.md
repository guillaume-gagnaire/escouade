---
number: 270
id: "overseer-d-equipe-o3-o5-il-lance-ce-qu-o"
title: "Overseer d'équipe (O3, O5) : il lance ce qu'on confie à la machine, propose aux membres, et ne valide que sous consigne d'un administrateur"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-588"
pages: ["serveur-equipe/overseer-equipe"]
tickets: ["ESC-580","ESC-588","ESC-613","ESC-615","ESC-616"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-21T20:49:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-22 (ESC-580, docs/TEAM-OVERSEER.md §5). C11-A borne le pilote auto aux tickets qu'on lui confie, pour que le quota de chacun ne serve que son travail choisi ; un ticket d'équipe se valide sur le poste qui le tient. Un Overseer qui assignerait ou validerait seul engagerait les membres et la branche cible. Choix O3 et O5, tranchés par l'utilisateur le 2026-11-21 (demande a-23) : « Je prends toutes les recommandations : A1-C, A2-A, A3-B, A4-A, A5-B, A6-A, A7-A, A8-A ; O1 à O8 en A ; C16 : a. » Reprise par ESC-588.

## Décision

O3-A et O5-A, les recommandations. Seul : il lance ce qui est confié à la machine (assigné à elle, ou « À faire » là où l'équipe le permet), prend un agent qu'on lui passe, répond à ses agents, relit leur travail, crée et découpe des tickets. En proposant, dans la boîte « À toi » du membre : lui confier un ticket, relire son travail, reprendre sur la machine un ticket interrompu. Sous une consigne durable d'un administrateur, avec sa portée : valider, fusionner, pousser ou publier, seulement les tickets que la machine tient ; jamais valider, renvoyer, déplacer ni arrêter ce qu'un membre tient ; jamais signer un réglage, un membre, un coupe-circuit ou un plugin privé. board.on_approve: pr, imposable, en fait toujours une PR. Écartés : O3-B (assigner directement, le pilote auto du membre dépensant son quota), O3-C (B pour qui le coche, plus tard si demandé), O5-B (sous la consigne de tout membre), O5-C (jamais).

## Conséquences

Compris dans OE-3 et OE-4 : la garde des consignes est liée à l'auteur administrateur. Une clé overseer.mayApprove (false = O5-C) dans le fichier d'équipe (OE-5).
