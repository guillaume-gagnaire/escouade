---
title: "Fonctionnalités"
subject: "fenetre"
sources: ["docs/FEATURES.md § 1 (conception du 2026-10-28, choix du chef)","design/features/reglages-fonctionnalites.png","design/features/assistant.png","docs/ONBOARDING.md § 7 (les découvertes)","crates/escouade-model/src/features.rs (le registre)","src-tauri/src/features.rs (ce que chacune emporte dans le cœur)","src/lib/features.ts (dans la fenêtre)"]
tickets: ["ESC-550"]
decisions: [226]
compared: null
---

# Fonctionnalités

## Ce que c'est

Escouade a grandi : Kanban, Docs, éditeur, coût, statistiques, workflows, tâches programmées, Overseer, voix. Tout le monde ne veut pas tout. Chaque **fonctionnalité entière** s'allume ou se coupe d'un interrupteur, comme un feature flag. Une fonctionnalité coupée **disparaît vraiment** : ses vues, ses boutons, ses menus, ses raccourcis, sa place dans la palette, ses outils MCP, ses découvertes. **Ce qu'elle faisait en fond s'arrête.** **Ses données restent** et reviennent telles quelles quand on la rallume.

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
- **Les pages d'une fonctionnalité coupée** quittent la barre et la recherche des réglages : Overseer, Voix, et pour un projet Kanban et Tickets importés. Les rangées d'une autre page qu'elle emporte en quittent la recherche aussi.
- « Revoir la présentation » vient avec l'assistant (F-2) : la page ne l'a pas encore.

## Imposer une fonctionnalité

Chaque fonctionnalité a sa stratégie dans la liste unique (`features.kanban` … `features.voice`, catégorie « Fonctionnalités »). C'est un booléen de sorte **valeur** : `true` l'impose allumée, `false` l'impose coupée. Le poste géré et l'équipe peuvent l'imposer (voir *Réglages imposés*). Le cœur ramène l'interrupteur à la valeur imposée et refuse de le changer. Ce qu'elle emporte suit, comme si l'utilisateur l'avait coupée. La raison des refus nomme qui l'impose. Une stratégie retirée laisse l'interrupteur où elle l'avait mis.

## Après une mise à jour, et pour une installation neuve

- **Une installation neuve** a tout allumé (`NEW_INSTALL`), jusqu'à ce que l'assistant du premier lancement (F-2) fasse le choix. Ainsi rien n'est caché à qui ne voit pas l'assistant (un parcours passé, un client d'un serveur).
- **Une installation mise à jour** vers la version qui apporte les interrupteurs les règle une fois, à son premier démarrage. Elle relève ce qu'elle utilise et l'allume :
  - le Kanban : les projets qui ont des tickets ;
  - l'Espace Docs : les projets qui ont une spec, des décisions ou des documents dans `.escouade/`, une branche des docs, ou une discussion ;
  - l'Éditeur : un fichier ouvert dans l'éditeur (la découverte), un volet de l'éditeur, des extensions, des serveurs de langage réglés ou permis, une taille de texte réglée ;
  - les Workflows : les projets dont des workflows ont été lus ;
  - les Tâches programmées : les projets qui en ont ;
  - l'Overseer : allumé, ou déjà parlé ;
  - la Voix : réglée, un modèle téléchargé, ou la dictée déjà utilisée ;
  - les Statistiques : leur vue laissée ouverte ;
  - le Coût : un budget par jour, ou les économies du cache dans la barre.
  Le reste suit la valeur d'une installation neuve : allumé.
- Ce relevé est gardé (`featuresUsed`, le nombre de projets pour chacune). Réglages › Fonctionnalités le dit. L'assistant de F-2 le dira (« Tu l'utilises déjà sur 2 projets : il reste allumé ») et ne coupera jamais une fonctionnalité utilisée. Le relevé ne se refait jamais : le choix de l'utilisateur n'est plus deviné.

## À distance

L'état des fonctionnalités vient du cœur : un client de l'API distante ou une fenêtre sur un serveur (voir *Le mode serveur*) montre celles du cœur où il est, avec leurs refus. Les interrupteurs passent par `save_settings`, déjà de la liste unique des commandes (`src-tauri/src/remoteapi/list.rs`) ; aucune commande n'est ajoutée.

## Limites

- Le groupe « Workflow » est dans Réglages › Projet › Kanban : Kanban coupé, il part avec la page, même avec les Workflows allumés.
- Un client MCP qui a gardé la liste d'avant voit encore les outils d'une fonctionnalité coupée : leur appel est refusé avec la raison.
- Le Coût coupé ne retire pas les multiplicateurs de prix des modèles (Copilot « ×3 ») du menu des modèles : ils servent à choisir un modèle.
- Les découvertes des fonctionnalités coupées partent de la carte ; une découverte faite reste faite et revient cochée.
- « Revoir la présentation », l'assistant (F-2) et les propositions au bon moment (F-3) viennent après.
