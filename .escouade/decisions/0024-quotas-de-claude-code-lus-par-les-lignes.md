---
number: 24
title: "Quotas de Claude Code lus par les lignes limits[] du serveur, compteur d'un modèle jugé pour ce modèle seul"
date: "2026-10-31"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-312"
pages: ["harness/comptes","fenetre/barre-de-statut","kanban/pilote-auto"]
tickets: ["ESC-312"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"harness/comptes","added":13,"removed":7},{"page":"fenetre/barre-de-statut","added":2,"removed":0},{"page":"kanban/pilote-auto","added":3,"removed":1}]
accepted_at: "2026-10-31T17:23:57+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Claude Code 2.1.314 renvoie dans rate_limits (get_usage, endpoint /api/oauth/usage) des lignes limits[] : session (5 h), weekly_all et weekly_scoped (semaine de Fable, sa propre remise à zéro). Escouade ne lisait que five_hour et seven_day : le quota Fable était invisible (ni barre, ni seuil, ni pause), et le forfait Max sans fenêtre de 5 h gardait pour toujours une barre 5 h figée, car chaque lecture était fusionnée avec la précédente. Depuis la décision 20, le choix du compte passe par un seul jugement, pace::judge.

## Décision

1) limits[] est lu quand il est là, par groupe (session → 5 h, weekly → semaine), dans l'ordre et sous les noms du serveur ; une ligne de surface (Cowork…) est ignorée ; sans limits, repli sur five_hour, seven_day et model_scoped[]. 2) QuotaMeter gagne une portée facultative model (nom du serveur, « Fable »), absente pour un compteur de tous les modèles : l'état sauvé et le fil restent compatibles. 3) Une lecture complète (get_usage, endpoint) remplace les compteurs du compte ; seul rate_limit_event fusionne, par durée et portée ; sa fenêtre seven_day_overage_included (sans nom) prend celui du seul compteur hebdomadaire de modèle du compte. 4) Le filtre par modèle est dans pace::judge (et son miroir judge du front) : le jugement reçoit le modèle visé ; un compteur d'un modèle n'y entre que pour l'agent ou le ticket de ce modèle, rapproché par sa famille (premier mot du nom qui n'est ni claude ni un nombre), sans liste figée, et y est jugé par les règles de la décision 20 selon sa longueur ; sans modèle (le compte en général : barre d'état, panneau, toast) aucun n'entre. Ce qui retient les tickets à côté du choix (pause au seuil, attente d'un agent) prend le même filtre (pace::for_model). Un refus rate_limit_event de type seven_day_* ne met pas le compte en pause. 5) Une barre « Fable · semaine » de plus, même composant, partout où s'affichent les compteurs.

## Conséquences

Fable au seuil ou à sa limite : les tickets et agents Fable partent sur un autre compte ou attendent, Opus et Sonnet continuent sur le même compte ; la bannière de pause et le compte en cours de la barre d'état ne comptent pas ce compteur. Des cas avec un compteur Fable s'ajoutent à tests/fixtures/quota-pace.json, vérifiés des deux côtés. Un compte sans fenêtre 5 h perd sa barre 5 h à la lecture suivante. La forme d'un refus Fable est lue dans le binaire, pas observée. Une carte « À faire » d'un ticket Fable retenu ne dit pas encore pourquoi elle attend.
