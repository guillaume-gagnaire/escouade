---
title: "Fonctionnalités"
subject: "fenetre"
sources: ["docs/FEATURES.md § 1 et § 2 (conception du 2026-10-28, choix du chef)","design/features/reglages-fonctionnalites.png","design/features/assistant.png","design/features/captures/proposition-fonctionnalite-clair-1440.png","docs/ONBOARDING.md § 7 (les découvertes)","crates/escouade-model/src/features.rs (le registre)","src-tauri/src/features.rs (ce que chacune emporte dans le cœur)","src-tauri/src/offers.rs (les propositions, leurs signaux et leurs seuils)","src/lib/features.ts (dans la fenêtre)","src/lib/offers.svelte.ts (les propositions dans la fenêtre)","docs/FEATURES.md § 1 (conception du 2026-10-28, choix du chef)","docs/ONBOARDING.md § 7 (les découvertes) et § 4.5 bis (la station des fonctionnalités)","crates/escouade-model/src/features.rs (le registre, NEW_INSTALL)","src-tauri/src/features.rs (ce que chacune emporte dans le cœur, l'assistant)","src/components/features/FeatureTour.svelte (l'assistant)","tests/site-shots/features-tour.spec.ts (les captures de l'assistant)"]
tickets: ["ESC-550","ESC-558","ESC-559","ESC-701"]
decisions: [226,276,280,"la-2-2-11-a-la-2-2-19-reversees-dans-la"]
compared: null
---

# Fonctionnalités

## Ce que c'est

Escouade a grandi : Kanban, Docs, éditeur, coût, statistiques, workflows, tâches programmées, Overseer, voix. Tout le monde ne veut pas tout. Chaque **fonctionnalité entière** s'allume ou se coupe d'un interrupteur, comme un feature flag. Une fonctionnalité coupée **disparaît vraiment** : ses vues, ses boutons, ses menus, ses raccourcis, sa place dans la palette, ses outils MCP, ses découvertes. **Ce qu'elle faisait en fond s'arrête.** **Ses données restent** et reviennent telles quelles quand on la rallume. Quand une fonctionnalité coupée servirait, Escouade la **propose au bon moment**, une à la fois (ci-dessous).

**L'essentiel ne se coupe pas** : les agents et leur conversation, le panneau Git, les terminaux et les lancements. Il n'a pas d'interrupteur.

## Le registre

Un seul registre, dans le cœur (`escouade_model::features`), dit les neuf fonctionnalités, dans cet ordre : **Kanban**, **Éditeur**, **Espace Docs**, **Coût** (la température du cache comprise), **Statistiques**, **Workflows**, **Tâches programmées**, **Overseer**, **Voix**.

- Chacune a son id (`kanban`, `editor`, `docs`, `cost`, `stats`, `workflows`, `scheduled`, `overseer`, `voice`). Son interrupteur est dans les réglages de l'app (`features.<id>` de `settings.json`). Il passe par le registre des réglages : une rangée par fonctionnalité (`featureKanban`…), qu'on exporte avec les réglages et que la recherche trouve.
- **Les Tâches programmées partent avec le Kanban** : leurs passages sont des tickets. Kanban coupé, elles sont coupées aussi, quel que soit leur propre interrupteur, qui reste tel quel.
- L'état vient du cœur. La fenêtre le lit dans les réglages qu'il lui donne. Le cœur refuse ce qu'une fonctionnalité coupée ferait, quel que soit le client : fenêtre, MCP, API distante.

## Ce que chacune emporte quand on la coupe

| Fonctionnalité | Vues, boutons, menus, raccourcis, palette | Outils MCP (absents de la liste, refusés avec la raison) | Découvertes d'« À découvrir » | Ce qui s'arrête en fond | Ce qui reste et revient |
| --- | --- | --- | --- | --- | --- |
| **Kanban** | L'onglet « Kanban » et le Kanban (ses colonnes, ses cartes), les pages « Kanban » et « Tickets importés » des réglages d'un projet, les étapes Kanban et Tickets importés de l'assistant d'un nouveau projet. Un renvoi vers le Kanban (boîte « À toi », Vue d'ensemble, Overseer, une PR) n'ouvre rien. | `get_board`, `set_board`, `list_tickets`, `get_ticket`, `get_ticket_file`, `get_ticket_image`, `create_ticket`, `update_ticket`, `move_ticket`, `delete_ticket`, `start_ticket`, `approve_ticket`, `reject_ticket`, `resume_ticket`, `resolve_conflict`, `dismiss_block`, `wait_ticket`, `split_ticket` | Écrire un ticket, Le pilote auto, Valider un ticket, Les sources de tickets | Aucun ticket ne démarre, ni par le pilote auto ni à la main. L'import automatique des sources s'arrête. Les PR, la CI et les environnements des tickets ne sont plus relus. Un agent de ticket déjà au travail finit, car c'est un agent. | Les tickets, leurs colonnes, leurs fichiers et leurs images, les réglages du Kanban. Rallumé, le pilote auto repart. |
| **Éditeur** | Le bouton « Éditeur » de la barre de titre, la vue de l'éditeur (et son volet dans l'écran partagé), `Ctrl+P` (la palette des fichiers), « Ouvrir dans l'éditeur » des menus d'un agent et d'un fichier, le bouton d'un fichier modifié, les liens des fichiers dans la conversation (le chemin reste écrit), « Parcourir » du dossier du projet. Dans Réglages › Général › Avancé : « Taille du texte de l'éditeur », « Serveurs de langage », « Extensions permises de l'éditeur ». | aucun | L'éditeur | Aucun serveur de langage ne démarre. Ceux qui tournent s'arrêtent. | Les réglages de l'éditeur, ses extensions et ses serveurs de langage. |
| **Espace Docs** | L'onglet « Docs » et ses espaces (spécification, décisions, documents, discussions). « Branche des docs » de Réglages › Projet › Git et worktrees. Un renvoi vers une page de la spec, une décision ou un document n'ouvre rien. | `get_spec_tree`, `read_spec_page`, `write_spec_page`, `search_spec`, `list_decisions`, `read_decision`, `propose_decision`, `replace_decision`, `list_documents`, `read_document`, `search_documents`, `add_document`, `get_docs_to_share`, `share_docs` | La spec et les décisions | Aucun index des documents n'est tenu à jour (il ne l'est qu'à la demande des outils, refusés). | Les pages de la spec, les décisions, les documents, les discussions, la branche des docs. |
| **Coût** | Le coût du jour dans la barre d'état et ce que le cache a fait économiser. Le coût sur les cartes des agents, des agents finis et des tickets, et dans la Vue d'ensemble. Le coût d'un tour et celui de l'en-tête de la conversation. La **température du cache** (cartes et en-tête). Le coût du jour d'un harness sans quota. Dans Comptes et harness : « Économies du cache dans la barre d'état ». Une carte montre alors les tokens à la place. | aucun | — | Rien : le coût n'a pas de travail en fond. | Les coûts restent comptés et réapparaissent entiers. Le budget par jour d'un compte garde son effet sur le pilote auto. |
| **Statistiques** | La tuile « Stats » du rail et la vue des statistiques. Ouverte, elle se ferme pour le projet. | aucun | — | Rien (elle n'a pas de travail en fond). Elle n'a rien à voir avec la télémétrie (« Statistiques d'usage »). | Les chiffres restent comptés. |
| **Workflows** | L'onglet « Workflow » du panneau latéral (« Fichiers » prend sa place), le groupe « Workflow » de Réglages › Projet › Kanban, l'étape du workflow sur les cartes. | `get_workflow` | — | Aucun workflow n'est lu ni suivi, ni au démarrage, ni au retour de la fenêtre au premier plan, ni à la demande. | Les fichiers des workflows et les corrections. Rallumés, ils sont relus. |
| **Tâches programmées** | La section « Programmées » de la barre latérale. | `list_scheduled_tasks`, `create_scheduled_task`, `update_scheduled_task`, `pause_scheduled_task`, `delete_scheduled_task` | — | Aucune tâche ne part. Chaque échéance passe, et sa ligne le dit : « La fonctionnalité Tâches programmées était coupée à la dernière échéance : la tâche attend la suivante. » Rallumée, une tâche part à sa prochaine échéance, pas pour celles qui sont passées. | Les tâches, leur rythme, leur historique. |
| **Overseer** | Le bouton « Overseer » de la barre de titre, son écran (ouvert, il se ferme), la page Réglages › Overseer, son raccourci global. | `add_directive`, `list_directives`, `revoke_directive`, `undo_action`, `leave_to_user` | — | L'Overseer s'arrête, même allumé dans ses réglages : son tour en cours est interrompu, puis son process s'arrête. Aucun fait ne le réveille plus. | Son journal, ses consignes, sa mémoire, ses réglages (« Activer l'Overseer » compris). |
| **Voix** | Le micro de chaque champ (composer, terminal, ticket, boîte « À toi »), le bandeau de dictée, le bouton de la conversation vocale, le raccourci de la dictée, la page Réglages › Voix. | aucun | Dicter un message, Parler à un agent | Rien n'écoute : la dictée et la conversation vocale sont refusées à leur départ. Celles en cours s'arrêtent. | Les réglages de la voix, les modèles téléchargés, les clés des services en ligne. |

Un appel d'un outil d'une fonctionnalité coupée, par un client qui avait lu la liste avant, est refusé avant de tourner. La raison est dite : « La fonctionnalité « Kanban » est coupée (Réglages › Fonctionnalités) : ce qu’elle fait n’est pas disponible. L’utilisateur peut la rallumer, ses données sont gardées. » Pour une fonctionnalité imposée : « … est coupée par la stratégie de ce poste : ce qu’elle fait n’est pas disponible sur ce poste. » Le journal de l'activité MCP le compte comme refusé. La liste des outils n'est jamais gardée par le client (TTL 0).

## Réglages › Fonctionnalités

Une page d'Escouade de la direction A, juste après Général : « Fonctionnalités », « Ce qu’Escouade te montre : chaque fonctionnalité s’allume ou se coupe en entier. » (maquette `design/features/reglages-fonctionnalites.png`).

- **En tête**, « Agents et conversation » : « L’essentiel, toujours là : les agents et leur conversation, le panneau Git, les terminaux et les lancements. » Son interrupteur est allumé et grisé.
- **Puis une ligne par fonctionnalité**, dans l'ordre du registre. Chaque ligne a son icône, son nom, à quoi elle sert en une phrase, puis ce qu'elle emporte, par exemple : « Coupé : son onglet et ses outils MCP disparaissent. Les pages, décisions et documents restent. » Son interrupteur est à droite et s'enregistre au geste ; la phrase de ce qu'elle emporte le décrit pour un lecteur d'écran.
- **Tâches programmées**, Kanban coupé : son interrupteur est grisé, avec « Part avec « Kanban » : rallume-le d’abord. »
- **Une fonctionnalité imposée** est grisée, avec le cadenas et qui l'impose (« Imposé par la stratégie de ce poste », « Imposé par l’équipe Acme »).
- **Ce que l'installation utilisait** au moment de la mise à jour est dit sur la ligne : « Utilisée sur 2 projets », « Utilisée ».
- En pied : « Une fonctionnalité coupée disparaît vraiment, ce qu’elle faisait en fond s’arrête ; ses données restent et reviennent quand tu la rallumes. »
- **« Propositions écartées »** (ESC-558), après le pied : « Ce que tu as choisi de ne plus te voir proposer quand ça servirait. » Une ligne par fonctionnalité écartée par « Ne plus proposer », son icône et son nom, avec « Reproposer » (« Reproposer « Kanban » » pour un lecteur d'écran), qui la rend aux propositions. Sans aucune : « Aucune : Escouade peut te proposer une fonctionnalité coupée au moment où elle servirait, une à la fois, au plus une par jour. »
- **Les pages d'une fonctionnalité coupée** quittent la barre et la recherche des réglages : Overseer, Voix, et pour un projet Kanban et Tickets importés. Les rangées d'une autre page qu'elle emporte en quittent la recherche aussi.
- En pied aussi, **« Revoir la présentation »** rouvre l'assistant (voir *L'assistant* plus bas).

## Proposer une fonctionnalité coupée au bon moment (ESC-558)

Quand Escouade voit qu'une fonctionnalité coupée servirait, il la propose dans une **petite fenêtre en bas à droite**, au-dessus de la barre d'état, dans la langue des bulles d'« À découvrir » (capture `design/features/captures/proposition-fonctionnalite-clair-1440.png`) :

- en tête, l'icône de la fonctionnalité, « Fonctionnalité coupée » et son nom ;
- **« Ce qu’Escouade a remarqué »** : une phrase, avec le chiffre, par exemple « 2 tickets demandés au serveur MCP cette semaine ont été refusés : le Kanban est coupé. » ;
- **« Ce qu’elle ferait »** : une phrase, par exemple « Le Kanban en ferait des tickets : chacun son agent, son worktree et ses critères, lancés par le pilote auto. » ;
- « Ce qu’Escouade remarque reste sur ta machine. » ;
- **« Activer »**, **« Pas maintenant »**, **« Ne plus proposer »** (son infobulle : « Tu pourras la reproposer dans Réglages › Fonctionnalités. »).

C'est un dialogue non modal : il ne prend pas le focus, il est annoncé poliment au lecteur d'écran, et Échap, le focus dedans, vaut « Pas maintenant ». Il reste dans son coin jusqu'à sa réponse ; il ne gêne rien (la frappe continue dans le champ où elle est). Les textes sont en français et en anglais (« Turn on », « Not now », « Don’t suggest again »).

### Les signaux et leurs seuils

| Fonctionnalité coupée | Ce qu'Escouade remarque | Seuil (au seuil : proposée ; juste en dessous : non) |
| --- | --- | --- |
| **Kanban** | Des appels de `create_ticket` au serveur MCP, refusés parce que le Kanban est coupé. | **2 en 7 jours** (1 : non). |
| **Kanban** | Des agents de l'utilisateur (ni d'un ticket, ni d'une discussion des Docs, ni créés par un autre agent) lancés à la main sur un premier message long. | **3 agents en 7 jours**, chacun sur un premier message de **300 caractères ou plus** (2 agents, ou 299 caractères : non). |
| **Espace Docs** | `SPEC.md` (toute casse, à la racine) ou `docs/` changés par les commits d'un projet. | **3 jours différents sur les 14 derniers**, dans un même projet (2 jours : non ; plusieurs commits d'un jour comptent pour un). |
| **Espace Docs** | Des documents glissés dans les conversations : pdf, doc, docx, odt, rtf, md, markdown, txt, ppt, pptx, odp, xls, xlsx, ods, csv (une image ne compte pas). | **3 en 14 jours** (2 : non). |
| **Coût** | Une limite d'usage atteinte par un agent. | **1 en 7 jours** (aucune, ou la dernière il y a plus de 7 jours : non). |
| **Workflows** | Les skills (un dossier avec son `SKILL.md`), commandes et agents (un `.md`) d'un projet, sous `.claude/` et `.agents/`. Ses réglages et le reste ne comptent pas. | **2 dans un même projet** (1 : non). |
| **Tâches programmées** | La même consigne envoyée à la main : les mêmes mots, 20 caractères ou plus, la casse et les espaces ignorés. | **3 jours de suite**, la première de chaque jour toutes dans **une heure** de la journée (60 minutes d'écart au plus) (2 jours, un jour manqué entre, ou 61 minutes : non). |
| **Overseer** | Des agents qui attendent une réponse (une permission, une question) sur plusieurs projets à la fois. | **3 projets à la fois**, vus dans les **7 derniers jours** (2 projets : non). |

- Un signal n'est relevé que pendant que sa fonctionnalité est coupée. L'Éditeur, les Statistiques et la Voix n'ont pas de signal : ils ne sont jamais proposés.
- « À la main » : un message envoyé par la fenêtre ou par un client de l'API distante. Un message écrit par l'app (« continue » après la limite, une passation), par un agent ou par un client MCP ne compte pas.
- Les Tâches programmées ne sont proposées que le Kanban allumé : leurs passages sont des tickets. Kanban coupé, c'est le Kanban qui vient d'abord. « Activer » d'une tâche programmée rallume aussi le Kanban.
- Plusieurs signaux à la fois : la première fonctionnalité dans l'ordre du registre, et dans une fonctionnalité l'ordre du tableau.
- Ce qui est noté est gardé deux semaines au plus. Les commits d'un projet sont relus une fois par heure ; ses fichiers de `.claude/` et `.agents/` à chaque demande.

### Le rythme

- **Une seule proposition à la fois** : tant qu'une est à l'écran, aucune autre n'est demandée.
- **Au plus une par jour** : aucune dans les 24 heures qui suivent celle montrée, quelle que soit sa réponse.
- **Jamais pendant un tour visible** (un agent à l'écran au travail), **ni pendant qu'un agent attend une permission**, **ni pendant qu'on écrit** (une touche dans la seconde et demie, comme les bulles), ni par-dessus une fenêtre, le premier lancement ou une bulle. Le signal reste noté : la proposition vient au premier moment calme. La fenêtre redemande à chaque fois que le cœur a noté quelque chose, et chaque minute.
- **« Pas maintenant »** : la même ne revient pas avant une semaine (une autre peut venir le lendemain).
- **« Ne plus proposer »** : définitif, jusqu'à « Reproposer » dans Réglages › Fonctionnalités › « Propositions écartées ». Il tient après un redémarrage.
- **« Activer »** allume la fonctionnalité comme son interrupteur : ses vues, ses outils MCP et ce qu'elle fait en fond reviennent aussitôt.
- **Avec les bulles au bon moment** (*À découvrir*) : **une seule chose au bon moment par session**, une bulle ou une proposition. **La bulle passe d'abord** : elle naît d'un geste fait à l'instant et se perdrait sinon, alors qu'une proposition naît d'un signal gardé et peut attendre la session suivante. Une bulle à l'écran ou déjà montrée dans la session : aucune proposition. Une proposition montrée : plus aucune bulle dans la session.

### Ce qui reste sur la machine

- Ce qu'Escouade remarque est gardé dans `state.json`, par installation, avec le premier lancement et la découverte : des instants, et d'une consigne son empreinte seule, jamais son texte. Les commits et les fichiers d'un projet sont lus sur place. Rien n'est envoyé, rien n'est compté dans les statistiques d'usage, accord ou non.
- **Une fonctionnalité imposée** (allumée ou coupée) par la stratégie du poste ou par une équipe, ou celle dont elle dépend (le Kanban pour les Tâches programmées), n'est jamais proposée. « Activer » demandé par un client pour elle est refusé par le cœur, avec la raison du verrou.

### À distance

La proposition vient du cœur : un client de l'API distante ou une fenêtre sur un serveur montre celles de son cœur, avec ses règles. Le cœur ne propose rien pendant le tour d'un agent que ce client montre. Une réponse donnée sur un client vaut pour tous. Trois commandes de la liste unique (`src-tauri/src/remoteapi/list.rs`, `docs/REMOTE.md`) : `feature_offer` (celle à montrer, `visible` : les agents à l'écran du client), `feature_offer_act` (montrée, « Activer », « Pas maintenant », « Ne plus proposer », « Reproposer ») et `feature_offers` (les écartées).

## Imposer une fonctionnalité

Chaque fonctionnalité a sa stratégie dans la liste unique (`features.kanban` … `features.voice`, catégorie « Fonctionnalités »). C'est un booléen de sorte **valeur** : `true` l'impose allumée, `false` l'impose coupée. Le poste géré et l'équipe peuvent l'imposer (voir *Réglages imposés*). Le cœur ramène l'interrupteur à la valeur imposée et refuse de le changer. Ce qu'elle emporte suit, comme si l'utilisateur l'avait coupée. La raison des refus nomme qui l'impose. Une stratégie retirée laisse l'interrupteur où elle l'avait mis. Une fonctionnalité imposée n'est jamais proposée.

## Après une mise à jour, et pour une installation neuve

- **Une installation neuve** a tout coupé sauf l'essentiel (`NEW_INSTALL`, le choix de l'assistant, ESC-559) : la station des fonctionnalités du premier lancement allume chacune d'un geste, et « Ce que tu veux brancher » allume celle de ce qu'on y choisit.
- **Une installation mise à jour** vers la version qui apporte les interrupteurs les règle une fois, à son premier démarrage. Elle relève ce qu'elle utilise et l'allume :
  - le Kanban : les projets qui ont des tickets, ou une source de tickets connectée (ESC-559) ;
  - l'Espace Docs : les projets qui ont une spec, des décisions ou des documents dans `.escouade/`, une branche des docs, ou une discussion ;
  - l'Éditeur : un fichier ouvert dans l'éditeur (la découverte), un volet de l'éditeur, des extensions, des serveurs de langage réglés ou permis, une taille de texte réglée ;
  - les Workflows : les projets dont des workflows ont été lus ;
  - les Tâches programmées : les projets qui en ont ;
  - l'Overseer : allumé, ou déjà parlé ;
  - la Voix : réglée, un modèle téléchargé, ou la dictée déjà utilisée ;
  - les Statistiques : leur vue laissée ouverte ;
  - le Coût : un budget par jour, ou les économies du cache dans la barre.
  Le reste suit la valeur d'une installation neuve : coupé, jusqu'à l'assistant de la mise à jour qui le présente.
- Ce relevé est gardé (`featuresUsed`, le nombre de projets pour chacune). Réglages › Fonctionnalités le dit. L'assistant le dit (« Tu l'utilises déjà sur 2 projets : il reste allumé ») et son « Passer » ne coupe jamais une fonctionnalité utilisée. Le relevé ne se refait jamais : le choix de l'utilisateur n'est plus deviné.

## À distance

L'état des fonctionnalités vient du cœur : un client de l'API distante ou une fenêtre sur un serveur (voir *Le mode serveur*) montre celles du cœur où il est, avec leurs refus. Les interrupteurs passent par `save_settings`, déjà de la liste unique des commandes (`src-tauri/src/remoteapi/list.rs`) ; les propositions par leurs trois commandes (ci-dessus).

## Limites

- Le groupe « Workflow » est dans Réglages › Projet › Kanban : Kanban coupé, il part avec la page, même avec les Workflows allumés.
- Un client MCP qui a gardé la liste d'avant voit encore les outils d'une fonctionnalité coupée : leur appel est refusé avec la raison.
- Le Coût coupé ne retire pas les multiplicateurs de prix des modèles (Copilot « ×3 ») du menu des modèles : ils servent à choisir un modèle.
- Les découvertes des fonctionnalités coupées partent de la carte ; une découverte faite reste faite et revient cochée.
- Les propositions : les jours et les heures de « la même consigne » sont ceux du fuseau du cœur ; une proposition montrée sur deux clients à la fois y reste jusqu'à sa réponse sur chacun (la réponse de l'un vaut pour le cœur, pas pour l'écran de l'autre). L'Overseer n'est remarqué que lorsque la fenêtre redemande (chaque minute) : des agents en attente sur trois projets moins d'une minute peuvent passer inaperçus.

## L'assistant (ESC-559)

**Une fonctionnalité par écran** (maquette `design/features/assistant.png`, mise dans la ligne du premier lancement) :

- En haut, les fonctionnalités en pastilles : « ✓ Agents et conversation » d'abord (l'essentiel, toujours là, pas un bouton), puis les neuf dans l'ordre du registre, cochées quand elles sont allumées ; celle à l'écran est l'étape en cours (`aria-current`). À droite, « 2 / 9 » (« Fonctionnalité 2 sur 9 » pour un lecteur d'écran).
- L'écran : **une capture de la vraie app** (la fonctionnalité à sa place, dans le thème et la langue de la fenêtre), son titre (« Le Kanban »), à quoi elle sert en une phrase, **trois points**, et **un grand interrupteur « Activer le Kanban »** (un `switch`, décrit par ce qu'elle emporte quand on la coupe). Dessous :
  - ce qui est utilisé : « Tu l'utilises déjà sur 2 projets : il reste allumé. » (« Tu l'utilises déjà : elle reste allumée. » pour une fonctionnalité sans projet ; accordé à chacune) ; quand l'assistant est revu, simplement « Utilisée sur 2 projets » ;
  - les Tâches programmées, Kanban coupé : « Part avec « Kanban » : rallume-le d'abord. », l'interrupteur fermé ;
  - **une fonctionnalité imposée** par le poste géré ou l'équipe : l'interrupteur grisé, le cadenas et qui l'impose (« Imposé par la stratégie de ce poste », « Imposé par l'équipe Acme »).
- Ses interrupteurs sont ceux des réglages : chaque geste s'enregistre aussitôt, comme sur la page Réglages › Fonctionnalités, et le cœur arrête ou relance ce qu'elle fait.
- On passe d'un écran à l'autre par « Suivant » et « Précédent », par les pastilles, ou par ← →. Clavier, lecteur d'écran, « réduire les animations » (un fondu seulement), 1440 × 900 et 1024 × 728 (la capture passe au-dessus du texte quand la place manque), clair et sombre, français et anglais.
- **« Passer : garder l'essentiel » ne change aucun interrupteur** : ce qui n'a pas été allumé reste coupé, ce qui est utilisé reste allumé. Il ne garde donc que l'essentiel et ce qui est utilisé.
- **Ce qui est utilisé** : le relevé de la mise à jour (`featuresUsed`) et ce qui l'est au moment où l'assistant s'ouvre, avec le plus grand nombre de projets des deux. Une source de tickets connectée compte pour le Kanban.

**Où il s'ouvre** :

- **Au premier lancement**, c'est une station de la ligne, entre le premier agent et les statistiques (voir *Premier lancement*).
- **À la première fenêtre de la version qui apporte le choix**, sur une installation qui l'a mise à jour : la fenêtre « Choisis tes fonctionnalités » (« Escouade te laisse maintenant choisir ce qu'il te montre. Ce que tu utilises reste allumé ; le reste attend ton geste. ») s'ouvre d'elle-même, par-dessus l'app, jamais sur le premier lancement. « Passer : garder l'essentiel », « Précédent », « Suivant », « Terminer » au dernier écran ; Échap passe. Finie ou passée, elle ne revient plus d'elle-même. Un client de l'API distante ne la voit jamais.
- **« Revoir la présentation »**, au pied de Réglages › Fonctionnalités (« Chaque fonctionnalité, écran par écran, avec son interrupteur. ») : la fenêtre des réglages se ferme, l'assistant s'ouvre, « Les fonctionnalités d'Escouade », avec les interrupteurs tels qu'ils sont ; « Fermer la présentation » au lieu de « Passer ».
- **La fenêtre des nouveautés** (celle d'une mise à jour installée, « Voir les nouveautés ») : une fonctionnalité qu'une mise à jour apporte, que l'installation n'a pas encore rencontrée, y a son écran et le même interrupteur, sous « Nouvelle fonctionnalité », avant les notes. Elle y vient allumée (un interrupteur absent des réglages est allumé) ; vue, elle est rencontrée et ne revient plus. La 2.3 apporte les neuf ensemble : l'assistant les présente, la fenêtre des nouveautés n'en a aucune.

**L'état gardé** (`state.json`, avec le premier lancement) : l'assistant de la mise à jour à ouvrir (`featuresDue`, posé par le relevé d'une installation mise à jour, jamais d'une neuve) et les fonctionnalités rencontrées (`featuresKnown` : toutes celles de la version au premier démarrage, puis celles que la fenêtre des nouveautés a montrées). Commandes `features_tour_view` et `features_tour_act` (la fenêtre seule les montre, servies comme les autres).

**Les captures** viennent de la vraie app, refaites par un script : `npm run site-shots -- --grep "features"` (`tests/site-shots/features-tour.spec.ts`, sur la démo des captures du site), chaque fonctionnalité dans les deux thèmes et les deux langues, en JPEG de 960 × 540, dans `public/features/`. Une capture qui manque laisse l'icône de la fonctionnalité à sa place.

## Une installation neuve, une mise à jour : le choix de l'assistant (ESC-559)

- **Une installation neuve a toutes les fonctionnalités coupées** (`NEW_INSTALL`), l'essentiel seul, jusqu'à la station de l'assistant du premier lancement, où chacune s'allume d'un geste. Ce que l'utilisateur choisit à « Ce que tu veux brancher » allume sa fonctionnalité : une source de tickets le Kanban, la voix (Whisper ou un service) la Voix.
- **Une installation mise à jour** garde allumé ce qu'elle utilise (le relevé) et coupe le reste jusqu'à l'assistant de la mise à jour, qui le lui présente. Seule la constante `NEW_INSTALL` a changé dans le cœur.
- Ce que l'utilisateur choisit n'est jamais deviné ensuite : le relevé ne se refait pas, l'assistant ne s'ouvre plus de lui-même une fois fini ou passé.

## Avec le Workspace de la 2.2 (ESC-701)

La 2.2.13 à la 2.2.14 ont apporté le Workspace (la toile des agents) : le bouton « Éditeur » de la barre de titre ouvre un menu (« Éditeur », « Workspace »), et les workflows ont quitté le panneau latéral pour la toile. Sur la 2.3, ce que l'Éditeur et les Workflows emportent suit ces nouvelles places ; ceci précise leurs lignes du tableau :

- **Éditeur coupé** : le bouton reste dans la barre de titre, pour « Workspace », mais son menu n'a plus l'entrée « Éditeur », et son glisser vers un volet n'ouvre plus rien. Le reste de la ligne vaut tel quel.
- **Workflows coupés** : la barre du Workspace n'a plus « Workflows » ni son panneau, et la carte d'un agent n'a plus l'étape de son workflow (ni son schéma à ouvrir). L'onglet « Workflow » du panneau latéral n'existe plus, allumé ou non. Le groupe « Workflow » de Réglages › Projet › Kanban, `get_workflow` et le fond s'en vont comme avant.
- Le **Workspace** lui-même n'est pas une fonctionnalité : il n'a pas d'interrupteur.
