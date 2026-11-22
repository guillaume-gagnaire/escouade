---
number: 285
id: "qui-revoque-et-qui-appaire-l-auteur-pris"
title: "Qui révoque et qui appaire : l'auteur pris de la connexion, la cascade des appareils, le code donné à la fenêtre seule, « Retirer » un téléphone révoque son jeton"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-630"
pages: ["a-distance/api-distante","a-distance/les-espaces","a-distance/tunnel-du-telephone"]
tickets: ["ESC-630","ESC-624"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"a-distance/api-distante","added":6,"removed":0},{"page":"a-distance/les-espaces","added":3,"removed":3},{"page":"a-distance/tunnel-du-telephone","added":2,"removed":0}]
accepted_at: "2026-11-22T15:12:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-624, constats SR1-02, SR1-03, SR1-10) montre qu'un appareil appairé pouvait demander lui-même un code d'appairage, faire naître un appareil qui survivait à sa propre révocation, et signer une révocation du nom de son choix (l'argument `by` de `remote_device_revoke`). « Retirer » un téléphone dans Réglages › Téléphone retirait son pair WireGuard mais pas son jeton, qui passait encore par Tailscale ou la boucle locale. Et le désappairage par le tunnel ne vérifiait pas, comme la poignée de main, que le jeton était celui de l'appareil lié au pair.

## Décision

1. L'auteur d'une révocation vient de la connexion : l'appareil de la connexion qui la demande (son nom), ou « la fenêtre d'Escouade » ; `remote_device_revoke` ne prend plus que l'id. 2. Chaque appareil garde `pairedBy` (l'appareil qui a demandé son code ; rien pour la fenêtre ou `escouade-server pair`) ; révoquer un appareil révoque ceux qui sont nés de lui, en cascade, et l'écran Appareils le dit avant de confirmer ; un code demandé par un appareil révoqué depuis n'appaire plus rien. 3. `remote_pair_code` n'est donné qu'à la fenêtre du cœur, refusé à un client distant avec sa raison, jusqu'aux appareils « poste » de MO-4 ; l'écran Appareils (toujours sur un serveur, donc distant) n'offre plus « Créer un code » et dit de lancer `escouade-server pair`. 4. `phone_remove` (et `escouade-server phone remove`) révoque l'appareil lié au pair ; un pair jamais lié est dit tel ; l'écran dit « … retiré et son accès révoqué ». 5. `unpair` fait le contrôle de liaison de la poignée de main.

## Conséquences

Un jeton qui fuit ne fait plus naître d'appareil, ne signe plus une révocation d'un autre nom, et un téléphone retiré n'a plus d'accès par aucune voie. En contrepartie, un client de bureau sur un serveur ne crée plus de code d'appairage depuis Réglages de l'espace › Appareils : il faut `escouade-server pair` sur le serveur, jusqu'aux portées de MO-4. Le fichier des appareils gagne un champ `pairedBy` (lu sans lui par les versions d'avant) ; celui du code qui attend, un champ `by`.
