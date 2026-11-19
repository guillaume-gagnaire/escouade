---
number: 215
id: "on-2-le-premier-projet-et-le-premier-age"
title: "ON-2 : le premier projet et le premier agent du premier lancement"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-505"
pages: ["fenetre/premier-lancement","fenetre/a-decouvrir"]
tickets: ["ESC-505"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/premier-lancement","added":48,"removed":15},{"page":"fenetre/a-decouvrir","added":4,"removed":2}]
accepted_at: "2026-11-19T09:40:56+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

ON-1 (ESC-504, décision 195) a posé le parcours du premier lancement avec une station du projet réduite à « Ajouter un projet… » et sans premier agent. La direction C « La ligne » et les choix O3 à O5 de docs/ONBOARDING.md demandent un premier projet par un dossier, un clone ou une démonstration jetable, et un premier agent qui répond, avec sa première permission et son diff expliqués, sans rien envoyer de lui-même.

## Décision

Le parcours a six stations : une station « Premier agent » s'ajoute entre le premier projet et les statistiques, faite quand un agent a fini un tour sans erreur. La station du projet offre un dossier de la machine (recommandé), un clone par une adresse (vérifiée avant git, par les accès git de l'utilisateur, les dépôts des hébergeurs connectés proposés) et un projet de démonstration écrit par l'app sans réseau dans ~/.escouade/demo/bac-a-sable (un dépôt à lui sans remote, une liste de tâches en TypeScript, deux tests qui échouent exprès), plus les projets d'une équipe rejointe ; elle dit en une ligne ce que sont un projet, un agent, un worktree et une branche. La démonstration n'est retirée que par « Supprimer » sur sa bande, jamais d'elle-même (la conception ON-0 l'a tranché ainsi, contre « retiré quand on le quitte » de l'ébauche du ticket). La station de l'agent encadre sa vraie conversation, propose des messages selon le projet (« Lance les tests » seulement s'il a une commande de test qui peut tourner, en tête pour la démonstration), les met dans le compositeur sans jamais les envoyer, dit le quota consommé, explique la permission, le diff, où retrouver l'agent et comment l'arrêter ; la limite d'usage, hors ligne, pas connecté et l'échec disent quoi faire et laissent reprendre.

## Conséquences

Quatre commandes de plus, servies comme les autres commandes de chemins (onboarding_folder, onboarding_clone, demo_create, demo_remove), et la vue du parcours dit où vit la démonstration. Le compositeur accepte un texte qui remplace tout le champ (les messages proposés). L'explication de la permission dit les raccourcis réels de la carte (Ctrl+Entrée, Ctrl+Maj+Entrée), pas « 1, 2 ou 3 » des maquettes. Avec la découverte d'ON-3 (ESC-506) : aucune bulle tant que le premier lancement est à l'écran (le micro du compositeur encadré l'aurait déclenchée), la fin du parcours mène à la carte par « Voir ce qu'il y a à découvrir » à côté de « Commencer », et rien du premier projet, de la démonstration ni du premier agent ne coche une découverte. Les événements de statistiques du parcours restent pour ON-4 (ESC-507) : aucun n'est ajouté ici, la page de confidentialité ne change pas.
