---
number: 166
id: "une-pr-se-lit-entiere-par-les-plugins-d"
title: "Une PR se lit entière par les plugins d'hébergeur, son diff par git, gardée tant qu'elle ne change pas"
date: "2026-11-14"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-452"
pages: ["git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-452"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/hebergeurs-de-code","added":7,"removed":0}]
accepted_at: "2026-11-14T21:43:20+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La 2.3 montre une PR entière dans Escouade (direction A choisie le 2026-11-14, design/pr-2-3/README.md). Le cœur ne savait lire d'une PR que son état, sa CI, ses relectures, ses commentaires et ses déploiements : ni sa description, ni ses commits, ni ses fichiers, ni ses fils avec leurs réponses et leur état. PR-1 (ESC-452) donne au cœur de quoi lire tout cela avant toute vue.

## Décision

Quatre opérations de plus au protocole des hébergeurs (escouade-plugin-protocol et kit 0.1.3) : forge/description, forge/commits, forge/files et forge/threads, refusées par défaut (unsupported) et déclarées par la capacité `reading` du fournisseur. GitHub (plugin 2.3.0) les sert toutes ; GitLab, Gitea / Forgejo, Bitbucket et Azure DevOps n'en déclarent aucune jusqu'à PR-9. Les lignes du diff viennent de git, par des références refs/escouade/review/<n>/read/… supprimées aussitôt, jamais de l'hébergeur ; forge/files ne donne que les fichiers et leurs comptes. Un fil a trois états : ouvert, résolu (prioritaire), dépassé ; un commentaire de la conversation est un fil sans chemin. Le cœur (prread.rs, commande pr_contents) garde la lecture 20 s, puis tant que la PR suivie et sa description (commit de tête, date de mise à jour) ne changent pas ; le suivi des PR (liste du projet, agents, tickets) la marque à relire et le dit à la fenêtre (prChanged). Tout est marqué données d'un tiers (thirdParty), et get_pull_request l'ajoute dans sa balise, sans les lignes.

## Conséquences

PR-2 à PR-9 lisent une seule commande, sans appel de plus ni diff à eux. Une PR inchangée coûte deux appels (la description est conditionnelle sur GitHub, une réponse 304 ne compte pas) ; une PR changée en coûte cinq de plus et une récupération git. Le plugin GitHub 2.3.0 attend la publication du kit 0.1.3 et du protocole 0.1.3 sur crates.io (AWAITING_PUBLICATION) avant sa signature au store par le chef. Un plugin d'hébergeur d'un tiers peut servir les mêmes lectures.
