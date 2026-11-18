---
number: null
id: "la-2-2-3-reversee-dans-la-2-3-le-fichier"
title: "La 2.2.3 reversée dans la 2.3 : le fichier des secrets imposable, la création d'un agent par un agent jugée, la boîte servie à distance"
date: "2026-11-17"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-520"
pages: ["fenetre/reglages-imposes","harness/boite-a-toi","plugins/vue-d-ensemble"]
tickets: ["ESC-520"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages-imposes","added":9,"removed":0},{"page":"harness/boite-a-toi","added":7,"removed":0},{"page":"plugins/vue-d-ensemble","added":2,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

La 2.2.3 (PR 44, ESC-491, ESC-499, ESC-500, ESC-511) et le cœur de la boîte « À toi » (ESC-509) ont été écrits sur main, sans les réglages imposés, l'API distante ni le cœur sans Tauri de la 2.3. En les reversant, il faut décider comment les stratégies du poste et de l'équipe les jugent, et ce qu'un client distant peut faire de la boîte.

## Décision

1. Une clé de stratégie, `secrets.fileFallback`, de sorte « restreindre » (seul false s'impose), dans la catégorie Plugins et valable pour toute l'app : interdite, le fichier des secrets des plugins n'est ni proposé ni écrit, et celui déjà choisi n'est plus lu (ses secrets restent à déplacer vers le trousseau ou à effacer). La fenêtre montre le cadenas au champ et sur le groupe de Réglages › Plugins.
2. `create_agent` juge, avant de rien créer, le harness du compte choisi, le modèle et l'effort qu'un agent demande pour celui qu'il crée. Un effort demandé au-dessus du plafond est refusé avec qui l'impose, au lieu d'être ramené en silence ; un effort laissé à son compte descend au plafond.
3. Les commandes de la boîte sont servies à distance, et `inbox_file_bytes` donne un fichier en octets bruts (trames binaires). `inbox_open_file`, qui ouvre sur la machine du cœur, est refusée à distance jusqu'au mode serveur, comme `sent_file_open`.
4. Le pilote ACP passe par le jugement du mode d'ESC-497 : la session neuve par `judge_start`, et un tour que le harness lance seul est jugé à sa fin.

## Conséquences

Un poste géré ou une équipe peut interdire de garder des secrets en clair sur le disque. Un agent ne peut pas donner à celui qu'il crée plus que ce que les stratégies permettent. Le téléphone a ce qu'il faut pour lire et répondre à la boîte. Le serveur d'équipe, l'Overseer, les relectures de PR et l'écran du téléphone restent à brancher sur la boîte (docs/INBOX.md).
