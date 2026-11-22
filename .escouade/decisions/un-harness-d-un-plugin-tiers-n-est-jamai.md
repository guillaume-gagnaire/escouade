---
number: null
id: "un-harness-d-un-plugin-tiers-n-est-jamai"
title: "Un harness d'un plugin tiers n'est jamais lancé dans un projet privé (SEC-02)"
date: "2026-11-22"
state: "proposed"
origin: "ticket"
origin_ref: "ESC-590"
pages: ["projets/projets-prives","plugins/harness-en-process"]
tickets: ["ESC-590"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"projets/projets-prives","added":13,"removed":5},{"page":"plugins/harness-en-process","added":5,"removed":0}]
accepted_at: ""
accepted_by: ""
---

## Contexte

L'audit de sécurité de la 2.2.0 (SEC-02, gravité moyenne) a démontré que, pour le harness d'un plugin tiers, le cœur vérifie la clôture d'un projet privé en cherchant chaque chemin comme une simple sous-chaîne des arguments, des variables ou des fichiers du lancement : un chemin posé dans une variable que personne ne lit, ou dans un argument qui l'autorise, satisfait le contrôle (test W-04). Le contrat des plugins ne permet aujourd'hui aucune preuve plus forte. Par ailleurs, sous Windows natif, la clôture reste une consigne au harness, même de première main. L'utilisateur a choisi le 2026-11-21 (demande a-4) l'option « C · a, plus l'étude d'un vrai cloisonnement Windows ».

## Décision

Un harness d'un plugin tiers (ni le plugin qui le sert ni celui qui le décrit n'est `dev.escouade.*`) n'est jamais lancé dans un projet privé, qu'il y soit activé ou non. Le refus se dit avec sa raison et le geste possible (« Un harness d’un plugin tiers ne peut pas garantir la clôture d’un projet privé : … Choisis un harness d’Escouade, ou retire « Projet privé » à ce projet. ») dans la conversation, le menu « Harness », la création d'un agent, un ticket bloqué et les réglages du projet. Un agent tiers déjà actif quand son projet devient privé s'arrête : au repos aussitôt, au travail à la fin de son tour, et sa conversation le dit. Les harness de première main gardent leur clôture comme avant. Un vrai cloisonnement sous Windows natif est chiffré dans une étude à part avant d'être promis.

## Conséquences

Le trou démontré par W-04 est fermé tout de suite, à peu de frais, pour les projets privés. Un utilisateur qui faisait tourner un harness tiers dans un projet privé ne le peut plus : il choisit un harness d'Escouade ou retire « Projet privé ». Un agent d'un projet non privé sur un harness tiers garde le contrôle d'avant (la sous-chaîne) pour la clôture des projets privés : c'est la part de SEC-02 qui reste ouverte, avec Windows natif. Une preuve vérifiable dans le contrat (option b de la demande) pourrait un jour rouvrir les projets privés aux harness tiers ; elle demanderait de changer le contrat, le kit et le SDK.
