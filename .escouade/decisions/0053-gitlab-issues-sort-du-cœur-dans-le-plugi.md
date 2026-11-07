---
number: 53
id: "gitlab-issues-sort-du-cœur-dans-le-plugi"
title: "GitLab Issues sort du cœur dans le plugin GitLab, un client par instance"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-218"
pages: ["escouade/spec/plugins/livres-avec-escouade"]
tickets: ["ESC-218"]
documents: []
supersedes: []
superseded_by: null
changes: []
accepted_at: "2026-11-02T05:49:48+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

INT-8.2 (ESC-218), après ESC-56 qui avait écrit le client de MR en plugin. GitLab Issues était encore dans le cœur, avec un client tenant toutes les instances à la fois ; GitHub Issues (ESC-217) n'a qu'un seul compte, github.com.

## Décision

La source GitLab Issues du Kanban est servie par le plugin de première main dev.escouade.gitlab (issues.rs, un SourceKind du kit, ticketSources du manifeste), sur les mêmes comptes que les MR : un compte d'instance sert les deux usages. Le client du plugin est celui d'un compte ; le cœur en garde un par instance (HostedTicketSource) et route par l'hôte du conteneur, dont le format <hôte>/<groupes>/<projet> ne change pas. integrations/gitlab.rs est supprimé.

## Conséquences

Aucun changement visible : mêmes comptes, mêmes liens enregistrés, mêmes projets. Plugin GitLab coupé ou arrêté : les MR et GitLab Issues le disent. Azure Boards (ESC-280) peut reprendre HostedTicketSource pour ses organisations.
