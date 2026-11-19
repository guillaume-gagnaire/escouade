---
number: null
id: "un-hebergeur-ou-une-source-injoignable-s"
title: "Un hébergeur ou une source injoignable se dit par -32503, dans chaque plugin livré"
date: "2026-11-18"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-536"
pages: ["git-fichiers/hebergeurs-de-code"]
tickets: ["ESC-536"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/hebergeurs-de-code","added":6,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

Le 2026-11-18, la CI de main (run 44280141503) a échoué sur le banc de Gitea. Sans compte réglé, le plugin joint codeberg.org, que le DNS du runner ne résolvait pas, et il rend -32000, une erreur que le banc refuse. Le kit (0.1.1 et 0.1.2) dit « injoignable » par une erreur sans type. Le protocole n'a pas de code à lui pour ce cas.

## Décision

Un service hors d'atteinte (DNS, connexion, délai) est un HttpError 503, donc le code -32503 du protocole. Il n'y a pas de code nouveau et le protocole garde sa version. Le message fr et en dit quoi vérifier : la connexion, l'adresse du compte, le proxy de Réglages › Avancé › Réseau. Le changement est fait dans chaque plugin livré (src/reach.rs), pas dans le kit. Un kit 0.1.3 aurait bloqué la republication au store jusqu'à sa sortie sur crates.io, puis un plugin-lock.sh de neuf plugins. Le kit pourra reprendre ce module à sa prochaine version. Le banc de la première main joue hors ligne (--offline), comme celui de la revue du store (68b2467d).

## Conséquences

Gitea 2.2.3, GitHub 2.2.4, GitLab 2.2.4, Bitbucket 2.2.4, Azure DevOps 2.2.5, Trello 2.2.5, Jira 2.2.5, Linear 2.2.5 et ClickUp 2.2.5 sont à republier au store. Le cœur réessaie comme avant (503 et injoignable étaient déjà des pannes passagères), et le banc accepte l'erreur. Une app d'avant comprend ce code sans mise à jour.
