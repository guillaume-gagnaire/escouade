---
number: 342
id: "inscrire-une-machine-de-l-equipe-par-un"
title: "Inscrire une machine de l'équipe par un code de l'administrateur, et son rôle de service figé"
date: "2026-11-26"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-606"
pages: ["serveur-equipe/overseer-equipe","serveur-equipe/entree","serveur-equipe/audit-siem"]
tickets: ["ESC-606"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/overseer-equipe","added":14,"removed":4},{"page":"serveur-equipe/entree","added":8,"removed":0},{"page":"serveur-equipe/audit-siem","added":4,"removed":3}]
accepted_at: "2026-11-26T17:52:22+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

OE-1 (ESC-606) construit le rôle de service décidé par A2-A et O2-A (une machine de l'équipe, ni membre ni administrateur). La conception (TEAM-SERVER.md §3.4) disait que `escouade-server team pair` affiche le code court sur la machine, à taper dans le client ; mais le relais d'appairage (SR2-05) ne laisse ouvrir un créneau qu'à un appareil déjà de l'équipe, ce que la machine n'est pas encore. Il fallait aussi dire si le rôle de service peut se donner ou se retirer après l'entrée, et comment le serveur la traite. Enfin, `escouade-server` n'ouvrait pas les équipes : la machine en mode serveur ne pouvait pas en tenir une.

## Décision

1. Le code va de l'administrateur à la machine, comme pour « Ajouter un appareil » : « Machines de l'équipe › Inscrire une machine de l'équipe » (un administrateur seul) affiche un code d'une heure et d'un essai, sous une invitation `NewMember(Service)` ; sur la machine, `escouade-server team pair <serveur> <CODE> [--name <nom>]` le laisse au serveur qui tourne sur son dossier (fichier privé, effacé avant d'être joué), qui joue le SPAKE2 et affiche les six mots de la demande ; l'administrateur les compare avant d'admettre. 2. Le rôle de service ne se donne qu'à l'entrée et ne change jamais : ni une personne faite machine, ni une machine faite personne ; le propriétaire est une personne (règle de `policy.rs`, vérifiée par chaque poste, et refusée aussi par le serveur : `service_stays`). 3. `policy::may_sign` dit ce que signe chaque rôle ; la machine signe ses appareils et le contenu des projets ouverts, rien d'autre (ni proposition de plugin, ni proposition de rattachement, à la différence d'un membre). 4. Le serveur voit la sorte « service » et la refuse sur ses routes d'administrateur et sur les flux propres de l'équipe (sauf les totaux des coûts). 5. `escouade-server` ouvre les équipes comme le poste et les sert aux fenêtres distantes ; « Retirer » une machine est un geste du poste seul, aucun outil du MCP ne donne ni ne retire le rôle.

## Conséquences

La machine en mode serveur devient un appareil d'équipe à part entière (Kanban, docs, passages), base d'OE-2 à OE-6 et d'AU-5. Une machine compromise ne peut ni faire entrer quelqu'un, ni changer un rôle, ni imposer un réglage, ni couper l'équipe : au pire elle écrit du contenu, comme un membre, et se retire comme un appareil. Pour changer de rôle, on retire la machine et on en inscrit une autre. Un client d'avant OE-1 ne lit pas un registre qui porte une machine (rôle inconnu) : sans effet, la 2.3 n'est pas publiée.
