---
number: 65
id: "copilot-cli-passe-par-son-plugin-en-proc"
title: "Copilot CLI passe par son plugin, en process, et sa vue lit sa traduction en bibliothèque"
date: "2026-11-04"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-224"
pages: ["plugins/copilot-cli-en-plugin","plugins/harness-en-process","plugins/livres-avec-escouade"]
tickets: ["ESC-224"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/copilot-cli-en-plugin","added":18,"removed":0},{"page":"plugins/harness-en-process","added":8,"removed":0},{"page":"plugins/livres-avec-escouade","added":2,"removed":0}]
accepted_at: "2026-11-04T08:06:29+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-10.3 sort Copilot CLI du cœur. Ce n'est pas un harness ACP : son client JSON-RPC parle en Content-Length, ouvre la session en plusieurs étapes, et sa traduction (1 200 lignes) était écrite sur l'état de l'agent. La vue Copilot lit le journal de la session par cette même traduction. Le contrat d'INT-9 ne tenait ni ce cadrage, ni l'ouverture par le plugin, ni /compact, ni le titre, ni le quota demandé au harness.

## Décision

Le plugin de première main dev.escouade.copilot-cli tient le client et la traduction, en machine à états sans process. Le moteur Copilot reste au registre pour les comptes, le quota et les modèles d'un compte, copilot -p, le fichier de l'entrée d'Escouade et la vue. La vue lit le journal par la traduction du plugin, prise comme une bibliothèque : le cœur dépend de la crate du plugin pour cela seul (et ses règles pures, lues par le quota et le menu). Le contrat grandit pour tous : framing, closeLine, opens, des writes dans les réponses d'agent/compact et d'agent/rename (ceux d'OpenCode), et de nouveaux éléments plutôt que des champs ajoutés aux anciens, pour que les plugins déjà écrits restent tels quels. Copilot garde git add et git commit acceptés sans demander : son plugin lance git pour lire où sont le dépôt et ses hooks, et les règles passent dans crates/escouade-acp.

## Conséquences

Aucun changement pour l'utilisateur. Une seule traduction pour la conversation et la vue, au prix d'une dépendance du cœur vers une crate de plugin, à retirer si la vue passe un jour par le process. Le plugin de Copilot lance git, ce que les autres harness sortis ne font pas : c'est dit dans son README.
