# Les fonctionnalités, la température du cache et les propositions

Conception du 2026-10-28, faite avec l'utilisateur, pour une 2.x : trois idées de `docs/TODO-2.1.md` (« Les tokens chauds et froids », « Choisir ce que l'interface affiche », « Des réglages proposés au bon moment »), demandées par Zied. Les maquettes sont dans `design/features/`. Ce document dit ce que l'utilisateur a décidé, ce que le chef d'escouade a tranché (marqué **« choix du chef »**) et ce qui reste au constructeur.

## L'intention

Escouade a beaucoup grandi : Kanban, Docs, coût, statistiques, workflows, tâches programmées, Overseer, voix. Tout le monde ne veut pas tout, et personne ne fait le tour de tous les réglages. Trois réponses :

1. **Choisir ses fonctionnalités** : chacune s'active ou se coupe en entier, présentée une par une au premier lancement.
2. **Proposer une fonctionnalité coupée au moment où elle servirait**, plutôt que d'attendre qu'on la trouve dans les réglages.
3. **Voir l'état du cache de chaque agent**, d'un chiffre : un agent qui garde son cache coûte bien moins cher à relancer qu'un agent dont le cache a expiré.

## Ce qui n'en fait pas partie

- **Pas de réglage élément par élément** de l'interface (chaque jauge, chaque colonne) : on choisit des fonctionnalités entières (décidé, « un genre de feature flag »).
- **Pas de propositions de réglages fins** (modèle par défaut, branche de départ…) : seulement d'allumer une fonctionnalité coupée (décidé). La reprise automatique, que l'utilisateur juge indispensable, est déjà allumée par défaut (`auto_resume: true`).
- **Pas de garde-chaud automatique** du cache : le thermomètre montre, il n'envoie rien de lui-même (choix du chef : un message envoyé pour garder le cache coûterait un tour).

## 1. Les fonctionnalités (décidé)

- Des **interrupteurs de fonctionnalités entières**, comme des feature flags : Kanban, Espace Docs, **Éditeur** (décidé), Coût, Statistiques, Workflows, Tâches programmées, Overseer, Voix.
- **L'essentiel ne se coupe pas** (décidé) : les agents et la conversation, le panneau Git, les terminaux et les lancements.
- **Un assistant** au premier lancement, ou à la première mise à jour qui apporte ce choix : **une fonctionnalité par écran**, avec une capture de la vraie app, à quoi elle sert en trois points, et un grand interrupteur « Activer ».
- **« Passer » ne garde que l'essentiel** (décidé).
- **Après une mise à jour, ce qu'on utilise déjà reste allumé** (décidé), et l'écran le dit (« Tu l'utilises déjà sur 2 projets : il reste allumé »).
- **Réglages › Fonctionnalités** : les mêmes interrupteurs à tout moment, et « Revoir la présentation ».
- **Choix du chef** : une fonctionnalité coupée disparaît vraiment (son onglet, ses boutons, ses colonnes, ses menus, ses outils MCP qui la concernent) et ce qu'elle faisait en fond s'arrête ; ses données restent, et reviennent quand on la rallume.
- **Choix du chef** : une fonctionnalité nouvelle, apportée par une mise à jour, a son écran dans la fenêtre des nouveautés, avec le même interrupteur.
- Maquettes : `design/features/assistant.png`, `reglages-fonctionnalites.png`.

## 2. Proposer une fonctionnalité coupée au bon moment (décidé)

- Quand Escouade voit qu'une fonctionnalité coupée servirait, il la propose dans une **petite fenêtre en bas à droite**, comme Orca : ce qu'il a remarqué, ce que la fonctionnalité ferait, et « Activer », « Pas maintenant », « Ne plus proposer ».
- **Choix du chef**, les signaux de départ (le constructeur en ajoute s'ils sont sûrs) :
  - Kanban coupé : plusieurs agents lancés à la main sur des consignes longues, ou des tickets créés par le serveur MCP ;
  - Espace Docs coupé : une spec (`SPEC.md`, `docs/`) souvent modifiée, ou des documents glissés dans les conversations ;
  - Coût coupé : une limite d'usage atteinte ;
  - Workflows coupé : un projet qui a des skills, des commandes ou des agents (`.claude/`, `.agents/`) ;
  - Tâches programmées coupé : la même consigne relancée à la main à peu près à la même heure plusieurs jours de suite ;
  - Overseer coupé : des agents qui attendent une réponse sur plusieurs projets à la fois.
- **Choix du chef** : une seule proposition à la fois, au plus une par jour ; jamais pendant un tour visible ni pendant qu'on écrit ; « Pas maintenant » ne repropose pas la même avant une semaine ; « Ne plus proposer » est définitif (réversible dans Réglages › Fonctionnalités).
- **Choix du chef** : rien de ce qu'Escouade remarque ne sort de la machine.

## 3. La température du cache (décidé)

- Sur **chaque agent, quel que soit son harness**, un **chiffre en degrés** : **100 °C** quand le cache vient de servir, qui baisse avec le temps qui reste avant son expiration, jusqu'à **0 °C**. La **couleur du texte** suit : rouge-orangé au chaud, ambre à mi-course, bleu en refroidissant (décidé).
- **0 °C** aussi quand le cache est cassé (modèle changé, compaction, consigne système modifiée) : le prochain tour relira tout.
- **« — °C »** quand le harness ne dit rien de son cache : rien d'inventé.
- **Où** : sur la carte de l'agent, à côté du modèle, et dans l'en-tête de la conversation.
- **Choix du chef** : au survol de l'en-tête, le détail du dernier tour : le temps qui reste, les tokens lus du cache, mis en cache et lus au plein tarif, ce que le cache a fait économiser, et ce que coûterait le prochain message s'il partait à froid. Dans la barre d'état, ce que le cache a fait économiser aujourd'hui.
- **Choix du chef** : la température fait partie de la fonctionnalité « Coût » ; coupée, elle part avec.
- **Harness par harness** (décidé : « gérer ce cache pour chaque harness ») : la durée de vie et les chiffres viennent de chaque harness (Claude Code : `cache_read_input_tokens`, `cache_creation_input_tokens`, cache de 5 minutes ou d'1 heure ; Codex, OpenCode, Copilot : ce que leurs tours disent). Le constructeur vérifie, harness par harness, ce qui est lu vraiment ; une durée inconnue donne « — °C ».
- Comment ça marche, pour la doc : le modèle relit toute la conversation à chaque tour ; le début déjà lu est gardé en cache quelques minutes et relu pour ~5 à 10 % du prix ; chaque relecture remet le compteur à zéro ; après une pause plus longue que la durée de vie, tout est relu au plein tarif (exemple sur Opus 5.5 avec 100 k tokens de contexte : ≈ 0,02 $ au chaud, ≈ 0,40 à 0,50 $ à froid).
- Maquettes : `design/features/temperature.png`, `temperature-echelle.png` (retenues) ; `ecarte-thermometre-jauge.png` (la première idée, une jauge, écartée pour un chiffre).

## Ce qui est livré

- **§ 3, la température du cache** : ESC-398, ESC-421 (spec « Température du cache »).
- **§ 1, le socle (F-1, ESC-550)**, spec « Fonctionnalités » :
  - le registre unique, dans le cœur (`escouade_model::features`) : les neuf fonctionnalités, l'essentiel sans interrupteur, les Tâches programmées qui partent avec le Kanban ;
  - chaque interrupteur dans le registre des réglages (`features.<id>`), imposable par le poste géré et l'équipe (`features.kanban` … `features.voice` dans la liste unique des stratégies) ;
  - ce que chacune emporte quand on la coupe : la table complète est dans la spec. Ses vues, ses boutons, ses menus, ses raccourcis et `Ctrl+P` disparaissent ; ses outils MCP sont absents et refusés avec la raison ; ses découvertes partent ; ce qu'elle fait en fond s'arrête (pilote auto, import, tâches programmées, Overseer, lecture des workflows, serveurs de langage, écoute de la voix). Ses données restent ;
  - Réglages › Fonctionnalités (direction A), sans « Revoir la présentation » tant que l'assistant n'existe pas ;
  - la reprise après la mise à jour : ce qu'on utilise est relevé une fois (`featuresUsed`, avec le nombre de projets) et reste allumé. Le reste suit la valeur d'une installation neuve (décision 226), **tout allumé** jusqu'à l'assistant.
- **§ 1, l'assistant (F-2, ESC-559)**, spec « Fonctionnalités » et « Premier lancement » :
  - **une fonctionnalité par écran** : les fonctionnalités en pastilles (l'essentiel en tête, coché, jamais un bouton), une capture de la vraie app, à quoi elle sert en une phrase et trois points, et un grand interrupteur « Activer le Kanban » (un `switch`, décrit par ce qu'elle emporte). Dessous : ce qui est utilisé (« Tu l'utilises déjà sur 2 projets : il reste allumé »), « Part avec « Kanban » » pour les Tâches programmées, le cadenas et qui l'impose ;
  - **au premier lancement, une station de la ligne** (la 7ᵉ de 8), **après le premier agent et avant les statistiques** (choix du chef) : l'utilisateur a vu l'app au travail et choisit ce qu'elle montre autour des agents en le sachant ; le premier agent, le « aha » du parcours, n'arrive pas plus tard pour autant. « Suivant » d'écran en écran, « Continuer » au dernier, « Retour » revient d'un écran avant de revenir d'une station, ← → et les pastilles aussi ; sa « Plus tard » est « Passer : garder l'essentiel » ;
  - **à la mise à jour**, la fenêtre « Choisis tes fonctionnalités » s'ouvre d'elle-même à la première fenêtre de la version qui apporte le choix, sur une installation qui l'a mise à jour (`Onboarding.featuresDue`, posé par le relevé) ; jamais sur le parcours, jamais pour un client de l'API distante ; finie ou passée, plus jamais d'elle-même ;
  - **ses interrupteurs sont ceux des réglages** (enregistrés au geste, comme la page) : **« Passer » n'en change aucun**, donc il ne garde que l'essentiel et ce qui est utilisé, le reste étant coupé par `NEW_INSTALL` ; ce qui est utilisé est le relevé de la mise à jour (`featuresUsed`) et ce qui l'est maintenant (une source de tickets connectée compte pour le Kanban, la voix réglée pour la Voix) ;
  - **« Ce que tu veux brancher »** allume la fonctionnalité de ce qu'on y choisit (choix du chef) : choisir une source de tickets allume le Kanban, choisir la voix (Whisper ou un service) allume la Voix, puisqu'une installation neuve les a coupées avant l'assistant ;
  - **`NEW_INSTALL` passe à `false`** (décision proposée « Fonctionnalités : l'assistant fait le choix… ») : une installation neuve a tout coupé sauf l'essentiel jusqu'à la station, une installation mise à jour garde allumé ce qu'elle utilise et coupe le reste jusqu'à la fenêtre de la mise à jour. Seule la constante a changé dans le cœur ;
  - **« Revoir la présentation »** dans Réglages › Fonctionnalités ferme les réglages et rouvre l'assistant, avec « Fermer la présentation » ;
  - **la fenêtre des nouveautés** (celle d'une mise à jour installée, « Voir les nouveautés ») donne son écran et le même interrupteur à une fonctionnalité qu'une mise à jour apporte (`Onboarding.featuresKnown` : les fonctionnalités rencontrées ; une nouvelle n'y est pas encore et y vient en allumée, comme un interrupteur absent du fichier). Aucune aujourd'hui : la 2.3 apporte les neuf ensemble, présentées par l'assistant ;
  - **les captures** viennent de la vraie app, refaites par un script : `npm run site-shots -- --grep "features"` (`tests/site-shots/features-tour.spec.ts`, sur la démo des captures du site), 9 fonctionnalités × 2 thèmes × 2 langues en JPEG 960 × 540 dans `public/features/<fr|en>/` ;
  - la station a sa valeur dans l'entonnoir des statistiques : `onboarding_step` avec `step: features` (page de confidentialité du site mise à jour), aucun événement de plus.
- **§ 2, les propositions au bon moment (F-3, ESC-558)**, spec « Fonctionnalités » (« Proposer une fonctionnalité coupée au bon moment ») et décision proposée « Fonctionnalités : proposer une fonctionnalité coupée au bon moment, avec ses seuils… » :
  - la petite fenêtre en bas à droite (`src/components/FeatureOffer.svelte`), dans la langue des bulles d'ESC-506 : ce qu'Escouade a remarqué, ce que la fonctionnalité ferait, « Activer », « Pas maintenant », « Ne plus proposer » ; captures dans `design/features/captures/proposition-fonctionnalite-*.png` ;
  - les six signaux et leurs seuils, constantes de `src-tauri/src/offers.rs` (2 tickets refusés par le serveur MCP ou 3 agents sur une consigne de 300 caractères en 7 jours ; la spec changée sur 3 jours des 14 derniers ou 3 documents glissés en 14 jours ; 1 limite d'usage en 7 jours ; 2 skills, commandes ou agents dans un projet ; la même consigne 3 jours de suite dans une heure ; des agents en attente sur 3 projets à la fois) ;
  - les règles : une à la fois, une par 24 heures, jamais pendant un tour à l'écran, une permission, la frappe ; « Pas maintenant » une semaine ; « Ne plus proposer » réversible dans Réglages › Fonctionnalités › « Propositions écartées » ; une fonctionnalité imposée jamais proposée ;
  - une seule chose au bon moment par session avec les bulles d'« À découvrir » : la bulle passe d'abord ;
  - rien ne sort : ce qui est remarqué est dans `state.json` (`Onboarding::offers`), deux semaines, sans texte ; trois commandes de la liste unique pour l'API distante (`feature_offer`, `feature_offer_act`, `feature_offers`).

## Ce qui reste au constructeur

- Des signaux de plus pour les propositions (F-3), s'ils sont sûrs : l'Éditeur, les Statistiques et la Voix n'en ont pas encore, et ne sont donc jamais proposés.
- La courbe de la température (linéaire sur la durée de vie, ou plus fine) et ses couleurs, accessibles dans les deux thèmes.
