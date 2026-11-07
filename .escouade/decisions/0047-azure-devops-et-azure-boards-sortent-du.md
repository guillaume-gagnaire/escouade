---
number: 47
id: "azure-devops-et-azure-boards-sortent-du"
title: "Azure DevOps et Azure Boards sortent du cœur et deviennent un plugin de première main"
date: "2026-11-01"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-280"
pages: ["plugins/livres-avec-escouade"]
tickets: ["ESC-280"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/livres-avec-escouade","added":2,"removed":1}]
accepted_at: "2026-11-02T00:57:58+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Décision du 2026-10-26 : tout l'externe en plugins, en process. GitHub, Bitbucket, Gitea et Trello sont déjà sortis ; Azure DevOps (ESC-241) et Azure Boards (ESC-242) restaient au cœur.

## Décision

Un seul plugin livré, dev.escouade.azure-devops, porte l'hébergeur et la source Azure Boards sur le compte de l'organisation. Le jeton de az reste lu par le cœur et prêté comme un jeton ordinaire. Azure Boards sur plusieurs organisations passe par un aiguilleur du cœur (ByOrganization), un compte par organisation. Le kit gagne TicketSource::states_view pour les défauts de colonnes par catégorie.

## Conséquences

Rien ne change à l'usage. Les essais réels real_azure_devops et real_azure_boards restent compilés et ignorés, à jouer avec un jeton (noté dans docs/TODO-2.2.md).
