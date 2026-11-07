---
number: 45
id: "gitlab-son-client-de-mr-en-plugin-livre"
title: "GitLab : son client de MR en plugin livré, avant GitLab Issues"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-56"
pages: ["git-fichiers/hebergeurs-de-code","git-fichiers/pull-requests","plugins/livres-avec-escouade"]
tickets: ["ESC-56","ESC-218","ESC-217"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/hebergeurs-de-code","added":12,"removed":2},{"page":"git-fichiers/pull-requests","added":7,"removed":7},{"page":"plugins/livres-avec-escouade","added":2,"removed":1}]
accepted_at: "2026-11-02T00:27:19+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ESC-56 devait donner à l'interface « hébergeur » son client GitLab (MR, merge et auto-merge, pipelines, journaux, relance, discussions, déploiements) sur gitlab.com et les instances auto-hébergées. Depuis INT-8.1 (décision 39), aucun client d'hébergeur nouveau ne va dans le cœur : il s'écrit en plugin de première main sur le kit. GitLab Issues (ESC-165) est encore dans le cœur, sur les mêmes comptes, et ne sort qu'avec INT-8.2 (ESC-218). L'utilisateur a décidé le 2026-10-16 qu'il n'y a pas d'essai réel sur GitLab : tout se fait d'après la doc et les sources.

## Décision

Le client GitLab est le plugin livré `dev.escouade.gitlab` (crate `escouade-plugin-gitlab`, programme `escouade-plugins gitlab`, au registre des releases). Le cœur ne garde de GitLab que ce qu'il en déclare (hôtes, droits, `glab`) et passe ses comptes au plugin ; GitLab passe à « ouvre et lit les MR » et « relance ». Le manifeste ne déclare que l'hébergeur : la place de GitLab Issues y est prévue pour INT-8.2, et en attendant le genre de compte `gitlab` du plugin garde la source du cœur que ses comptes servent (`accounts::kinds`), si bien que le compte d'une instance sert toujours ses Issues et ses MR. Choix d'API : `auto_merge` et `merge_when_pipeline_succeeds` envoyés ensemble (avant et après GitLab 17.11) ; merge ou squash seulement (rebase et fast-forward sont un réglage du projet) ; « relancer » relance le pipeline de tête ; les fils ouverts ne sont lus que quand GitLab dit qu'il en reste ; les déploiements de la branche lus parmi les 50 derniers du projet (GitLab ne les filtre pas par branche). Tests sur un faux serveur nourri de réponses enregistrées sur gitlab.com (une MR d'un sous-groupe venue d'un fork), les discussions et déploiements écrits d'après la doc faute de jeton.

## Conséquences

« Ouvrir une MR », la lecture des MR, l'onglet « MR et CI », la relance et les déploiements marchent sur GitLab comme sur GitHub, sans essai réel : ce qui reste à essayer avec un jeton est listé dans docs/TODO-2.2.md. INT-8.2 n'a plus qu'à déplacer GitLab Issues dans ce plugin (une SourceKind et `ticketSources` au manifeste). Un plugin de première main qui remplace un genre du cœur sans déclarer toutes ses sources les garde : à retirer de `kinds` quand plus aucune source n'est au cœur.
