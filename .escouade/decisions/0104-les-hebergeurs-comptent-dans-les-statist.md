---
number: 104
id: "les-hebergeurs-comptent-dans-les-statist"
title: "Les hébergeurs comptent dans les statistiques, sans nom ni adresse, et la page Confidentialité suit le code"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-69"
pages: ["git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-69"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/hebergeurs-de-code","added":10,"removed":0}]
accepted_at: "2026-11-07T03:41:47+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le plan de la 1.8 voulait des statistiques sur les pull requests, sans nom ni adresse, et une page Confidentialité qui dise que rien ne part vers Escouade. Le code n'envoyait aucun de ces événements, et la page ne listait plus tout ce que l'app envoie : plugin_installed manquait, ainsi que gitlab, azure et other dans service. Un hébergeur d'instance auto-hébergée ou de plugin tiers pouvait aussi révéler un nom s'il était envoyé tel quel.

## Décision

Quatre événements entrent dans le cœur : pr_opened (from, forge), pr_merged (forge), ci_fix_started (from, by) et pr_review_started (forge). forge vient d'une liste fermée (github, gitlab, bitbucket, gitea, azure), tout autre hébergeur étant other : jamais une adresse ni un nom de plugin. pr_merged compte la PR d'un ticket, mergée depuis Escouade ou chez l'hébergeur ; pr_review_started compte le départ de la relecture. Un test du site compare la page Confidentialité à src-tauri/src/telemetry.rs, en français et en anglais. La page dit que jetons et appels restent entre l'app et l'hébergeur, et ce qui part quand même : le diff chez le service du harness, et le simple fait qu'une PR a été ouverte, si l'utilisateur a accepté les statistiques. Le site dit ce qui a été essayé : GitHub en lecture sur le vrai service, ses écritures contre le faux GitHub des tests, les autres hébergeurs d'après leur documentation.

## Conséquences

Le nombre d'événements passe à 30, et le registre RGPD le dit. Le traitement reste celui des statistiques : mêmes base légale, durée et destinataire. La build du site lit désormais src-tauri/src/telemetry.rs (Dockerfile.dockerignore). Un événement ajouté sans sa ligne sur la page fait échouer les tests du site, donc le déploiement. La phrase sur ce qui est essayé est à revoir à chaque essai réel (ESC-70).
