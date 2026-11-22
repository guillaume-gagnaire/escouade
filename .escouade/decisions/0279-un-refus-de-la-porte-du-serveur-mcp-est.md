---
number: 279
id: "un-refus-de-la-porte-du-serveur-mcp-est"
title: "Un refus de la porte du serveur MCP est une erreur JSON-RPC, et l'agent arrêté au passage en privé montre « Activer » aussitôt"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-619"
pages: ["harness/serveur-mcp","projets/projets-prives"]
tickets: ["ESC-619","ESC-582","ESC-404"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/serveur-mcp","added":5,"removed":0},{"page":"projets/projets-prives","added":4,"removed":0}]
accepted_at: "2026-11-22T03:06:02+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La CI de release/2.3 (run 44936350320, Windows) tombait sur tests/e2e/mcp.spec.ts:343 : après le passage de « coffre » en privé, l'agent de « coffre » appelait list_projects et le serveur répondait « Requête refusée : jeton inconnu » en texte nu, que le client ne pouvait pas lire comme du JSON. Dans un projet privé, chaque harness est coupé d'office tant que l'utilisateur ne l'y active pas (décision 142, ESC-404), et un process qui tourne quand son projet devient privé est jugé comme un démarrage (décision 259, ESC-582) : l'arrêt de l'agent est voulu, son jeton est oublié avec son process. Le test décrivait donc un comportement retiré à dessein. Restait que la porte du serveur (Gate) répondait tout refus en text/plain, alors qu'un client MCP attend du JSON-RPC.

## Décision

1) Le produit garde la règle des décisions 142 et 259 : le test de mcp.spec.ts active Claude Code pour « coffre » une fois privé (« Activer » sur la carte de l'agent, l'accord, puis un message qui relance l'agent avec un nouveau jeton), et un second test montre l'agent arrêté, sa conversation qui le dit, sa carte « Activer », et son ancien jeton refusé. 2) Tout refus de la porte du serveur MCP (Host, Origin, jeton, chemin, outil inconnu, taille, corps illisible) garde son statut HTTP et répond une erreur JSON-RPC en application/json : jsonrpc 2.0, l'id de la requête quand son corps a été lu (nombre, ou chaîne de 128 octets au plus ; sinon null), error.code (-32602 pour un outil inconnu, -32600 pour une requête trop volumineuse ou illisible, -32001 pour les autres) et error.message (« Requête refusée : … »). 3) Réglages › Confidentialité relit, dès que « Projet privé » est enregistré, quels harness du projet attendent « Activer » (harness_scope) : la carte de l'agent arrêté au passage en privé est là aussitôt, et part quand le projet redevient non privé. Avant, la fenêtre ne la relisait qu'à un autre geste, et l'agent arrêté n'offrait pas « Activer ».

## Conséquences

Un client MCP lit la raison d'un refus comme la réponse qu'il attend, au lieu d'échouer à lire du texte. Les statuts, WWW-Authenticate et l'activité MCP ne changent pas ; seul le corps change de forme. Un id de requête court est répété dans la réponse, rien d'autre de ce que le client a envoyé. L'utilisateur qui rend un projet privé voit tout de suite comment relancer son agent ; l'entrée non publiée d'ESC-582 au CHANGELOG le dit.
