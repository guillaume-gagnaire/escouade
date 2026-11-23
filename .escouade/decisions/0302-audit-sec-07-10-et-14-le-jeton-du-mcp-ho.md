---
number: 302
id: "audit-sec-07-10-et-14-le-jeton-du-mcp-ho"
title: "Audit SEC-07, 10 et 14 : le jeton du MCP hors des lignes de commande, les secrets au repos, les mises à jour jamais hors du proxy"
date: "2026-11-22"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-595"
pages: ["fenetre/reseau","harness/serveur-mcp","introduction/donnees-et-coupures"]
tickets: ["ESC-595"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reseau","added":23,"removed":4},{"page":"harness/serveur-mcp","added":14,"removed":7},{"page":"introduction/donnees-et-coupures","added":10,"removed":3}]
accepted_at: "2026-11-23T20:47:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'audit de sécurité de la 2.2.0 (docs/security/audit-2026-11/rapport.md) relève trois constats faibles : SEC-07, Claude Code recevait la déclaration du serveur MCP par `claude mcp add --header "Authorization: Bearer <jeton>"`, le jeton lisible dans la liste des process ; SEC-10, pas de trousseau sous Linux, dossier de données lisible selon l'umask sous Unix, mot de passe du proxy en clair dans settings.json ; SEC-14, un proxy injoignable était contourné une fois par les mises à jour. L'utilisateur a accepté le 2026-11-21 (a-7) d'appliquer les recommandations telles quelles, avec pour SEC-14 un réglage allumé d'office quand un proxy est configuré.

## Décision

1. SEC-07 : Escouade écrit elle-même l'entrée `mcpServers.escouade` du `.claude.json` de chaque compte Claude Code (JSON strict, le reste du fichier gardé), comme pour Codex, OpenCode et Copilot CLI ; plus aucun programme n'est lancé pour déclarer, et la déclaration à la main est l'entrée à coller. L'entrée de l'utilisateur garde le jeton écrit (fichier lisible du seul propriétaire) : elle sert un harness lancé hors d'Escouade, où aucune variable d'Escouade n'est posée ; les variables d'environnement restent ce qu'Escouade emploie pour ce qu'elle lance (hooks de la vue Claude Code, terminal Codex). La spec dit ce que le format de chaque harness permet.
2. SEC-10 : le dossier de données est créé en 0700 sous Unix et remis en 0700 à chaque démarrage ; sous Linux, le service de secrets du bureau (Secret Service par D-Bus, keyring avec zbus en Rust) garde les secrets quand il répond au démarrage (3 s au plus), le repli d'avant sinon ; le mot de passe du proxy va au trousseau (entrée `proxy`), jamais dans settings.json ni dans la fenêtre, migré au démarrage, gardé à part dans settings.json (`proxyPasswordInFile`) sans trousseau.
3. SEC-14 : « Ne jamais sortir sans le proxy » (`proxyNeverDirect`, vide = allumé dès qu'un proxy est configuré) ; allumé, une mise à jour dont le proxy ne répond pas échoue avec sa raison, sans tentative directe, et un agent ne démarre pas quand le mot de passe du proxy gardé au trousseau (`proxyPasswordKept`) n'y est plus au démarrage. Clé simple des réglages, à ajouter à la liste des politiques de la 2.3 (`proxy.neverDirect`, Restrict) au reversement.

## Conséquences

Le jeton n'est plus l'argument d'aucun process ; une dépendance de plus sous Linux (secret-service, zbus) ; un réglage de plus dans Réseau ; un poste derrière un proxy en panne ne reçoit plus de mise à jour tant que l'utilisateur n'éteint pas le réglage. Sur la ligne 2.2, le réglage n'est pas encore imposable par un poste géré : sa politique vient avec la 2.3.
