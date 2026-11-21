---
number: 224
id: "le-sdk-et-le-kit-refusent-un-appel-du-co"
title: "Le SDK et le kit refusent un appel du contrat de la 2.2.5 sur une Escouade qui ne le sert pas, et escouade.min est dit en un seul endroit"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-531"
pages: ["plugins/profils-et-reglages-d-un-projet","plugins/contrat-du-manifeste"]
tickets: ["ESC-531"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/profils-et-reglages-d-un-projet","added":6,"removed":2},{"page":"plugins/contrat-du-manifeste","added":18,"removed":0}]
accepted_at: "2026-11-20T07:29:51+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-531 (T9 de l'étude ESC-501) livre aux auteurs ce que les tickets ESC-524 à ESC-530 ont servi dans l'app : profils, réglages d'un projet, jokers, limite de programmes. Une app plus ancienne ignorerait le paramètre profile de secrets/get et rendrait un autre secret, ou garderait 4 programmes ; la version qui sert le contrat devait aussi être dite à plusieurs endroits (protocole, SDK, deux lecteurs du manifeste, documentation) sans pouvoir dériver.

## Décision

Le kit Rust (0.1.4) et le SDK (0.5.0) vérifient capabilities.profiles et capabilities.typedCommands reçus à initialize avant chaque appel qui en dépend, et refusent (-32601) en disant la version qui les sert, sans rien envoyer. La version (2.2.5) a une source, x-escouadeMin de schemas/plugin-protocol-1.json, que scripts/escouade-min.mjs écrit partout et qu'un test tient. validate avertit, sans refuser, un manifeste qui prend le contrat avec un escouade.min plus ancien, et ses messages disent le champ fautif et pourquoi. Le banc de conformité gagne quatre scénarios (profils, secret de plusieurs lignes, jokers, limite) et attend qu'un plugin se taise après un fait (3 s au plus).

## Conséquences

Un plugin du contrat ne reçoit jamais en silence un autre secret d'une app ancienne. Le chef fixe la version à la release par une commande. À publier dans l'ordre : escouade-plugin-protocol 0.1.4, escouade-plugin-kit 0.1.4, @escouade-dev/plugin 0.5.0, create-escouade-plugin 0.6.0. Le banc est un peu plus long pour un plugin qui travaille sur un fait.
