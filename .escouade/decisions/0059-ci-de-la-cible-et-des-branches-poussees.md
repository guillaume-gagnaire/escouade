---
number: 59
id: "ci-de-la-cible-et-des-branches-poussees"
title: "CI de la cible et des branches poussées : pastilles, journaux, relance, pause du pilote"
date: "2026-11-02"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-63"
pages: ["kanban/ci-des-branches","kanban/pilote-auto","kanban/kanban","kanban/en-revue"]
tickets: ["ESC-63"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/ci-des-branches","added":46,"removed":4},{"page":"kanban/pilote-auto","added":10,"removed":4},{"page":"kanban/kanban","added":4,"removed":2},{"page":"kanban/en-revue","added":5,"removed":5}]
accepted_at: "2026-11-02T11:53:11+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Escouade suivait la CI des PR (tickets « En revue », agents), pas celle de la branche cible ni des branches poussées sans PR (« Valider et pousser », agents). Le journal d'un job ne se lisait que dans l'onglet « PR et CI » d'un agent, et le pilote auto lançait des tickets sur une cible cassée. Les hébergeurs sont des plugins livrés : une opération nouvelle passe par le trait Forge. Rien ne se publie au nom de l'utilisateur sans un réglage allumé ou un geste à la main (décision du 2026-10-16).

## Décision

Deux opérations d'hébergeur nouvelles dans le trait Forge, servies par chaque plugin : forge/branchChecks (les checks du dernier commit d'une branche) et forge/rerunFailedOfBranch (relancer ses jobs en échec ; Unsupported chez Bitbucket, que la fenêtre dit). La CI de la cible de chaque projet et celle des branches poussées par les tickets se lisent dans la boucle des PR des tickets, sans boucle à elles ; celle de la branche poussée d'un agent sans PR, avec ses PR. Une pastille (« main ✗ depuis 2 h ») dans l'en-tête du Kanban, sur la carte « Terminé » d'un ticket poussé et sous la carte d'un agent ; un clic ouvre une fenêtre des jobs : journal en lecture seule rendu comme un terminal (couleurs ANSI, erreurs en rouge), « Envoyer à un agent » (un agent du projet choisi dans un menu, le journal donné comme information), « Relancer les jobs en échec » sur un clic seulement. Réglage du Kanban « Pause quand la CI de la cible est rouge », éteint par défaut : le pilote auto ne démarre rien tant que la cible est rouge, un ticket lancé à la main part quand même. Pas de pastille pour une cible sans aucune CI.

## Conséquences

Une à deux requêtes de plus par projet et par passage (toutes les minutes fenêtre devant) ; une branche poussée n'est lue que tant que sa CI n'est pas fixée, 3 jours au plus. La liste des checks de l'onglet « PR et CI » devient un composant partagé (CiChecks) et ses journaux s'affichent comme un terminal. Les essais réels (relance comprise) restent à faire avec l'utilisateur sur chaque hébergeur (ESC-53, ESC-70) ; la relance de Bitbucket Pipelines est à vérifier.
