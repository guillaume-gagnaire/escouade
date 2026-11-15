---
number: 169
id: "les-reglages-imposes-six-rangs-une-table"
title: "Les réglages imposés : six rangs, une table de clés, et le cœur qui refuse"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-455"
pages: ["fenetre/reglages-imposes"]
tickets: ["ESC-455"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages-imposes","added":49,"removed":0}]
accepted_at: "2026-11-15T02:22:50+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le serveur d'équipe (docs/TEAM-SERVER.md §6) prévoit des réglages imposés par une équipe ou par le poste géré. L'utilisateur veut que les stratégies du poste partent tôt (C17) : elles servent sans serveur, à une entreprise qui gère sa flotte. ESC-454 a fait la lecture des stratégies (crates/escouade-policy) sans la brancher. TS-11 (ESC-455) devait faire le moteur des couches et le verrou.

## Décision

- Six rangs (défauts, équipe recommandé, dépôt, poste, équipe imposé, poste géré) ; les rangs 2 à 4 gardent l'ordre d'avant, 5 et 6 passent devant tout. Une borne se croise entre l'équipe et le poste géré ; une valeur du poste géré gagne. Une clé exécutable n'est jamais imposée par l'équipe ; une clé sensible de l'équipe attend l'accord du poste ; une clé qui restreint ne s'impose que dans son sens (éteint, stable).
- Une table unique dans le cœur (`locks::KEYS`) : `permissions.allowBypass`, `permissions.modes` (borne), `permissions.defaultMode`, `harnesses.allowed` (borne), `telemetry.enabled`, `updates.channel`, `remoteControl.enabled`, chacune avec sa sorte et les lignes du registre des réglages qu'elle verrouille. Les noms de `PROVISIONAL_KEYS` d'escouade-policy suivent cette table (`permissions.modes`, `harnesses.allowed`, `remoteControl.enabled`).
- Le rang 6 est lu au démarrage seulement ; le rang 5 est injectable et vide dans l'app.
- Le cœur refuse l'écriture de ce qui est imposé, par la fenêtre, le MCP, le `.escouade.yml` du dépôt et l'export ; l'export ne fait qu'omettre `board.mode` et `agents.engine` quand une borne les touche sur ce poste.
- La fenêtre grise un réglage imposé (cadenas, « Imposé par la stratégie de ce poste » ou « par l'équipe X ») et les choix interdits d'une borne avec la raison. Les avertissements de lecture vont au journal et dans Réglages › Général › « Stratégies de ce poste ».
- Au démarrage, les réglages, les projets et les agents sont ramenés dans les verrous ; un agent dans un mode devenu interdit passe, à la fin de son tour, au mode permis le plus proche qui ne permet jamais plus, et le dit.
- La source de l'e2e (`ESCOUADE_TEST_POLICIES`) n'existe que dans un build de développement, vérifié par un test (SEC-13).

## Conséquences

TS-12 complète la table, en génère l'ADMX, le profil MDM et le schéma, et ajoute la relecture à chaud. TS-13 donne au rang 5 sa source (le fichier d'équipe signé), son espace et l'accord des clés sensibles. Un harness devenu interdit n'arrête pas encore ses agents entre deux tours : ils ne redémarrent simplement plus.
