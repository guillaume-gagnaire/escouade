---
title: "Premier lancement"
subject: "fenetre"
sources: ["docs/ONBOARDING.md","design/onboarding-2-3/README.md","design/onboarding-2-3/maquette.html","design/onboarding-2-3/captures/c-harness.png","design/onboarding-2-3/captures/c-modes-clair-1024.png"]
tickets: ["ESC-503","ESC-504"]
decisions: [195]
compared: null
---

## Ce que c'est

Le parcours qui règle Escouade sur un poste neuf, une question à la fois : le harness, son compte, le mode des nouveaux agents, le premier projet et l'accord des statistiques. Il prend toute la fenêtre, sauf la barre de titre et la barre de statut, et dessine ses stations comme les commits du git graph (direction C « La ligne » de `docs/ONBOARDING.md`). Il ne règle rien à sa façon : chaque station écrit ce qu'écrivent les réglages, par les mêmes appels, et chacune se passe.

## Comportement

### Quand il s'ouvre

- **De lui-même**, au premier démarrage d'une **nouvelle installation** seulement : des réglages sans mode pour les nouveaux agents (SEC-09 : un `settings.json` d'avant reçoit `auto` à son chargement) et aucun projet. Jamais sur un poste déjà réglé, même sans compte, même après une mise à jour.
- **Repris** : fermé au milieu (la fenêtre, l'app, une coupure), il rouvre au démarrage suivant, à la première station ni faite ni passée. Fini ou « Tout passer », il ne revient plus de lui-même.
- **Refait** : Aide › Premier lancement (l'icône Aide du rail) et Réglages › Général › « Refaire le premier lancement » le rouvrent sur n'importe quel poste. Toutes les stations s'y montrent depuis la première, avec la valeur en place sur la ligne (« Claude Code », « Principal », « Demander »). Rien n'est remis à zéro : refaire ne retire ni compte ni projet. Il se ferme par « Fermer ».
- Un client de l'API distante ne le voit jamais.

### L'état gardé

- Par installation, dans `state.json` (l'état durable, ESC-469), jamais dans `.escouade.yml` : la version du parcours, quand il a commencé et fini, s'il est refait, et ce que l'utilisateur a fait à chaque station (faite, passée).
- **Le parcours lit l'état réel** : une station est faite quand ce qu'elle règle l'est, où que ce soit (un compte ajouté dans Réglages coche celle du compte). Une station faite le reste, même si l'état change ensuite : un compte retiré ne rouvre rien.
  - harness et compte : un compte ajouté ;
  - réglages de départ : le mode des nouveaux agents choisi ;
  - premier projet : un projet ouvert ;
  - statistiques : l'accord donné ou refusé.
- Le cœur relit la vue à chaque changement de ces réglages, des projets ou des réglages imposés.

### La fenêtre

- **À gauche, la ligne** (une `nav`, « Les stations du premier lancement ») : « Premier lancement », « 5 stations · environ 2 minutes », puis une station par question. Faite : un nœud plein et sa valeur ; en cours : teal, « maintenant » (`aria-current="step"`) ; passée : creuse, « passé · à reprendre » ; imposée : pleine, « imposé » ; à faire : en pointillés. Chaque station est un bouton qui y mène. Sous la ligne, les voies de la découverte (« Ensuite : À découvrir », Planifier · Coder · Relire · Livrer · Piloter, décoratives) et, en bas, « Tout se passe et se reprend. Le parcours revient depuis Aide › Premier lancement. » avec « Tout passer » (« Fermer » quand il est refait).
- **À droite, la station** : « Station 2 · le compte », sa question (le `h1`, qui prend le focus à chaque station) et son pourquoi (sa description), ses choix (un groupe radio, le choix recommandé déjà pris, « Recommandé » en pastille).
- **Le pied** : « Retour », « ↑ ↓ choisir · Entrée valider » (caché en 1024), « Plus tard » et le bouton principal (« Continuer », « Garder Édits auto »…).
- **Au clavier** : ↑ ↓ vont d'un choix à l'autre en sautant ceux qui ne se prennent pas (depuis la question, ↑ ↓ amènent aux choix), Espace prend le choix qui a le focus, Entrée fait le bouton principal, Échap fait « Plus tard ». Rien de cela tant qu'une fenêtre est ouverte par-dessus (« Ajouter un compte… », l'assistant d'un projet).
- **Tailles** : 1440 × 900 et 1024 × 728, sans défilement en travers ; en 1024, la ligne passe à 248 px et la station se resserre. Une station trop haute défile en elle-même.
- **Animation** : l'arrivée d'une station monte de 8 px en 240 ms ; avec « réduire les animations », un fondu seulement.
- Thèmes clair et sombre ; les voies du thème clair sont foncées pour tenir 3:1 sur le cadre et le panneau (choix O14), le git graph en profite.

### Station 1, le harness

- « Avec quel harness tes agents travaillent-ils ? » — « Un harness est le programme d'agent de code de ta machine. Escouade le lance et le pilote ; il garde ta connexion et tes réglages. »
- Chaque harness qu'Escouade connaît, avec sa version et son état : « Connecté · ada@atlas.dev » (son compte de la machine connecté, ou un compte déjà ajouté), « Installé, pas connecté » (« Tu t'y connecteras dans un terminal, juste après. »), « Trop ancien », « Version non prise en charge », « Introuvable ». Les prêts d'abord, puis les installés, puis les autres, dans « Ordre des harness ».
- **Recommandé** : le premier connecté, sinon le premier trouvé. Un harness trop ancien ne se prend pas.
- **Introuvable** : choisi, il montre sa commande d'installation (celle du manifeste de son plugin, Windows ou Unix) et « Copier » (« Commande copiée »), avec la ligne du plugin tiers qui la donne (SEC-30). Aucun trouvé : « Aucun harness n'est installé. Copie la commande de celui que tu veux, lance-la dans un terminal : Escouade le trouvera dès que tu reviens. » « Continuer » attend un harness trouvé ; « Chercher à nouveau » relit le PATH (« Cherché dans le PATH », et WSL), et la fenêtre revenue au premier plan aussi.
- **Ce que « Continuer » écrit**, comme Réglages › Comptes et harness : le harness en tête de « Ordre des harness » (« Le harness choisi passe en tête de l'ordre des harness : c'est celui des nouveaux projets. ») ; s'il est connecté sur la machine et sans compte, son compte de la machine, ajouté comme « Ajouter un compte… » l'ajoute (la station du compte se fond alors dans celle-ci : « Ses agents partent sur le compte déjà connecté ; un compte à part se crée dans Réglages › Comptes et harness. ») ; sinon son plugin seul.
- **Les plugins** : seul le plugin du harness choisi s'installe, depuis l'amorce, sans réseau (le tri de l'amorce), et la station le dit avant : « Escouade installe le plugin de Codex, depuis l'app, sans réseau. »

### Station 2, le compte

- « Quel compte pour Claude Code ? » — les quotas, les statistiques et la connexion sont tenus par compte ; d'autres s'ajoutent ensuite.
- « Le compte de la machine » (avec son e-mail quand il est connecté ; « Son dossier habituel, tel quel ») ou « Un compte à part » (« Un dossier à lui sous ~/.escouade, et sa propre connexion »). Le compte de la machine ne se prend pas quand il est hors de portée d'Escouade (Gemini CLI) ou déjà ajouté.
- « Le compte de la machine » l'ajoute ; pas encore connecté, sa connexion suit dans la fenêtre « Se connecter… » (son programme dans un terminal). « Un compte à part » ouvre « Ajouter un compte… » ; fermée, la fenêtre revient au parcours, et le compte fait coche la station.
- Un compte déjà là : « Compte ajouté : Principal », et la station est faite.

### Station 3, les réglages de départ

- « Quand tes agents doivent-ils te demander ? » — « Le mode des nouveaux agents ; tu le changes pour chacun, sous son message. Modèle Sonnet, effort moyen : ceux de ton compte. » Le modèle et l'effort sont dits, pas demandés : ceux du compte (Claude Code : Sonnet, moyen). Un harness sans modèle à lui : « Effort moyen : celui de ton compte, avec le modèle par défaut de ton harness. »
- Les quatre modes de SEC-09, chacun en une phrase (les textes de la question d'avant) : Édits auto (recommandé), Demander, Plan, Auto. Un mode que le harness n'a pas est grisé (« Codex n'a pas de mode Auto. »), un mode qu'une stratégie interdit aussi, avec sa raison.
- **Bypass** est montré, éteint, jamais proposé : « Éteint : l'agent ferait tout sans rien te montrer, même ce qu'un contenu piégé lui dicterait. À réserver à une machine jetable ; il s'allume dans Réglages › Agents. » Déjà permis : « Permis dans Réglages › Agents : jamais proposé comme mode de départ, il se choisit agent par agent. »
- Un harness qui ne tourne qu'en Bypass le dit (ESC-489) : « Mistral Vibe ne tourne qu'en Bypass : ses agents agiront sans demander. »
- « Garder Édits auto » écrit « Mode par défaut des agents » (`agentMode`), comme Réglages › Agents.

### Station 4, le premier projet

- « Où travaille ton premier agent ? », avec ce qu'est un projet, un worktree et une branche. Ici, sa forme courte : « Un dossier de ta machine », et « Ajouter un projet… » ouvre l'assistant « Nouveau projet » tel quel. Un projet ouvert : « Projet ouvert : demo-api ». La démonstration, le clone, les projets d'une équipe et la station du premier agent viennent avec ON-2 (ESC-505).

### Station 5, les statistiques

- La question de toujours (« Aider à améliorer Escouade ? », ses textes, son lien « Ce qui est envoyé, en détail »), le même réglage, plus : « Tes projets, ton code, tes conversations et tes comptes restent sur ta machine, dans ~/.escouade. » « Non merci » et « Partager les statistiques » ont le même poids. Répondre finit le parcours.
- Passées, elles sont redemandées **une fois** au démarrage suivant, dans la fenêtre d'avant, puis plus jamais. Une installation qui n'a jamais vu le parcours garde la règle d'avant : redemandées à chaque démarrage tant qu'elles n'ont pas de réponse.

### La fin

- « Escouade est prêt » — « Le reste se découvre en le faisant : la voix, le Kanban, les PR… », et « 2 étapes passées : elles t'attendent dans Aide › Premier lancement. ». « Commencer » ferme le parcours, sur le premier projet s'il y en a un.

### Ce qui a changé autour

- Le parcours remplace l'enchaînement du premier démarrage : la fenêtre du mode (`FirstModeModal`, retirée), puis celle des statistiques, puis celle du nom pour l'équipe. Le nom n'est plus demandé au premier lancement : il l'est au premier partage (choix O8).
- L'écran sans projet (`Welcome`) reste celui d'après le parcours, et celui d'un poste réglé.

### Réglages imposés

- Une station qu'une stratégie impose est faite et verrouillée, avec qui l'impose (« Imposé par la stratégie de ce poste », « Imposé par l'équipe Acme ») : le mode des nouveaux agents (`permissions.defaultMode`), les statistiques (`telemetry.enabled` : « Éteintes par la stratégie de ce poste. »). Elle se traverse en un « Continuer ».
- Un harness que `harnesses.allowed` ne permet pas est grisé, « non permis par la stratégie de ce poste », jamais recommandé ; un seul permis : « Codex est le seul harness permis sur ce poste. » Un mode que `permissions.modes` refuse est grisé avec sa raison ; « Autoriser le mode Bypass » imposé est dit sous Bypass.
- Le parcours ne contourne jamais un refus du cœur : il montre ce que le cœur refuserait.

### L'aide

- L'icône « Aide » du rail, au-dessus des stats (et au premier démarrage, entre `+` et « Réglages ») ouvre un menu : « Premier lancement » (le parcours refait), « Documentation » (le site, dans la langue de l'interface), « Signaler un problème » (les tickets du dépôt sur GitHub). « À découvrir » s'y ajoutera avec la découverte (ON-3, ESC-506).

## Réglages

- Aucun réglage à lui : il écrit `engineOrder`, les comptes, `agentMode` et `telemetry`, comme leurs pages.
- Réglages › Général › Toi › « Premier lancement » › « Refaire le premier lancement ».

## Limites

- La bande « Nouveau dans la 2.3 » d'un poste mis à jour et la découverte (Aide › À découvrir) sont d'ON-3 ; la démonstration, le clone et le premier agent d'ON-2 ; les fonctionnalités à brancher (sources de tickets, voix) d'ON-1b (ESC-512) ; les événements de statistiques du parcours d'ON-4 (ESC-507).
- Une invitation d'équipe ouverte au premier lancement n'est pas encore une station.
- Un poste qui a fermé la question du mode sans répondre avant la 2.3 (son mode vide, des projets) n'est pas un poste neuf : le parcours ne s'ouvre pas, le mode recommandé reste en vigueur.
