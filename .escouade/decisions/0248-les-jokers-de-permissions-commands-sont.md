---
number: 248
id: "les-jokers-de-permissions-commands-sont"
title: "Les jokers de permissions.commands sont servis : un mot entier d'une forme fermée, vérifié par l'hôte"
date: "2026-11-19"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-528"
pages: ["plugins/contrat-du-manifeste","plugins/tous-les-plugins-en-wasm"]
tickets: ["ESC-528"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/contrat-du-manifeste","added":34,"removed":6},{"page":"plugins/tous-les-plugins-en-wasm","added":3,"removed":0}]
accepted_at: "2026-11-21T11:12:27+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

L'issue #42 (un auteur de plugin tiers, un tunnel OpenVPN par projet) demandait des arguments variables dans permissions.commands ; le seul moyen était un `*` final, qui admet n'importe quoi après la tête (`openvpn *` vaut `--up /bin/sh --script-security 2`). L'étude ESC-501 (§5) a proposé des jokers typés, ESC-525 les a lus au manifeste en coupant tout plugin qui en avait, et l'utilisateur a tranché le 2026-11-17 : cette liste de jokers et rien d'autre, pas de chemin dans le projet ni dans le dossier de l'agent. Restait à les servir à l'exécution sans ouvrir plus que ce que l'utilisateur lit à l'installation.

## Décision

- `procs::allowed` compare la commande demandée à chaque entrée selon le §5.3 de l'étude : un mot littéral octet pour octet ; `{int}`, `{port}`, `{name}`, `{pid}`, `{path:temp}`, `{path:plugin}` admettent chacun un seul mot de leur forme fermée, jamais `--` ni un mot qui commence par `-` ; l'entrée garde sa longueur, sauf un `*` final, inchangé. Une accolade inconnue ou un joker dans un mot ne correspond à rien.
- `{name}` et les parties d'un chemin refusent les noms de périphérique de Windows sous tous les systèmes, pour qu'un manifeste dise la même chose partout ; `{path:plugin}` refuse en plus une partie cachée, comme `files/read`.
- `{pid}` : seul le pid d'un programme vivant lancé par cette instance ; `proc/spawn` rend `{ proc, pid }`, le pid est retiré dès que l'hôte voit la fin, et vérifié de nouveau sous le verrou du lancement.
- `{path:temp}` et `{path:plugin}` : l'hôte joint le chemin relatif à la racine, le résout comme `fs/*`, le garde s'il reste dessous et nomme un fichier, et passe au programme le chemin absolu résolu.
- `initialize` annonce `capabilities.typedCommands` ; `escouade-plugin-protocol` 0.1.4 le lit (`greeting().typed_commands`), `escouade-plugin-kit` 0.1.3 le demande.
- Les jokers sortent de la liste de ce qui coupe un plugin (`Manifest::unserved`).
- La fenêtre d'installation et l'onglet Permissions montrent chaque joker en pastille de mots, puis une fois la phrase de garde, en français et en anglais. Chaque refus est dit au journal, avec le mot et la raison ; chaque lancement avec ses mots réels.

## Conséquences

- Un plugin comme celui de l'issue #42 se passe de `*` : chaque variable est un mot vérifié, lisible à l'installation et à la revue.
- Une app qui ne sert pas les jokers (2.2.4 et avant) compare un joker comme un mot littéral : l'entrée ne correspond à rien de ce qu'un plugin demande, jamais plus. Le plugin dit `escouade.min`.
- La course entre la vérification d'un chemin et sa lecture par le programme est bornée, pas fermée : le chemin passé est déjà résolu, et pour le détourner il faudrait remplacer une de ses parties par un lien entre les deux ; le module n'en crée aucun, seul un programme accordé par l'utilisateur le pourrait.
- La réutilisation d'un pid est bornée par ce que l'hôte tient : sous Windows le handle du programme (le système ne redonne pas un pid tenu), sous Unix son zombie tant qu'il n'est pas récolté ; il ne reste que l'instant entre la récolte et le retrait de la liste, sur lequel Linux devrait avoir fait le tour de ses pids.
- Trouvé en route : un module de voix (groq) tombait par manque de mémoire sur une dictée de 10 minutes dès que la poignée de main grandissait de quelques octets (`turn::write` doublait une ligne de 25 Mo, `turn::lines` en faisait trois copies). `escouade-plugin-protocol` 0.1.4 lit et écrit une grosse ligne en une copie : la dictée passe avec plus de 32 Mio de marge. Les modules de voix publiés, construits sur 0.1.3, restent au bord : ils sont à reconstruire sur 0.1.4 et à republier avec l'app qui annonce `typedCommands`.
- Les crates `escouade-plugin-protocol` 0.1.4 et `escouade-plugin-kit` 0.1.3 sont à publier par le chef. Le SDK JS/TS (0.4.0), le banc et les pages du site viennent avec T9.
