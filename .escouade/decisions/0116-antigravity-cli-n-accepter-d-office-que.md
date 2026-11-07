---
number: 116
id: "antigravity-cli-n-accepter-d-office-que"
title: "Antigravity CLI : n'accepter d'office que ce qui tient si agy lit une règle comme un début de texte"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-376"
pages: ["plugins/antigravity-en-plugin"]
tickets: ["ESC-376"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/antigravity-en-plugin","added":14,"removed":13}]
accepted_at: "2026-11-07T21:32:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La revue de dev.escouade.antigravity-cli@2.2.1 demande des changements : ce que words::judge accepte devient, pour la reprise du tour, une règle d'agy qu'agy lit peut-être comme un simple début de texte. Une écriture du dossier de l'agent lui-même (write_file(C:\wt)) ouvrait son .git et son .agents ; une règle faite du texte brut (relatif, `.`, `..`) n'était pas le chemin jugé ; `git add .` accordé pouvait laisser passer `git add . ; node x.js` ; sous clôture, `type ..\prive\secret.txt` passait ; la vue terminal du cœur écrivait encore les règles de la 2.2.0 et lisait escouade-always.json. L'essai réel (real_agy_rules) ne peut pas tourner : agy n'est pas sur le poste.

## Décision

Jusqu'à l'essai réel, le plugin (2.2.2) n'accepte d'office que ce qui tiendrait dans le pire cas : une écriture seulement pour un chemin strictement sous le dossier de l'agent, ni le dossier lui-même ni un début de nom de dossier gardé ; une acceptation d'office seulement quand le texte de l'appel est déjà le chemin absolu et normal jugé ; aucune commande d'office (git add et git commit en Édits auto, toute commande en Tout accepter sous clôture vont à une carte), seul « Pour cet agent » rejoue une commande ; un mot de commande résolu dans la clôture la fait refuser ; une reprise d'écriture ajoute des deny défensifs (dossiers gardés à la racine, <chemin>\.. et <chemin>/..). La carte et la conversation disent ce que valent « Toujours » et l'acceptation d'office. agentRepo quitte les permissions, le plan se lit par agentHome. La vue terminal du cœur pose les mêmes règles que le plugin, passe Édits auto en mode par défaut d'agy, n'utilise plus escouade-always.json et l'efface.

## Conséquences

Plus de cartes en Édits auto (git add, git commit) et en Tout accepter sous clôture (chaque commande), et une écriture nommée par un chemin relatif demande. Rien n'est relâché sans real_agy_rules, dont les cas couvrent maintenant le retour à la ligne, &, le ; de PowerShell, <fichier>\..\.git\x, le dossier lui-même, deny contre allow, la casse, * et les crochets. La 2.1 a le même défaut dans son moteur agy (règles 2.2.0, accept-edits, escouade-always.json lu et écrit) : un correctif en 2.1 est à décider.
