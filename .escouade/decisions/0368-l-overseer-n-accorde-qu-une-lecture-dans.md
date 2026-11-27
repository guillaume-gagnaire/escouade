---
number: 368
id: "l-overseer-n-accorde-qu-une-lecture-dans"
title: "L'Overseer n'accorde qu'une lecture dans le projet de l'agent, et ne lit que ses dossiers"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-646"
pages: ["overseer/overseer","harness/serveur-mcp"]
tickets: ["ESC-646","ESC-627"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"overseer/overseer","added":13,"removed":9},{"page":"harness/serveur-mcp","added":2,"removed":2}]
accepted_at: "2026-11-26T02:04:32+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de sécurité de la 2.3 (ESC-627) a trouvé deux failles hautes. SR4-02 : l'Overseer répondait aux permissions des agents à la place de l'utilisateur, et sa seule garde cherchait quelques commandes (git push, merge, tag, publish) écrites d'une seule façon dans le texte de la demande : `\git push`, un alias, un script, `curl`, une écriture ou une suppression passaient s'il les accordait. SR4-03 : ses propres lectures (Read, Grep, Glob, LS) étaient accordées d'office sur tout le poste, identifiants des harness et clé de signature compris. Les deux se chaînent sans geste de l'utilisateur : un texte piégé lu par l'Overseer (petit modèle, textes de tiers de tous les projets) lui fait lire un secret puis accorder à un agent de l'envoyer.

## Décision

1. L'Overseer n'accorde plus qu'une liste fermée de permissions : une lecture ou une recherche de fichiers (Read, Grep, Glob) qui mène dans le dossier du projet de l'agent ou dans son worktree, liens et jonctions suivis, jamais un lieu d'identifiants ou de clés. Toute autre demande (commande quelle que soit sa forme, écriture, suppression, réseau, outil d'un autre serveur MCP, lecture ailleurs) reste à l'utilisateur ; aucune consigne ne l'ouvre ; plus de garde par motif de texte pour décider d'un oui. L'Overseer peut refuser, ou laisser la demande à l'utilisateur par leave_to_user avec sa recommandation (recommend allow ou deny), montrée sur la demande dans « À toi », jamais une réponse.
2. Ses propres lectures passent sans question seulement dans son dossier, les dossiers des projets non privés et ceux de leur spec ; jamais ~/.claude, ~/.codex, ~/.copilot, ~/.escouade-signing, ~/.ssh, les trousseaux, ni un fichier d'identifiants ou de clé (.credentials.json, auth.json, .env, *.key, *.pem…), même dans un dossier permis. Le reste est refusé sur-le-champ.
3. La garde de ce qu'il dit aux agents (messages, réponses) reste, et lit le texte sans les masques d'un shell (barres obliques inverses, guillemets, $IFS) et avec les alias de git.

## Conséquences

Une permission de commande qu'un agent demande attend toujours l'utilisateur, même quand l'Overseer est allumé et même sous une consigne « tu peux pousser » : plus de gestes à faire dans « À toi », mais rien ne sort sans lui. L'Overseer lit la spec, les tickets et les documents par les outils d'Escouade ; une recherche dans un projet peut encore lire un fichier d'identifiants rangé sous un autre nom (limite dite dans la spec). Ses recommandations restent en mémoire (perdues au redémarrage, la demande attendant toujours).
