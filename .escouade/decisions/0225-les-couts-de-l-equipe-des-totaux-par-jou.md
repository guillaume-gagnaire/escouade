---
number: 225
id: "les-couts-de-l-equipe-des-totaux-par-jou"
title: "Les coûts de l'équipe : des totaux par jour scellés à une clé des administrateurs, par projet tant que le membre n'a pas lu qu'on passe par membre"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-547"
pages: ["serveur-equipe/couts-equipe","serveur-equipe/fichier-equipe","kanban/pilote-auto"]
tickets: ["ESC-547"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"serveur-equipe/couts-equipe","added":51,"removed":0},{"page":"serveur-equipe/fichier-equipe","added":6,"removed":0},{"page":"kanban/pilote-auto","added":4,"removed":0}]
accepted_at: "2026-11-20T12:01:53+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

TS-16 (docs/TEAM-SERVER.md §10) : le tableau des coûts et des quotas de l'équipe. Seuls des totaux remontent, chiffrés pour les administrateurs ; le grain par membre est un réglage de l'équipe (C15, par projet d'office), dit à chaque membre. Il fallait choisir comment chiffrer pour les seuls administrateurs alors que l'équipe est un groupe MLS (C9-B) où chacun lit tout ce qui est scellé à la clé d'équipe, où vivent les budgets, et ce que « dit à chaque membre » veut dire avant le premier envoi à son nom.

## Décision

1) Une paire HPKE à part (escouade-team-crypto, costkey.rs) : chaque poste scelle ses totaux à sa moitié publique ; la moitié secrète est enveloppée pour chaque appareil d'administrateur, à une clé des coûts propre à l'appareil. Deux flux d'équipe : la clé (écrite par un administrateur seul, le serveur refuse un membre) et les totaux (ouverts à tous). Rotation dès qu'un appareil enveloppé n'est plus un appareil d'administrateur ; d'ici là aucun poste ne scelle à l'ancienne clé ; à une nouvelle clé, chaque poste renvoie son mois. 2) Ce qui part : par jour, projet (id du journal), harness, modèle et forfait, les tokens, le coût, le plus haut quota du jour, et le total de chaque ticket par sa clé ; un envoi par heure, chaque jour envoyé entier remplace le précédent (hors ligne sans doublon). 3) Les budgets et le grain sont des clés du fichier d'équipe signé (costs.grain, costs.budgets, costs.stopAutopilot) : chaque membre les lit pour peser un lancement. 4) Par membre, les totaux d'un membre ne portent son nom qu'une fois qu'il a vu l'écran qui le dit. 5) Un membre lit le total de chaque projet et de l'équipe par un résumé du mois qu'un appareil d'administrateur publie sous la clé d'équipe. 6) Au-delà d'un budget avec stopAutopilot, le passage du pilote auto retient les tickets du projet ; un budget de l'équipe ou du membre pose aussi un verrou (rang 5) sur la ligne « Pilote auto ». 7) L'alerte de l'administrateur est une demande de la boîte « À toi », à 80 % puis 100 %, une fois par budget et par mois.

## Conséquences

Le serveur ne voit ni coût, ni modèle, ni clé de ticket ; un membre ne lit que les siens. Le grain « par projet » est tenu par l'app, pas par la cryptographie (l'appareil d'un administrateur voit quel appareil a signé chaque envoi) : la page le dit. Un ancien administrateur ne lit rien de scellé après son départ, au prix de totaux qui attendent tant qu'aucun administrateur n'est en ligne. Le total d'un membre est celui de son poste. Le tableau vit dans la fenêtre des équipes (« Coûts ») tant que Réglages › Équipe n'existe pas (TS-4).
