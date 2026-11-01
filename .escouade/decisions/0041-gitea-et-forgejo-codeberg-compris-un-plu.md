---
number: 41
id: "gitea-et-forgejo-codeberg-compris-un-plu"
title: "Gitea et Forgejo (Codeberg compris) : un plugin livré, des capacités dites par version"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-58"
pages: ["git-fichiers/hebergeurs-de-code","git-fichiers/pull-requests","plugins/livres-avec-escouade"]
tickets: ["ESC-58"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/hebergeurs-de-code","added":15,"removed":5},{"page":"git-fichiers/pull-requests","added":6,"removed":6},{"page":"plugins/livres-avec-escouade","added":1,"removed":0}]
accepted_at: "2026-11-01T21:53:18+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-58 : le client Gitea / Forgejo des PR, du merge, de l'auto-merge, des statuts et des Actions. Décision 7 du 2026-10-26 (tout l'externe en plugins) et modèle d'INT-8.1 (GitHub, décision 39) : aucun client d'hébergeur nouveau dans le cœur. Pas d'essai réel pour Gitea et Forgejo (décision du 2026-10-16). Les swagger de Forgejo 16 et de Gitea (branche principale) montrent des écarts : merge (do / Do), relecteurs à la création, exécutions et jobs des Actions, relance et réponses de fils (Gitea seul), pas de champ brouillon ni de déploiements nulle part.

## Décision

Gitea et Forgejo sont servis par un seul plugin de première main, dev.escouade.gitea (crate plugins/builtin/dev.escouade.gitea sur escouade-plugin-kit, entrée « gitea » d'escouade-plugins), un hébergeur « gitea » qui couvre les deux. Un brouillon est un titre préfixé « WIP: ». L'auto-merge (merge_when_checks_succeed) n'est programmé que sur une branche protégée : le client lit la branche avant et refuse sans rien envoyer si elle ne l'est pas. Les déploiements sont une capacité absente (aucune API). Ce que la version de l'instance ne permet pas (jobs et journaux des Actions : Gitea 1.25+, Forgejo 12+ ; relance et réponses/résolution de fils : Gitea 1.26+ seulement) est dit appel par appel par « Unsupported », d'après /version lue une fois par compte. Les tests tournent sur un faux serveur nourri de réponses écrites d'après les swagger (tests/fixtures/forges/gitea/), une fois en Gitea et une fois en Forgejo ; elles ne sont pas enregistrées d'une instance.

## Conséquences

Les versions minimales viennent des sources et des notes de version, pas d'un essai : elles sont à relever sur de vraies instances avec les routes des Actions, la lecture d'un statut d'Actions vers son job, le sens de new_position, et le comportement de merge_when_checks_succeed sur des contrôles déjà verts (liste dans docs/TODO-2.2.md). Les capacités du hébergeur « gitea » sont statiques (journaux et relance déclarés présents) ; l'interface peut donc offrir « Relancer les échecs » à un Forgejo, qui répond qu'il ne sait pas le faire. L'auto-merge déjà programmé n'est pas lu (ni Gitea ni Forgejo ne le disent dans la PR).
