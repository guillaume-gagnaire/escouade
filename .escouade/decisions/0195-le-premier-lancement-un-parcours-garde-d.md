---
number: 195
id: "le-premier-lancement-un-parcours-garde-d"
title: "Le premier lancement : un parcours gardé dans l'état, qui lit l'état réel et remplace les fenêtres du premier démarrage"
date: "2026-11-17"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-504"
pages: ["fenetre/premier-lancement","fenetre/barre-des-projets","fenetre/reglages"]
tickets: ["ESC-504"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/premier-lancement","added":65,"removed":0},{"page":"fenetre/barre-des-projets","added":5,"removed":3},{"page":"fenetre/reglages","added":3,"removed":0}]
accepted_at: "2026-11-17T12:20:12+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Jusqu'à la 2.2, un premier démarrage enchaînait trois fenêtres (le mode des nouveaux agents, les statistiques, le nom pour l'équipe), puis l'écran sans projet (Welcome) pour le harness et le compte. L'utilisateur a demandé un onboarding très didactique (2026-11-16) et choisi la direction C « La ligne » de docs/ONBOARDING.md (2026-11-17, ESC-503). ON-1 en pose le socle : où garder le parcours, quand l'ouvrir, comment il sait qu'une étape est faite, et ce qu'il écrit.

## Décision

Le premier lancement est un parcours plein écran (la ligne de stations de la direction C) : harness, compte, réglages de départ (mode des nouveaux agents, avec Bypass montré éteint), premier projet (forme courte), statistiques. Son état est gardé par installation dans state.json (version, début, fin, refait, ce que l'utilisateur a fait à chaque station : faite ou passée), jamais dans .escouade.yml. Il s'ouvre de lui-même sur une nouvelle installation seulement (des réglages sans mode et aucun projet), se reprend au démarrage suivant s'il a été laissé au milieu, se passe station par station ou en entier, et se refait depuis Aide › Premier lancement (nouvelle icône du rail) et Réglages › Général. Une station est faite d'après l'état réel (un compte ajouté, le mode choisi, un projet, une réponse aux statistiques), où qu'il ait été réglé, et reste faite ensuite. Chaque station écrit ce qu'écrivent les réglages, par les mêmes appels (engine_order_set, account_add_machine et la fenêtre « Ajouter un compte… », save_settings pour agentMode, telemetry_set) ; seul le plugin du harness choisi s'installe, depuis l'amorce. Une station qu'une stratégie impose est montrée faite et verrouillée, avec qui l'impose ; un harness ou un mode interdit est grisé et jamais recommandé. La fenêtre du mode (FirstModeModal) est retirée ; le nom pour l'équipe n'est plus demandé au premier lancement mais au premier partage (O8) ; les statistiques passées sont redemandées une seule fois. Les voies du git graph sont foncées en thème clair (O14).

## Conséquences

Un poste déjà réglé ne voit jamais le parcours de lui-même, même après la mise à jour. Le cœur a deux commandes de plus (onboarding_view, onboarding_act, servies ; un client distant n'a jamais le parcours ouvert) et l'état initial porte la vue du parcours. ON-1b (les branchements optionnels), ON-2 (démonstration, clone, premier agent), ON-3 (découverte, bande « Nouveau dans la 2.3 ») et ON-4 (événements de statistiques, site) s'y ajoutent comme stations ou pages ; ajouter une station se fait dans onboarding.rs (STEPS, real) et dans l'interface. Un poste qui avait fermé la question du mode sans répondre avant la 2.3 n'est pas pris pour neuf.
