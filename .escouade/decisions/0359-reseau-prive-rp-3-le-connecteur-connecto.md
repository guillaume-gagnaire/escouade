---
number: 359
id: "reseau-prive-rp-3-le-connecteur-connecto"
title: "Réseau privé RP-3 : le connecteur, connector.toml, l'appairage et les contrôles"
date: "2026-11-27"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-599"
pages: ["a-distance/atteindre-un-reseau-prive"]
tickets: ["ESC-599","ESC-600"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/atteindre-un-reseau-prive","added":55,"removed":4}]
accepted_at: "2026-11-28T02:10:51+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

RP-3 (ESC-599) construit le programme escouade-connector d'après docs/PRIVATE-NETWORK.md § 4 et les décisions R1 à R8. L'étude laissait ouverts : le format exact de connector.toml, l'échange d'appairage (la crate du protocole de RP-2 n'en avait pas, et RP-4 le construit en parallèle côté serveur), où et comment garder la clé privée, ce que fait le connecteur quand une seule des adresses résolues est interdite, qui applique les bornes, ce que deviennent les flux ouverts quand le plafond rétrécit, et comment il réessaie quand le serveur ne le connaît plus.

## Décision

1. connector.toml : les clés server (facultative, pair la donne), proxy (http:// et Basic ; sinon HTTPS_PROXY et NO_PROXY), authorities, allow (le plafond : hôte:port, *.suffixe:port, une adresse écrite telle quelle), ranges (facultatif), auto_update (lu seulement), [limits] max_streams (1 à 4 096, 256), opens_per_second (1 à 1 024, 32), idle_minutes (1 à 1 440, 10). Une clé inconnue est une erreur. Relu toutes les 2 s ; un fichier illisible garde le dernier lu ; un flux que le nouveau plafond ne permet plus est fermé ; un autre serveur, proxy ou autorités rouvre le canal ; le silence vaut à la prochaine connexion.
2. L'appairage, dans la crate du protocole (module pairing, partagé avec RP-4) : au chemin de l'entrée suivi de /pair, Noise_XXpsk0_25519_ChaChaPoly_BLAKE2s dont la clé prépartagée est BLAKE2s-256(« escouade-connector-pair/1 » ‖ code mis en forme) ; trois messages de XX puis un quatrième chiffré (appairé, ou refusé avec la raison) ; un code faux ferme sans un mot ; une version refusée ne consomme pas le code. Le code ne passe jamais, même sous une inspection TLS.
3. Les clés : connector.key (0600 dans un dossier 0700 sous Unix ; scellée par DPAPI pour le compte qui a lancé pair sous Windows) et paired.toml (l'adresse, les deux clés publiques), dans --state, $STATE_DIRECTORY ou le dossier du système. unpair écrit des zéros sur la clé avant de l'effacer.
4. Les adresses : parmi plusieurs adresses résolues, les interdites sont écartées (et dites) et les autres jointes ; refusé s'il n'en reste aucune. Interdites : boucle, lien local, métadonnées connues (169.254.169.254, 100.100.100.200, fd00:ec2::254), non précisée, diffusion, multicast, la machine du connecteur (reconnue par une socket qui peut s'y lier, sans écouter), hors des plages ; IPv4 dans IPv6 et NAT64 lues comme leur IPv4. La boucle n'est permise que par une option des tests, jamais par connector.toml ni la ligne de commande.
5. Les bornes sont appliquées et journalisées par le connecteur (relues à chaud) ; le canal du protocole garde des limites dures au-dessus (4 096 flux, 1 024 par seconde).
6. La reconnexion suit le calendrier du protocole ; un serveur qui ne connaît plus le connecteur ou refuse sa version est réessayé toutes les 60 s, le journal disant quoi faire.
7. Le journal : une ligne JSON par événement (allowed, closed, refused et la vie du service), jamais le contenu, le code ni une clé ; en dernier garde, une ligne qui contiendrait la clé privée est remplacée par held-back.
8. La version du connecteur est celle de l'app (scripts/version.mjs) ; ses textes sont en anglais, comme ceux du protocole.

## Conséquences

RP-4 prend le module pairing tel quel (commit 62b3eb42c) et sert /connector/pair à côté de /connector ; il ne garde du code que sa clé prépartagée. RP-5 montre les plages du plafond reçu (des lignes sans port) sans s'en servir pour décider. RP-6 doit lancer pair sous le compte du service Windows (DPAPI) et faire la mise à jour. Une faute dans connector.toml arrête le démarrage, et à chaud n'élargit ni ne retire rien. Le connecteur fait un bind sans écoute par adresse résolue pour reconnaître sa propre machine.
