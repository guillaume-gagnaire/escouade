---
number: 345
id: "reseau-prive-rp-2-la-forme-du-protocole"
title: "Réseau privé, RP-2 : la forme du protocole"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-598"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-598","ESC-597","ESC-599","ESC-600"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/atteindre-un-reseau-prive","added":14,"removed":3}]
accepted_at: "2026-11-26T20:23:44+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

R2 (`reseau-prive-r2-le-connecteur-parle-par`) a choisi une WebSocket sur TLS, Noise IK épinglé à l'intérieur, des flux à contrôle de débit et des versions « la sienne et les deux mineures d'avant », sans fixer le détail. RP-2 (ESC-598) construit la crate `escouade-connector-proto` que partageront escouade-server et escouade-connector : il fallait choisir le format des trames, où passent les versions, ce que veut dire fermer un flux, les tailles de fenêtre et de trame, et ce qu'on fait d'un connecteur plus récent que le serveur, qu'aucun texte ne disait.

## Décision

- **Noise** : `Noise_IK_25519_ChaChaPoly_BLAKE2s`, prologue `escouade-connector-proto/1` ; le connecteur est l'initiateur (il connaît la clé du serveur), un message WebSocket binaire par message Noise. Sans clé connue, le serveur ferme sans répondre ni fermer la WebSocket ; `AcceptError::is_failed_attempt` dit à RP-4 ce qui compte comme essai raté (pas de poignée de main en 10 s, pas ce protocole, clé inconnue), pas une version refusée.
- **Les versions** voyagent dans les deux messages de la poignée de main : le connecteur dit `major.minor.patch` (3 × u16), le serveur répond sa version, son verdict (à jour, à mettre à jour, trop ancien, trop récent) et la plus ancienne qu'il accepte. Ces champs restent en tête pour toujours, et ce qui suit est ignoré : un serveur de n'importe quelle version sait répondre à un connecteur de n'importe quelle autre. Mineure du serveur ou celle d'avant : à jour ; deux d'avant : acceptée, à mettre à jour ; plus vieille ou autre majeur : refusée ; **plus récente que le serveur : refusée** (c'est le serveur à mettre à jour).
- **Les trames** : un format binaire à nous, un octet de type puis les champs en gros-boutiste, chaînes préfixées par leur longueur, décodage strict (une seule écriture par trame) ; un type inconnu est ignoré (un pair d'une mineure plus récente). `open` 0x01, `opened` 0x02, `refused` 0x03 (un code et une raison lisible), `data` 0x04, `close` 0x05, `window` 0x06, `ping` 0x07, `pong` 0x08, `ceiling` 0x09 (le plafond, à chaque connexion et à chaque relecture). Seul le serveur ouvre, seul le connecteur accepte, refuse ou envoie son plafond ; une trame hors de son rôle coupe le canal.
- **Fermer** : une `data` vide clôt un sens (le FIN de TCP, pour `git` et `ssh` qui ferment leur écriture avant de lire la réponse) ; `close` met fin au flux dans les deux sens (abandon, flux muet, flux lâché).
- **Le débit** : 256 Kio de fenêtre par flux et par sens, rendus par quarts à mesure de la lecture ; des trames `data` de 16 Kio ; une trame de 64 Kio au plus (le plus que tient un message Noise), 32 Kio au moins si l'appelant la baisse. Le canal écrit par une tâche à part, pour ne jamais attendre le réseau pendant qu'il lit.
- **Les bornes** (réglables) : 256 flux, 32 ouvertures demandées par seconde, refusées comprises, un flux muet fermé après 10 min ; ping 15 s, coupure à 45 s de silence ; reconnexion 250 ms, 500 ms, 1 s, 2 s, 4 s, puis 5 s.

## Conséquences

RP-1, RP-3 et RP-4 bâtissent sur cette crate sans rien redéfinir : `transport::connect` et `connect_server` pour le connecteur, `tls_acceptor`, `accept_websocket` et `accept_connector` pour l'entrée du serveur, un `Stream` qui est un `AsyncRead + AsyncWrite`. Changer une trame existante, c'est changer le protocole : on ajoute un type, on ne change pas le sens d'un ancien. Refuser un connecteur plus récent oblige l'entreprise à attendre que le serveur suive ; l'autre choix (l'accepter) aurait demandé qu'un connecteur sache parler les versions d'avant, ce que rien ne garantissait. Une trame de 64 Kio et une fenêtre de 256 Kio par flux bornent la mémoire d'un canal à 64 Mio au pire avec 256 flux pleins. Ce que l'utilisateur voit ne change pas encore (pas de ligne de CHANGELOG) ; la page « Atteindre un réseau privé » a une section « Le protocole ».
