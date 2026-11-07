---
number: 107
id: "les-regles-d-un-harness-se-reprennent-du"
title: "Les règles d'un harness se reprennent du cœur et lisent ce que le harness donne vraiment"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-362"
pages: ["plugins/copilot-cli-en-plugin","plugins/harness-en-process","plugins/harness-acp-decrits"]
tickets: ["ESC-362"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/copilot-cli-en-plugin","added":7,"removed":2},{"page":"plugins/harness-en-process","added":6,"removed":0},{"page":"plugins/harness-acp-decrits","added":4,"removed":0}]
accepted_at: "2026-11-07T07:23:46+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue des plugins pour le store a montré que les règles tenues dans un plugin ne résistaient pas à son redémarrage (la langue ou le réseau changés) : Copilot, Codex et escouade-acp repartaient sans clôture ni lecture seule, et un agent ACP perdait en plus sa session, sans que rien la rouvre. Sur Copilot, le texte de shell était lu comme un argv (`git add .; node evil.js` passait sans carte), `.git` s'écrivait sans carte, et `resolvedPath`, `managedApprovalRequired`, `resolvedPaths` et `possibleUrls` n'étaient pas lus dans leur forme réelle.

## Décision

1. Un plugin reprend la clôture et la lecture seule de `agent/attach`, à la création de l'agent et quand il lui est redonné ; le cœur l'emporte sur ce que le plugin a gardé. 2. Le texte d'un shell n'est jamais lu comme un argv : il l'est selon `shellType`, et un shell que ce nom ne dit pas doit se lire pareil en PowerShell et en POSIX. Codex garde sa règle, il donne un vrai argv. 3. Une écriture dans `.git` n'est pas une écriture dans le dossier de l'agent : elle a sa carte, sur Copilot et sur les harness d'escouade-acp, par une seule fonction de la crate (`in_git_folder`), Bypass excepté. 4. Un agent clôturé n'a pas de règle de session sur Copilot : ce qu'elle couvrirait ne serait plus demandé, donc plus jugé. 5. Pour un agent clôturé, un commit qui lit un fichier a sa carte quel que soit le chemin, au lieu de comparer ce chemin à la clôture : un lien de l'agent mènerait à la clôture sans la nommer, et un module n'a pas de disque pour le suivre. 6. Un agent ACP recréé pour un harness qui tourne encore reprend du cœur sa session, son tour en cours et ses permissions en attente (`Agent::started_again`, appelé par le service seul), et un lancement repart de zéro.

## Conséquences

Une clôture ne se perd plus à un changement de langue ou de réseau, et un agent ACP continue sa conversation après coup. Un agent clôturé voit plus de cartes : chaque écriture après un plan approuvé, et tout `git commit -F`, même vers un fichier de son dossier. Une permission reprise répond avec les options par défaut d'ACP, ce qu'elle proposait étant perdu ; un plan ou une question tenus par le client ne se reprennent pas. `escouade-acp` 0.1.3 gagne `in_git_folder`, `git::accepts_fenced`, `git::accepts_script` et `Agent::started_again` ; `git::accepts` reste, sans clôture.
