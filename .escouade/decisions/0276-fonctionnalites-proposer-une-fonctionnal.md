---
number: 276
id: "fonctionnalites-proposer-une-fonctionnal"
title: "Fonctionnalités : proposer une fonctionnalité coupée au bon moment, avec ses seuils, et la bulle avant la proposition"
date: "2026-11-20"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-558"
pages: ["fenetre/fonctionnalites","fenetre/a-decouvrir","fenetre/reglages"]
tickets: ["ESC-558"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"fenetre/fonctionnalites","added":43,"removed":4},{"page":"fenetre/a-decouvrir","added":2,"removed":0},{"page":"fenetre/reglages","added":1,"removed":0}]
accepted_at: "2026-11-21T22:12:57+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

docs/FEATURES.md § 2 (décidé avec l'utilisateur le 2026-10-28, avec les choix du chef) veut qu'Escouade propose une fonctionnalité coupée quand elle servirait, dans une petite fenêtre en bas à droite, avec « Activer », « Pas maintenant » et « Ne plus proposer ». Les signaux de départ étaient dits sans seuils, et l'ordre face aux trois bulles au bon moment d'ESC-506 (O11 : une seule chose au bon moment par session) restait à décider. Le socle F-1 (ESC-550) donne les interrupteurs, le registre et les verrous des stratégies.

## Décision

**La fenêtre** : en bas à droite, au-dessus de la barre d'état, un dialogue non modal qui ne prend pas le focus : « Fonctionnalité coupée » et le nom, « Ce qu’Escouade a remarqué » (une phrase chiffrée), « Ce qu’elle ferait », « Ce qu’Escouade remarque reste sur ta machine. », puis « Activer », « Pas maintenant », « Ne plus proposer ». Échap, le focus dedans, vaut « Pas maintenant ».

**Les seuils** (au seuil : proposée ; juste en dessous : non) :
- Kanban : 2 appels de `create_ticket` refusés par le serveur MCP en 7 jours ; ou 3 agents de l'utilisateur lancés à la main en 7 jours sur un premier message de 300 caractères ou plus.
- Espace Docs : `SPEC.md` ou `docs/` changés par les commits d'un projet sur 3 jours différents des 14 derniers ; ou 3 documents (pdf, doc(x), odt, rtf, md, txt, ppt(x), odp, xls(x), ods, csv) glissés dans les conversations en 14 jours.
- Coût : 1 limite d'usage atteinte en 7 jours.
- Workflows : 2 skills, commandes ou agents dans un même projet (`.claude/` ou `.agents/` : `skills/<nom>/SKILL.md`, `commands/*.md`, `agents/*.md`).
- Tâches programmées : la même consigne (mêmes mots, 20 caractères ou plus) envoyée à la main 3 jours de suite, toutes dans une heure de la journée ; proposée seulement Kanban allumé.
- Overseer : des agents qui attendent une réponse sur 3 projets à la fois, vus dans les 7 derniers jours.
- Éditeur, Statistiques, Voix : pas de signal, jamais proposées.
Plusieurs à la fois : l'ordre du registre.

**Le rythme** : une à la fois ; au plus une par 24 heures ; jamais pendant un tour d'un agent à l'écran, une demande de permission, la frappe (1,5 s), une fenêtre, le premier lancement ou une bulle ; « Pas maintenant » : pas la même avant 7 jours ; « Ne plus proposer » : définitif, réversible par « Reproposer » dans Réglages › Fonctionnalités › « Propositions écartées ».

**L'ordre avec les bulles d'ESC-506** : une seule chose au bon moment par session. La bulle passe d'abord (elle naît d'un geste fait maintenant et se perdrait ; la proposition naît d'un signal gardé et attend la session suivante). Une bulle à l'écran ou déjà montrée : pas de proposition dans la session ; une proposition montrée : plus de bulle.

**Rien ne sort** : ce qui est remarqué est gardé dans `state.json` deux semaines au plus, sans texte (une consigne par son empreinte), sans statistique. Une fonctionnalité imposée (poste géré, équipe), ou celle dont elle dépend, n'est jamais proposée ; « Activer » est alors refusé par le cœur.

**À distance** : trois commandes de la liste unique, `feature_offer`, `feature_offer_act`, `feature_offers` ; un client montre les propositions de son cœur.

## Conséquences

- Le cœur note les signaux (le refus d'un outil MCP, l'envoi d'un message à la main, la limite d'usage, les agents en attente lus à chaque demande) et lit la spec par `git log` (une fois par heure par projet) et les dossiers `.claude/`, `.agents/`, seulement pour les fonctionnalités coupées.
- `Onboarding` a un champ de plus (`offers`) dans `state.json` ; un ancien fichier le lit vide.
- Les seuils sont des constantes de `src-tauri/src/offers.rs`, chacune testée au seuil et juste en dessous ; en changer un change cette décision.
- La proposition et la bulle se partagent le drapeau de la session de la fenêtre (`discover.svelte.ts`).
- Le signal de l'Overseer n'est lu que quand la fenêtre redemande (chaque minute) : une attente plus brève peut passer.
