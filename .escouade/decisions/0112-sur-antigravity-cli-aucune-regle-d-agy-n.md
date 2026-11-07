---
number: 112
id: "sur-antigravity-cli-aucune-regle-d-agy-n"
title: "Sur Antigravity CLI, aucune règle d'agy ne garde rien seule : le plugin juge chaque refus"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-371"
pages: ["plugins/antigravity-en-plugin","projets/projets-prives"]
tickets: ["ESC-371"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/antigravity-en-plugin","added":18,"removed":8},{"page":"projets/projets-prives","added":5,"removed":0}]
accepted_at: "2026-11-07T16:54:15+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de dev.escouade.antigravity-cli@2.2.0 (B1 à B3, C1 à C8) : le plugin écrivait dans le settings.json d'agy des règles qu'agy juge seul, comme un début de texte (write_file(<dossier>) en Demander, command(git add) et command(git commit) en Édits auto, command(*) en Tout accepter même sous clôture), et ne voyait que ce qu'agy refusait. Une écriture dans .git ou .agents, ou `git add .; node x.js`, passait sans carte. Comment agy juge ses règles n'a jamais été essayé, et agy n'est pas sur le poste.

## Décision

Hors de « Tout accepter » sans clôture ni discussion, le plugin ne fait écrire aucune règle allow à lui, et « Édits auto » n'est plus le mode accept-edits d'agy : tout ce qu'agy refuse revient au plugin à la fin du tour, qui le juge (refusé sans carte : clôture au chemin résolu, discussion ; accepté sans carte : écriture dans le dossier de l'agent hors .git, .husky, .githooks, .agents et .gemini, git add/commit seuls lus par accepts_script avec fs/gitProtected en Édits auto, lecture des fichiers joints, tout en Tout accepter ; une carte pour le reste). Ce qui est accordé l'est par la règle exacte de l'appel, pour le tour suivant, puis retiré, agy relancé. Pas de règle faite d'un texte avec *, ?, parenthèse ou retour à la ligne. « Toujours » est gardé par le plugin en mémoire (appel exact), jamais offert sous clôture ni pour un dossier gardé ; escouade-always.json n'est plus lu, toolPermission et agentMode sont retirés, seul le serveur d'Escouade reste dans mcp_config.json. Les règles sont réécrites à la re-attache, à chaque fin de tour et à l'arrêt, et agy relancé après tout changement de mode.

## Conséquences

Une écriture ou un git add en Demander et Édits auto coûte une reprise du tour (agy relancé, l'appel refait). « Toujours » est oublié quand le plugin redémarre. Une commande avec * ou une parenthèse ne peut pas être autorisée en -p. Le plugin déclare agentRepo en lecture (fs/gitProtected). L'essai réel real_agy_rules (ignoré) est à lancer quand agy sera remis. La vue terminal du cœur garde ses propres règles (hors périmètre) ; Vibe a le même défaut que C8 (VIBE_HOME), non corrigé.
