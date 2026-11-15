---
number: 175
id: "les-strategies-du-poste-une-liste-unique"
title: "Les stratégies du poste : une liste unique, ses fichiers générés et la relecture à chaud"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-465"
pages: ["fenetre/reglages-imposes"]
tickets: ["ESC-465"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages-imposes","added":52,"removed":15}]
accepted_at: "2026-11-15T14:19:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-11 (ESC-455) avait posé le moteur des couches avec une table de sept clés, lue au démarrage seulement ; TS-0b (ESC-454) gardait une liste provisoire de noms (PROVISIONAL_KEYS). Le §6.3 de docs/TEAM-SERVER.md demande une seule liste d'où sortent l'ADMX, les ADML, le manifeste macOS, le schéma de /etc et la page du site, une relecture à chaud et l'enrôlement par le poste géré.

## Décision

- Une seule liste, crates/escouade-policy/src/list.rs (30 clés, tout ce que le §6.4 dit imposable par le poste géré, plus l'enrôlement) : chaque clé a son type, sa sorte, sa forme, les lignes des Réglages qu'elle verrouille et ses textes fr/en. Elle est la table du cœur (locks::KEYS) et remplace PROVISIONAL_KEYS.
- Deux formes s'ajoutent à celles de TS-11 : le plafond (harnesses.maxEffort, mcp.externalClients ; le plus bas tient entre l'équipe et le poste) et la liste d'interdits (plugins.denied, projects.deniedFiles, proxy.exclusions ; elles s'ajoutent).
- En sont générés, sous policies/ et website/content/docs/{fr,en}/managed-policies.md, l'ADMX et ses ADML, le manifeste ProfileManifests, le schéma Jamf, un .mobileconfig d'exemple, le schéma et un exemple de /etc ; un test refuse tout écart (cargo run -p escouade-policy --example generate les refait).
- Le rang 6 est relu à chaud : RegNotifyChangeKeyValue sur les deux clés Policies (sous-arbre) sous Windows, les fichiers de /Library/Managed Preferences et de /etc/escouade regardés chaque seconde, tout relu toutes les 30 s. Un changement est appliqué comme au démarrage ; la fenêtre reçoit les réglages ramenés avec les verrous.
- Ce qui tourne suit la règle de TS-11 ; un agent d'un harness devenu interdit voit son process fermé à la fin de son tour.
- L'enrôlement (team.server, team.fingerprint, team.onlyThisServer) est gardé par le cœur, avec le refus d'un autre serveur (Locks::check_server) ; l'écran viendra avec TS-4. Les clés dont la fonction n'existe pas encore (espace perso, vue agrégée, notifications sans contenu, motifs de fichiers) sont lues et gardées, dites « à venir ».

## Conséquences

Ajouter une stratégie, c'est une entrée de la liste puis la génération : l'ADMX, les profils et la doc suivent, et le test casse tant qu'on ne les a pas refaits. Les stratégies changées par un service informatique s'appliquent sans redémarrer. Reste à essayer avec les droits d'administrateur un vrai GPO sous gpedit (scripts/windows-trial.ps1) et un vrai profil MDM sur un Mac.
