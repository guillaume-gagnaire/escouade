---
title: "L'Overseer d'équipe"
subject: "serveur-equipe"
sources: ["docs/TEAM-OVERSEER.md (ESC-580, TS-22 : l'étude, les choix O1 à O8, le découpage OE-1 à OE-6 ; §7.1 pour le bail)","Réponse de l'utilisateur du 2026-11-21 (demande a-23) : toutes les recommandations, O1 à O8 en A","docs/OVERSEER.md","docs/TEAM-SERVER.md §4.2, §9, §10, §11, §12, §13, C11, C15","spec : L'Overseer ; Le Kanban d'équipe ; Les coûts et les quotas de l'équipe ; Passer un agent à un collègue","crates/escouade-team-server/README.md (OE-2 : table team_leases, routes /v1/team/leases/{nom})","crates/escouade-team/README.md (OE-2 : LeaseKeeper)"]
tickets: ["ESC-580","ESC-588","ESC-606","ESC-611","ESC-613","ESC-615","ESC-616","ESC-618","ESC-546"]
decisions: [269,275,270,271,272,273,342,376]
compared: null
---

# L'Overseer d'équipe

> **Décidé, en partie construit.** L'utilisateur a pris toutes les recommandations de l'étude d'ESC-580 (`docs/TEAM-OVERSEER.md`) le 2026-11-21 (O1 à O8 en A). Cette page dit ce qui est décidé ; chaque partie se construit par un ticket, de OE-1 (ESC-606) à OE-6 (ESC-618). **OE-1 est fait** (ESC-606) : le rôle de service au registre ; **OE-2 est fait** (ESC-611) : le bail d'équipe (sections ci-dessous). L'Overseer de chacun (page « L'Overseer ») ne change pas.

## Ce que c'est

Un Overseer qui suit les projets d'une équipe, jour et nuit : il lance et suit des agents, relit, rend compte, et propose aux membres ce qui les touche.

## Qui le fait tourner (O1-A, O2-A, O6-A)

- Une **machine de l'équipe en mode serveur** (`escouade-server team overseer`, OE-3, ESC-613), sous un **rôle de service** (OE-1, ESC-606, fait) : un appareil de l'équipe, pas d'un membre, qui ne signe rien d'administratif (ni membre, ni rôle, ni réglage, ni coupe-circuit, ni plugin privé) et ne répond pas aux invitations.
- Sur les **comptes de harness de la machine** (un siège d'équipe, une clé d'API), jamais le compte d'un membre ; son modèle se règle sur la machine (Haiku par défaut).
- **Un seul actif par équipe** : il tient un **bail d'équipe** au serveur (OE-2, ESC-611, fait : section « Le bail d'équipe »), 10 min renouvelées toutes les 3 min, sous un jeton qui croît et que portent ses écritures ; une autre machine attend et le dit.

## Le rôle de service (OE-1, fait)

- **Au registre** : un rôle de plus à côté d'administrateur et de membre, `Service`. La machine a **son propre id** au registre, rattaché à l'équipe et à aucune personne ; elle n'est ni membre ni administrateur, et ne compte pas parmi les administrateurs.
- **Son inscription**, par un administrateur seul : Réglages › Avancé › « Ouvrir les équipes… », carte de l'équipe, « Machines de l'équipe » › **« Inscrire une machine de l'équipe »** affiche un code court (une heure, un essai). Sur la machine en mode serveur, `escouade-server team pair <serveur> <CODE> [--name <nom>]` le tape (ou « Ajouter cet appareil par un code » dans la fenêtre connectée à la machine) ; la commande affiche les six mots de sa demande, que l'administrateur compare avant « Admettre », comme pour toute entrée. Le code va de l'administrateur à la machine, comme pour « Ajouter un appareil » : le relais ne laisse ouvrir un créneau qu'à un appareil de l'équipe.
- **Ce qu'elle signe** : ses propres appareils, le contenu des projets qu'on lui ouvre (Kanban, docs, mémoire, en proposition comme tout agent), ses prises de tickets.
- **Ce qu'elle ne signe jamais** : un membre, un rôle, un réglage de l'équipe, la liste des projets, la clé des coûts, un coupe-circuit, un plugin privé (ni proposition ni retrait), une proposition de rattachement ; elle ne répond à aucune invitation. Chaque poste le vérifie à la lecture, quoi qu'ait laissé passer son client et quoi que dise le serveur (`escouade-team-crypto`, `policy.rs` : `may_sign`).
- **Le rôle ne change jamais** : ni une personne faite machine, ni une machine faite personne ; on retire la machine et on en inscrit une autre. Le propriétaire de l'équipe est une personne.
- **Le serveur** ne voit du rôle que l'id opaque de l'appareil et sa sorte, « service » : il la refuse sur ses routes d'administrateur (jetons, rôles, révocation d'un autre, alertes) et sur les flux propres de l'équipe (réglages, liste des projets, propositions, clé des coûts, store privé, coupe-circuit) ; il lui laisse celui des totaux des coûts. Un administrateur ne lui donne pas d'autre sorte.
- **Réglages › Équipe** la range à part, sous **« Machines de l'équipe »** : son nom court, ses appareils, ce qu'elle peut et ne peut pas, et pour un administrateur **« Retirer »**, qui la coupe comme un appareil (coupée au serveur, sortie du groupe, les clés de l'équipe tournent). Le compte des membres ne compte que les personnes. « Passer à un collègue » la propose parmi « Machines de l'équipe ».
- **Le MCP** : aucun outil ne donne ni ne retire le rôle ; « Retirer » est un geste de la fenêtre du poste, jamais à distance.
- **La machine en mode serveur** tient ses équipes : `escouade-server` ouvre les équipes comme le poste, et les sert aux fenêtres distantes.

## Le bail d'équipe (OE-2, fait)

- **Au serveur**, `/v1/team/leases/{nom}` : prendre (`POST`), renouveler (`…/renew`), rendre (`…/release`), lire (`GET` : qui le tient, depuis quand, jusqu'à quand). Le nom est libre (`overseer` aujourd'hui pour l'Overseer d'équipe ; un nom par projet plus tard, O6-B, par les mêmes routes) : 1 à 64 caractères parmi `a-z`, `0-9`, `.`, `_`, `-`.
- **Réservé aux machines de l'équipe** (rôle de service, OE-1) : l'appareil d'une personne, administrateur compris, est refusé (403) sur chaque route, la lecture comprise.
- **Un seul détenteur** : la prise se fait en une transaction ; une autre machine reçoit « pris » (`taken`) et le bail en détail (l'appareil qui le tient, depuis quand, jusqu'à quand). **Bail de 10 min**, renouvelé toutes les 3 min par la machine qui le tient ; un détenteur révoqué ne tient plus rien. **Le jeton croît à chaque prise**, même quand la même machine le reprend (un redémarrage retire l'ancien jeton).
- **Le jeton sur les écritures** : une écriture qui se dit de l'Overseer d'équipe (une entrée du fil, une proposition, un ticket créé : elles arrivent avec OE-4) porte le nom et le jeton du bail, au flux de l'équipe comme à celui d'un projet. Le serveur la refuse (`stale_token`, comme les prises de tickets) quand le jeton n'est plus le courant, n'est pas celui de l'appareil qui écrit, **ou que le bail est fini** : une machine revenue d'une coupure ne parle plus au nom de l'Overseer, et même restée seule, elle renouvelle avant de reparler. Rien n'est écrit d'une écriture refusée.
- **La machine** (`escouade-team`, `LeaseKeeper`) prend le bail au départ, le renouvelle en tâche de fond, le rend à l'arrêt (l'autre le prend aussitôt). Elle le **perd proprement** : un renouvellement refusé, un `stale_token`, ou le bail fini à sa propre horloge sans renouvellement entendu du serveur ; dès lors elle n'a plus de jeton à donner et n'écrit plus rien au nom de l'Overseer.
- **La machine qui attend le dit** : « En attente : l'Overseer d'équipe tourne sur la machine build-2 » (le nom que la machine a écrit dans les projets de l'équipe, chiffré ; « sur une autre machine de l'équipe » s'il n'est pas connu). Elle relit le bail et **le prend à son expiration**, sous un jeton plus grand. OE-3 et OE-4 montrent cet état ; il n'a pas d'écran avant eux.
- **Ce que voit le serveur** : un nom, un appareil, des heures (prise, dernier renouvellement, fin) et le jeton ; jamais le nom de la machine, ce que fait l'Overseer, ni un projet. Le bail n'est pas encore dans l'audit du serveur (AU-2).

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
| OE-1 (ESC-606), **fait** | le rôle de service au registre (partagé avec l'audit) | ESC-588 |
| OE-2 (ESC-611), **fait** | le bail d'équipe | OE-1 |
| OE-3 (ESC-613) | l'Overseer d'équipe sur la machine | OE-2 |
| OE-4 (ESC-615) | le fil de l'équipe et son écran | OE-3 |
| OE-5 (ESC-616) | les bornes et le coupe-circuit | OE-3, ESC-546 |
| OE-6 (ESC-618) | la cohabitation, la reprise du bail, l'e2e | OE-4, OE-5 |

## Ce qui attend l'utilisateur

L'essai sur une vraie machine de l'équipe (le VPS d'essai), avec un compte de harness qu'il connecte lui-même à la machine, et deux membres qui lui parlent ; `escouade-server team pair` sur ce VPS.
