---
number: 105
id: "un-pret-a-un-plugin-finit-avec-ce-pour-q"
title: "Un prêt à un plugin finit avec ce pour quoi il a été fait, et une adresse se lit par un parseur"
date: "2026-11-07"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-360"
pages: ["plugins/comptes-partages"]
tickets: ["ESC-360"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"plugins/comptes-partages","added":7,"removed":0}]
accepted_at: "2026-11-07T06:06:13+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le cœur prête à un plugin de première main l'hôte saisi et le jeton d'un compte. Ces prêts ne finissaient jamais : l'hôte d'un essai raté restait joignable par {account} jusqu'au redémarrage, un seul hôte était retenu par plugin (deux essais en parallèle se le prenaient), et le jeton restait prêté après le retrait du compte. jira::secure découpait l'adresse à la main : http://localhost:1@exemple.dev passait pour locale.

## Décision

Un essai de connexion tient ses prêts dans un Trial, à lui seul, repris quand il finit, réussi, raté ou abandonné ; réussi, son jeton passe au compte enregistré. Un compte enregistré garde son jeton prêté jusqu'à son retrait et n'a pas d'hôte prêté (hosts_of le couvre). Le retrait reprend le jeton pour tout plugin, sous le verrou des comptes, et un jeton lu pendant le retrait n'est pas prêté. jira::secure passe par reqwest::Url : l'hôte réel décide.

## Conséquences

Un essai qui échoue ne laisse rien, et ne touche ni un compte qui tourne ni un autre essai. Reste vrai : pendant un essai sur un compte déjà connecté, un appel de ce compte lit le jeton essayé, secrets/get ne disant pas qui demande. Une adresse https reste acceptée quel que soit son utilisateur. Un plugin tiers, qui lit le trousseau, n'a toujours pas d'hôte prêté pendant son essai : inchangé ici.
