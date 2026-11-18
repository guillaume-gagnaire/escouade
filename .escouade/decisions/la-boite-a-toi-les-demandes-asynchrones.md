---
number: null
id: "la-boite-a-toi-les-demandes-asynchrones"
title: "La boîte « À toi » : les demandes asynchrones des agents, par le serveur MCP, avec leurs pièces jointes"
date: "2026-11-17"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-509"
pages: ["harness/boite-a-toi","harness/serveur-mcp"]
tickets: ["ESC-509","ESC-508"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/boite-a-toi","added":50,"removed":0},{"page":"harness/serveur-mcp","added":8,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'utilisateur veut voir en un endroit ce que ses agents attendent de lui sans s'arrêter (un choix entre des rendus de design, un feu vert, un essai à faire), pièces jointes comprises (images, PDF). Aujourd'hui, cela se perd dans les bilans et les messages. La conception (docs/INBOX.md, ESC-508) a été tranchée le 2026-11-17 : direction B « La file », sortie dans la ligne 2.2, la boîte reposant sur le serveur MCP.

## Décision

Une demande asynchrone (choix de 2 à 6 options, oui ou non, réponse libre, tâche) est déposée par un agent avec ask_user, gardée dans l'état durable avec son histoire et bornée (10 ouvertes par agent, 40 par projet, 200 en tout, 20 dépôts par heure). Ses fichiers, sur la demande ou sur une option, sont typés par leur contenu (une prétendue image qui n'en est pas une est refusée), bornés (12, 25 Mo chacun, 64 Mo par demande, 1 Go pour la boîte) et copiés dans <données>/asks/, où ils survivent au worktree et restent 30 jours après la fin de la demande. La réponse de l'utilisateur part 10 s après son geste (le temps de la reprendre) en message à l'agent, marqué comme venant de lui, qui le réveille au repos ; elle ne déclenche rien d'autre. Aucun outil ne répond à la place de l'utilisateur, sauf answer_ask pour l'agent à qui l'utilisateur a laissé une demande, une à une, selon le réglage élargi « Les agents peuvent répondre aux demandes des autres agents » ; jamais un feu vert, une tâche ni sa propre demande. Un ticket peut attendre une demande (le pilote auto le laisse, il se bloque si elle est retirée ou périmée). Serveur MCP éteint, les outils n'existent pas, la boîte est fermée et les demandes restent gardées, figées. Les textes et les fichiers d'une demande sont des données d'agent, marquées comme données de tiers pour un autre agent ; un projet privé garde ses demandes pour lui ; app.log n'en garde que l'identifiant et la forme.

## Conséquences

Huit outils MCP (ask_user, update_ask, withdraw_ask, list_asks, get_ask, wait_ask, get_ask_file, answer_ask) et douze commandes de la fenêtre (inbox_*) que l'écran (IN-2) utilisera. wait_ask attend 240 s au plus, comme wait_ticket, et non 600 s comme l'écrivait la conception. Le libellé du réglage des réponses change. Au reversement dans release/2.3, les commandes inbox_* sont à classer dans la liste de l'API distante (remoteapi/list.rs, docs/REMOTE.md) et les réglages imposés (locks.rs) doivent pouvoir porter la boîte ; l'Overseer (délégation, nouvelles) et les vues d'Escouade viennent avec IN-3.
