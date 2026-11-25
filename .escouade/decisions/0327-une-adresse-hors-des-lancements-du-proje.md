---
number: 327
id: "une-adresse-hors-des-lancements-du-proje"
title: "Une adresse hors des lancements du projet est demandée à l'utilisateur, une fois par origine"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-663"
pages: ["kanban/navigateurs-pilotes"]
tickets: ["ESC-663"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/navigateurs-pilotes","added":2,"removed":2}]
accepted_at: "2026-11-26T00:48:42+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-663 : un agent qui pilote un navigateur ouvre de lui-même trois sortes d'adresses : les origines des lancements du projet et de ses worktrees, `localhost` aux ports déclarés, ses pages d'agent. Pour une autre adresse `http(s)` (une doc, un service tiers), la question a été posée à l'utilisateur (a-39) : la demander, ou la refuser.

## Décision

Le choix A de l'utilisateur (2026-11-25) : demandée, une fois par origine.

`open_browser` ou `drive_browser` (`navigate`) vers une autre origine n'ouvre rien. Une bande demande « <agent> veut ouvrir <hôte> · Permettre une fois / Refuser », et l'outil répond `asked` ; l'accord ou le refus revient à l'agent par un message. L'accord vaut pour cette origine dans ce navigateur, jusqu'à sa fermeture. Jamais, même avec l'accord : les adresses d'Escouade, `file:`, `data:`, `javascript:`, `blob:` ni aucun autre schéma. Les liens que la page suit gardent la règle de `devpage::judge` (une autre origine part au navigateur du système).

## Conséquences

Un agent peut montrer une doc ou un service à côté de son travail, et l'utilisateur garde le dernier mot à chaque nouvelle origine. Les artefacts que l'agent a publiés, dont l'adresse est prise dans le résultat de son outil `Artifact`, s'ouvrent sans demander. AB-6 (`open_browser`, `asked`, le message de réponse) et AB-8 (la bande) partent sur ce choix. L'e2e AB-12 vérifie qu'une autre origine est demandée.
