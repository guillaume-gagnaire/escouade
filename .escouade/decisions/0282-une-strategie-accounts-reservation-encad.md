---
number: 282
id: "une-strategie-accounts-reservation-encad"
title: "Les stratégies du poste jugent la réservation d'un compte (accounts.reservation) et les réglages par projet d'un plugin (plugins.projectSettings)"
date: "2026-11-21"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-564"
pages: ["fenetre/reglages-imposes","harness/comptes","plugins/profils-et-reglages-d-un-projet"]
tickets: ["ESC-564","ESC-556","ESC-563"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/reglages-imposes","added":27,"removed":0},{"page":"harness/comptes","added":5,"removed":0},{"page":"plugins/profils-et-reglages-d-un-projet","added":6,"removed":0}]
accepted_at: "2026-11-22T09:31:34+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

La 2.2.5 apporte la réservation d'un compte à un projet privé et « Partager à nouveau » (ESC-556), reversées dans la 2.3 par ESC-563. Les stratégies imposées (poste géré : ADMX/ADML, profil macOS, schéma Linux ; équipe : enforced du .escouade.yml d'équipe) ne la jugeaient pas : une organisation ne pouvait ni l'interdire ni l'exiger.

## Décision

Une seule clé de la liste unique, accounts.reservation, de sorte « restreindre », à deux valeurs. denied : aucun compte n'est réservé à un projet ; les listes « Réservé aux projets » sont vidées au démarrage et à chaud, tout enregistrement qui réserve est refusé, et un projet privé part sur les comptes partagés, protégé par sa clôture comme en 2.2.5. required : un projet privé ne part que sur un compte actif réservé à lui seul ; « Comptes partagés à la limite » est éteint et verrouillé ; un nouvel agent, un ticket, create_agent, le compte d'un ticket et chaque démarrage sont refusés sur un compte partagé ou sans compte à lui ; un agent qui tourne sur un compte partagé s'arrête entre deux tours ; retirer à un projet privé son dernier compte réservé (Ne plus le réserver, Partager à nouveau, Inactif) est refusé. Une clé et deux valeurs plutôt que deux interrupteurs : les deux cas s'excluent, et deux interrupteurs laisseraient imposer les deux à la fois, ce qui empêcherait un projet privé de rien démarrer. C'est une clé de l'app : une équipe l'impose tant que le poste en est, la valeur du poste gagne. La fenêtre montre le cadenas et « Imposé par … » sur chaque geste concerné ; un client distant reçoit les mêmes verrous et les mêmes refus.

## Conséquences

Les fichiers de stratégies (ADMX, ADML, profils macOS, schéma et exemple Linux) et la page « Stratégies du poste » du site sont régénérés avec les deux clés. Le formulaire ouvert des Réglages reprend les champs d'un projet qu'une stratégie modifie (privé, comptes partagés à la limite, Remote Control) au lieu de réenregistrer l'ancienne valeur. Les réglages par projet d'un plugin : la clé plugins.projectSettings, du poste géré seulement, est un JSON {"<plugin>": {"<clé>": valeur}}. Elle impose au rang 6, devant l'équipe (rang 5, ESC-532), dans chaque projet où le plugin est actif. Elle est jugée par le même contrôle que celle de l'équipe (team_refusal) : jamais un secret, un profil, une valeur importée ni un réglage repo: false. Une équipe qui l'écrirait est ignorée, et c'est dit : elle passe par plugins.<id>.<clé> de son fichier, ce qui évite de refaire ESC-532.
