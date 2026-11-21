---
title: "L'Overseer d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-OVERSEER.md (ESC-580, TS-22 : l'étude, les choix O1 à O8, le découpage OE-1 à OE-6)","Réponse de l'utilisateur du 2026-11-21 (demande a-23) : toutes les recommandations, O1 à O8 en A","docs/OVERSEER.md","docs/TEAM-SERVER.md §4.2, §9, §10, §11, §12, §13, C11, C15","spec : L'Overseer ; Le Kanban d'équipe ; Les coûts et les quotas de l'équipe ; Passer un agent à un collègue"]
tickets: ["ESC-580","ESC-588","ESC-606","ESC-611","ESC-613","ESC-615","ESC-616","ESC-618","ESC-546"]
decisions: [269,275,270,271,272,273]
compared: null
---

# L'Overseer d'équipe

> **Décidé, pas encore construit.** L'utilisateur a pris toutes les recommandations de l'étude d'ESC-580 (`docs/TEAM-OVERSEER.md`) le 2026-11-21 (O1 à O8 en A). Cette page dit ce qui est décidé ; chaque partie se construit par un ticket, de OE-1 (ESC-606) à OE-6 (ESC-618). Une partie construite le dira ici. L'Overseer de chacun (page « L'Overseer ») ne change pas.

## Ce que c'est

Un Overseer qui suit les projets d'une équipe, jour et nuit : il lance et suit des agents, relit, rend compte, et propose aux membres ce qui les touche.

## Qui le fait tourner (O1-A, O2-A, O6-A)

- Une **machine de l'équipe en mode serveur** (`escouade-server team overseer`, OE-3, ESC-613), sous un **rôle de service** (OE-1, ESC-606) : un appareil de l'équipe, pas d'un membre, qui ne signe rien d'administratif (ni membre, ni rôle, ni réglage, ni coupe-circuit, ni plugin privé) et ne répond pas aux invitations.
- Sur les **comptes de harness de la machine** (un siège d'équipe, une clé d'API), jamais le compte d'un membre ; son modèle se règle sur la machine (Haiku par défaut).
- **Un seul actif par équipe** : il tient un **bail d'équipe** au serveur (OE-2, ESC-611), 10 min renouvelées toutes les 3 min, sous un jeton qui croît et que portent ses écritures ; une autre machine attend et le dit.

## Ce qu'il voit

Les projets de l'équipe dont la machine a la clé, leur Kanban (prises, assignations, passages), leurs docs et la mémoire d'équipe, ses propres agents, le fil de l'équipe, le résumé des coûts et les budgets. Jamais les projets personnels ni privés, les autres équipes, ni ce que fait un agent sur un poste.

## Ce qu'il décide seul, ce qu'il propose (O3-A, O5-A)

- **Seul** : lancer ce qui est confié à la machine (assigné à elle, ou « À faire » là où l'équipe le permet), prendre un agent qu'on lui passe, répondre à ses agents, relire leur travail, créer et découper des tickets.
- **En proposant**, dans la boîte « À toi » du membre : lui confier un ticket, relire son travail, reprendre un ticket interrompu sur la machine.
- **Sous une consigne d'un administrateur** : valider, fusionner, pousser ou publier, seulement pour les tickets que la machine tient. Jamais un ticket qu'un membre tient. `board.on_approve: pr`, imposé, en fait toujours une PR.

## Qui lui parle (O4-A)

Tout membre, dans un **fil commun** de l'équipe (OE-4, ESC-615), chiffré, chaque message signé par l'appareil de qui l'écrit ; ses réponses et ses actions y sont aussi, avec « Annuler ». Un écran « Overseer d'équipe » dans l'espace de l'équipe, et le téléphone par l'API distante. Seul un administrateur donne une consigne durable, avec la garde d'aujourd'hui (les mots cités, les négations) ; administrateur au moment du message et de l'action. Un texte lu (ticket, document, souvenir) ne commande jamais.

## Deux Overseers sans conflit (OE-6, ESC-618)

- Le bail d'équipe : un seul Overseer d'équipe ; la machine qui reprend le bail relit le fil et ne relance rien qui tourne ailleurs.
- La prise atomique : un ticket, un seul preneur, quel que soit qui le lance.
- L'Overseer personnel d'un membre agit au nom de son membre, voit l'Overseer d'équipe dans le fil, et ne lui parle pas ; aucun Overseer ne reçoit d'ordres d'un autre.

## Les bornes (O7-A, O8-A ; OE-5, ESC-616)

- Ses lancements comptent comme le pilote auto : les budgets de l'équipe et `stopAutopilot` les arrêtent ; ses coûts sont ceux de « machine de l'équipe » et « Overseer d'équipe », jamais d'un membre. L'équipe lui impose des clés `overseer.*` (enabled, projects, maxAgents à 3 par défaut, budget, model, engines, mayApprove).
- Le coupe-circuit (ESC-546) l'arrête avec ses agents, et gagne une portée « Overseer d'équipe » ; il ne peut ni le lancer ni le lever. Sans contact avec le serveur depuis 1 h, la machine ne lance plus rien. `escouade-server team overseer stop` l'arrête en local. Levé, il ne relance rien seul.

## Les tickets

| Ticket | Ce qu'il construit | Après |
| --- | --- | --- |
| OE-1 (ESC-606) | le rôle de service au registre (partagé avec l'audit) | ESC-588 |
| OE-2 (ESC-611) | le bail d'équipe | OE-1 |
| OE-3 (ESC-613) | l'Overseer d'équipe sur la machine | OE-2 |
| OE-4 (ESC-615) | le fil de l'équipe et son écran | OE-3 |
| OE-5 (ESC-616) | les bornes et le coupe-circuit | OE-3, ESC-546 |
| OE-6 (ESC-618) | la cohabitation, la reprise du bail, l'e2e | OE-4, OE-5 |

## Ce qui attend l'utilisateur

L'essai sur une vraie machine de l'équipe (le VPS d'essai), avec un compte de harness qu'il connecte lui-même à la machine, et deux membres qui lui parlent.
