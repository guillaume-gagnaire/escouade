---
number: 363
id: "reseau-prive-rp-4-l-entree-des-connecteu"
title: "Réseau privé, RP-4 : l'entrée des connecteurs, son appairage, ses bornes et la stratégie"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-600"
pages: ["a-distance/atteindre-un-reseau-prive","a-distance/mode-serveur","fenetre/reglages-imposes"]
tickets: ["ESC-600","ESC-599","ESC-654"]
documents: ["docs/PRIVATE-NETWORK.md"]
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/atteindre-un-reseau-prive","added":62,"removed":10},{"page":"a-distance/mode-serveur","added":15,"removed":0},{"page":"fenetre/reglages-imposes","added":7,"removed":0}]
accepted_at: "2026-11-28T08:09:13+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

RP-4 (ESC-600) construit dans escouade-server l'entrée des connecteurs que R3 a décidée : un port à part, éteint par défaut, qui ne parle que leur protocole. L'étude (§ 4.5) disait « un code de 8 lettres, 10 minutes, une fois, 5 essais faux au plus » ; depuis, ESC-654 a retiré aux appareils la règle « 5 codes faux finissent le code », qui laissait n'importe qui bloquer l'appairage, et la spec dit que celui d'un connecteur suit celui d'un appareil. L'entrée est faite pour être sur Internet. La crate du protocole (RP-2) n'avait pas d'échange d'appairage : RP-3 (ESC-599) et RP-4 l'ont accordé entre eux. Restaient à fixer les bornes, ce qui est gardé où, les options, et ce que fait la stratégie servers.privateNetwork quand elle arrive pendant que le serveur tourne. Question posée le 2026-11-27 : l'utilisateur (par l'overseer) a choisi « Les deux » (5 essais par source sans jamais finir le code, et 20 par minute sur toute l'entrée), en demandant de dire la durée et la taille du code et ce qu'elles bornent.

## Décision

1. L'entrée : `--connector-listen <adresse:port>` (`ESCOUADE_SERVER_CONNECTOR_LISTEN`), une adresse et un port toujours nommés ; elle ouvre la porte avec elle. Elle ne sert que `/connector` (un canal, Noise IK) et `/connector/pair` (l'appairage) en WebSocket ; tout autre chemin est un 404, toute autre parole fermée. TLS par `--connector-cert` et `--connector-key` (ensemble), sinon derrière un proxy TLS ; son adresse par `--connector-url`, sinon devinée de l'adresse d'écoute et dite au journal. `--trust-proxy` vaut pour elle.
2. L'appairage : le module `pairing` de la crate du protocole (fait avec RP-3) : Noise XXpsk0 sur `/connector/pair`, la clé prépartagée tirée du code par BLAKE2s, le code ne passant jamais. Le code : 8 lettres d'un alphabet de 32 (40 bits), 10 minutes, une fois, un code par nom (un nouveau remplace l'ancien), un nom déjà appairé refusé ; une version refusée ne consomme pas le code.
3. Les bornes (« Les deux ») : un code faux ne finit jamais le code ; par source, 20 essais ratés par minute ferment ses connexions jusqu'à la fin de la minute, et 5 codes faux pendant la vie d'un code (10 min) lui refusent l'appairage pour le reste ; pour toute l'entrée, 20 essais ratés par minute ferment l'appairage à tous jusqu'à la fin de la minute, un connecteur appairé entrant toujours. Au plus 200 codes essayés pendant la vie d'un code sur 2⁴⁰ : environ une chance sur 5,5 milliards ; la durée et la taille du code restent celles de l'étude.
4. Ce qui est gardé : dans `secrets.json`, la clé privée du serveur pour les connecteurs, la clé publique de chaque connecteur, les codes en attente (leur clé prépartagée, jamais le code) ; dans `connectors.json` (0600), la liste (nom, dates, version, version refusée, plafond, connecté depuis), la trace des 32 derniers retirés (empreinte de la clé), l'adresse de l'entrée ; jamais une clé ni un code au journal.
5. Les commandes : `escouade-server connector pair <nom>`, `connectors [list]`, `connectors remove <nom ou id>` (le serveur qui tourne ferme le canal dans les deux secondes) ; pour l'espace, par l'API distante et à un poste seulement : `private_network_status`, `_pair`, `_remove`, `_project`, `_set_project` (la liste d'un projet dans le plafond connu de son connecteur), `_journal` ; aucun outil MCP.
6. La stratégie `servers.privateNetwork` à `false` : au démarrage, ni entrée ni porte ; l'appairage refusé partout, avec qui l'impose ; arrivée pendant que le serveur tourne, l'entrée fermée et tous les canaux coupés, la porte des agents déjà lancés gardée sans plus rien atteindre d'un réseau privé (lui couper son proxy couperait Internet aux agents) ; retirée, l'entrée se rouvre.

## Conséquences

L'entrée peut être exposée sur Internet sans que l'appairage soit devinable ni bloquable par un inconnu sur une source ; derrière un proxy non déclaré, toutes les connexions comptent pour une source, et 5 codes faux d'un inconnu y refusent l'appairage aux autres pour 10 minutes : déclarer le proxy (`--trust-proxy`). Un attaquant réparti peut fermer l'appairage à tous minute par minute, jamais les canaux des connecteurs appairés. RP-3 prend le même module d'appairage (commit 62b3eb42c, cherry-pické tel quel). RP-5 construit l'interface sur les commandes servies. Une ligne de plus dans les stratégies (ADMX, ADML, profils macOS, schéma de /etc, page du site).
