---
number: 56
id: "merger-depuis-escouade-et-merger-quand-c"
title: "Merger depuis Escouade, et « Merger quand c'est vert »"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-62"
pages: ["kanban/en-revue"]
tickets: ["ESC-62"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/en-revue","added":18,"removed":3}]
accepted_at: "2026-11-02T08:53:43+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Un ticket « En revue » (ESC-59) attendait que l'utilisateur merge sa PR sur l'hébergeur. Il fallait pouvoir merger depuis la carte, avec une stratégie que le dépôt autorise, et merger à la fin de la CI, y compris sur Bitbucket qui n'a pas d'auto-merge. Rien ne doit partir au nom de l'utilisateur sans un geste (décision du 2026-10-16).

## Décision

Deux boutons sur la carte « En revue », jamais un réglage. « Merger » n'est proposé qu'à une PR approuvée (au moins une approbation, aucun changement demandé), verte (CI réussie, ou pas de CI), sans conflit ni brouillon. Il prend la stratégie du Kanban si le dépôt l'autorise, sinon la première autorisée, et le bouton la dit. Ce que le dépôt autorise se lit par un nouvel appel du trait Forge, `merge_options` (`forge/mergeOptions`), servi par les cinq plugins d'hébergeurs. Un plugin qui ne le sert pas : les capacités déclarées par son fournisseur. « Merger quand c'est vert » n'est proposé que tant que la CI tourne. Il active l'auto-merge de l'hébergeur quand le dépôt en a un (GitHub si le dépôt l'autorise, GitLab, Gitea sur une branche protégée, Azure DevOps). Sinon Escouade le dit, garde l'attente avec le ticket (`TicketPr::merge_when_green`) et merge elle-même à la première lecture où la PR est approuvée et verte, dans la boucle de suivi d'ESC-59 (pas de seconde boucle). Une CI en échec ou des changements demandés arrêtent l'attente, et la carte le dit.

## Conséquences

Un appel de plus par PR suivie, une seule fois. L'attente d'Escouade ne marche que tant que l'app tourne, et la carte le dit. Elle reprend au redémarrage. Une PR verte mais jamais approuvée attend sans fin, jusqu'à « Ne plus attendre ». L'auto-merge activé sur l'hébergeur se coupe sur l'hébergeur, pas depuis Escouade. Tout est essayé contre les faux serveurs. Les essais réels de GitHub (le compte de l'utilisateur), de GitLab, de Bitbucket et de Gitea restent à faire (ESC-53, ESC-70).
