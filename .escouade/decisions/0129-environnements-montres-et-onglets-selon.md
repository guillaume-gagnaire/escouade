---
number: 129
id: "environnements-montres-et-onglets-selon"
title: "Environnements montrés et onglets selon l'hébergeur de code"
date: "2026-11-08"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-390"
pages: ["kanban/environnements","kanban/kanban","git-fichiers/vue-pull-requests","fenetre/barre-laterale"]
tickets: ["ESC-390"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/environnements","added":14,"removed":7},{"page":"kanban/kanban","added":12,"removed":10},{"page":"git-fichiers/vue-pull-requests","added":4,"removed":1},{"page":"fenetre/barre-laterale","added":5,"removed":0}]
accepted_at: "2026-11-08T21:55:00+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Retour sur la 2.2.0-beta.2 : onglets de la barre latérale étroits, en-tête du Kanban où le sélecteur « Tickets | Environnements » chevauche la CI et les contrôles s'entassent, vue Environnements qui montre comme les autres un environnement sans déploiement d'application (github-pages jamais construit, npm qui compte 86 tickets « déployés »). L'onglet PR s'affichait toujours, même sans hébergeur.

## Décision

Direction A de design/kanban-envs-2-2, choisie sur aperçu par l'utilisateur le 2026-11-08 : onglets de la barre en mots seuls, l'actif en relief ; en-tête du Kanban sur deux rangées (titre, Importer, Pilote auto ; puis Tickets · Environnements soulignés, CI, places, Après validation) ; vue Environnements en couloirs sans cadre, l'état dans la tête de chaque couloir. Les onglets PR et Environnements n'existent que si la remote du projet est reconnue par un hébergeur de code dont le plugin est actif et qui lit, respectivement, des pull requests et des déploiements. La vue montre d'office les environnements où un déploiement a réussi ; un menu « N environnements sur M » coche et range les autres, choix gardé par projet sur la machine ; un ticket se place dans le dernier environnement montré qu'il a atteint.

## Conséquences

Nouvelle commande project_code_host et événement codeHosts (plugin changé, compte d'hébergeur connecté ou retiré) ; la fenêtre relit l'hébergeur à l'ouverture du projet et au retour au premier plan. Une remote changée hors d'Escouade ne se voit qu'au retour de la fenêtre. Le choix des environnements ne se partage pas avec l'équipe. Les tests de look qui comparaient l'en-tête et les onglets à l'aperçu de la 2.0 suivent désormais design/kanban-envs-2-2.
