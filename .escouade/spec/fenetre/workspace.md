---
title: "Workspace"
subject: "Fenêtre"
sources: ["docs/superpowers/specs/2026-11-25-vue-workspace-design.md","src/components/workspace/WorkspaceView.svelte","src/components/workspace/WorkflowsPanel.svelte","src/components/workspace/WorkflowBig.svelte","src/lib/workspace-workflow.ts","src/lib/workspace-layout.ts","crates/escouade-plan/src/canvas.rs","crates/escouade-base/src/secrets.rs","Correction de l'utilisateur du 2026-11-26, transmise par le chef (ticket ESC-689) : pas d'onglet Workspace, l'onglet Workflow retiré, les workflows dans la toile"]
tickets: ["ESC-689","ESC-701"]
decisions: ["le-workspace-une-toile-des-agents-de-leu","les-workflows-dans-le-workspace-l-onglet",374]
compared: null
---

## Ce que c'est

Une toile où l'on voit d'un regard ce que font les agents d'un projet, comme une salle de contrôle : qui travaille, qui attend une réponse, ce qu'il utilise, ce qu'il a lancé, ce qui est sorti. Chaque agent non archivé y est une « boussole » : sa carte au centre (avec son étape quand il suit un workflow du projet), son plan au-dessus, ses skills et ses plugins, son équipe à droite, ses livrables en dessous. Les workflows du projet y sont aussi, depuis que l'onglet Workflow du panneau de droite est parti. La vue ne remplace rien : c'est une autre façon de regarder le même état, d'où l'on peut aussi répondre à une question et écrire à un agent sans ouvrir sa conversation.

## Comportement

### Ouvrir et quitter

- Le bouton « Éditeur » de la barre de titre ouvre un menu : « Éditeur » et « Workspace », celle qui est ouverte cochée. Le glisser du bouton vers un volet ouvre toujours l'éditeur (voir *Écran partagé*).
- « Workspace » ouvre la toile du projet affiché à la place de la zone de travail ; la barre latérale reste. Choisir autre chose dans la barre latérale (un agent, le Kanban, les docs, un terminal…) quitte la toile.
- La « Vue d'ensemble » a un bouton « Workspace » : la toile des agents de tous les projets, à la place de sa liste.
- « Quitter le workspace », en haut à gauche et le premier dans l'ordre du clavier, ou Échap quand aucun champ n'a le focus et qu'aucune fenêtre n'est ouverte, revient à la vue d'avant, telle qu'elle était (le Kanban, les docs, les PR, un terminal, l'éditeur, ou la liste de la « Vue d'ensemble »).
- Échap ferme d'abord ce qui est par-dessus la toile : le diagramme d'un workflow, puis « Workflows » (le focus revient sur son bouton) ; la toile ensuite.
- Double-clic sur une carte, ou Entrée quand elle a le focus : quitte la toile et ouvre ce qu'elle montre, la conversation d'un agent ou d'un sous-agent, la sortie d'une tâche de fond ou d'une commande de tests.

### La toile

- Un fond à points, à la couleur de la zone de travail teintée par celle du projet ; les points suivent la caméra.
- Se déplacer : glisser le fond, ou la molette (sauf sur une liste qui défile encore dans ce sens : celle du plan, l'encart d'une question). Zoomer : Ctrl/⌘ + molette ou pincement, autour du pointeur, de 30 % à 250 % ; au clavier `+`, `-`, et `0` pour ajuster.
- Une barre flottante en haut au centre : `−`, le pourcentage, `+`, « Ajuster », puis « 3 agents · 1 question ». Sur la toile d'un projet, une pastille « Workflows » en haut à droite, à part de la barre pour qu'elle garde sa largeur dans une fenêtre de 1024 px (voir *Les workflows*).
- À l'ouverture, la toile est à 100 %, le premier agent en haut à gauche sous la barre flottante. « Ajuster » cadre tout. La caméra n'est pas mémorisée.
- Les agents sont rangés l'un sous l'autre, dans l'ordre de la barre latérale ; rien ne se déplace à la main, et une carte qui grandit pousse les autres sans chevauchement. Un agent lancé par un autre est relié à lui par un trait pointillé « a lancé », et sa carte dit « lancé par … ».
- Sur la toile de tous les projets, chaque projet est une colonne de ses agents sous son nom et sa couleur, les projets côte à côte dans l'ordre de la barre des projets, ceux sans agent omis.
- Sans agent : « Aucun agent dans ce projet » (« Aucun agent » sur la toile de tous les projets).

### Deux altitudes

- **De près**, à partir de 60 % : chaque boussole est dépliée.
- **À mi-hauteur**, sous 60 % : chaque côté d'une boussole est replié en une pile (« Outils ×4 », « Équipe ×4 », « Livrables ×3 ») et le plan en son en-tête et sa jauge. Un clic sur une pile la déplie sans changer le zoom, et le focus passe à son côté.
- Un agent qui attend une réponse est toujours déplié, à toutes les altitudes.

### La carte de l'agent

- Celle de la barre latérale : point d'état, nom, état en mots, modèle et durée, température du cache, tokens et coût, ce qu'il fait et sa ligne d'avancement. Contour vert quand il tourne, jaune avec un halo quand il attend, rouge en erreur.
- Son étape, quand il suit un workflow du projet (voir *Les workflows*).
- En bas, une barre de saisie d'une ligne, avec « Paramètres » (modèle, effort, mode, « Joindre un fichier ») et la dictée. Entrée envoie par le même chemin que la conversation : une réponse à la question en attente, le mode retenu, la file d'envoi, le brouillon rendu en cas de refus. Le brouillon est celui de l'agent : ce qu'on tape ici se retrouve dans sa conversation, et l'inverse ; les fichiers déjà joints sont dits « 2 fichiers ».
- **Une question ou une autorisation en attente** : la carte grandit et montre, au-dessus de la barre de saisie, l'encart jaune de la conversation, ses cartes de question et d'autorisation telles quelles. La question d'un sous-agent s'affiche sur la carte du sous-agent, et celle de l'agent dit « question de <sous-agent> ». La carte ne dépasse pas 70 % de la hauteur de la fenêtre ; au-delà, l'encart défile.

### Autour de la carte

- **Le plan**, au-dessus : le bandeau « Plan » de la conversation (voir *Avancée d'un plan*), tel quel, à la hauteur que le lecteur lui a donnée ; sa liste y défile à la molette. Sans plan, rien.
- **Les outils**, entre le plan et la carte : les skills (un appel de l'outil `Skill`, en violet) et les plugins (le serveur d'un outil `mcp__<serveur>__<outil>`, en bleu) que l'agent et ses sous-agents ont utilisés, avec leur nombre d'appels, dans l'ordre de leur premier appel. Les huit derniers appelés sont montrés, les autres dans une pile « + 3 outils » qui les déplie. Aucun trait permanent : le temps d'un appel, la carte de l'outil a un anneau vert et un flux descend vers l'agent.
- **L'équipe**, à droite : les sous-agents en cours (nom, « tâche 3 » quand on la connaît, modèle, durée, tokens, ce qu'il fait), un champ pour écrire à celui qui tourne en arrière-plan, puis les tâches de fond en cours, au contour pointillé. Un sous-agent lancé par un sous-agent est posé à droite de son parent. Les sous-agents terminés sont repliés en « + 4 terminés · 1 échec », qu'un clic déplie ; un échec pas encore vu reste hors de la pile. Un trait vert animé tant qu'il tourne, gris quand il a fini, rouge en échec.
- **Les tests**, collés sous la carte de l'agent ou du sous-agent qui les lance : sa dernière commande de tests, au terminal ou en tâche de fond, sa durée, son état (en cours, réussi, en échec, « arrêté » quand un redémarrage l'a coupée) et « Sortie ». Une commande de tests est la « Commande de tests » du Kanban, ou une commande qui y ressemble (`cargo test`, `npm test`, `vitest`, `pytest`, `go test`…), même dans un script ou un shell. Les lancements de la recette du Kanban n'en sont pas.
- **La zone d'atterrissage**, en dessous : « Zone d'atterrissage · 2 posés, 1 en approche », toujours dessinée (« Rien n'a encore atterri »). Un livrable en cours plane en pointillé ; fini, il se pose.
  - Un artefact publié par l'agent ou un sous-agent : son titre, « Artefact · version 2 · il y a 5 min », « Ouvrir ↗ ». Une carte par adresse, mise à jour à chaque publication.
  - La CI distante de sa branche : les contrôles de sa PR quand il en a une, en étapes, sinon « 12 contrôles, 1 en échec », et « Ouvrir ↗ ».
  - Les déploiements de sa PR ou d'un commit de sa branche, le plus récent par environnement, et « Ouvrir ↗ ».
  - Une carte de CI, de tests ou de déploiement a un ruban en tête : rayures vertes, tirets bleus pour un déploiement, qui bougent tant qu'elle tourne, figés et rouges en échec. Elle dit la branche, jamais le commit.

### Les workflows

Les workflows du projet, que montrait l'onglet Workflow du panneau de droite, sont dans la toile. Leur lecture, `.escouade/workflows/`, Réglages › Projets › Kanban › Workflow, l'outil MCP `get_workflow` et les faits `workflow.*` des plugins ne changent pas (voir *Workflows*).

- **L'étape sur la carte** : un agent qui suit un workflow du projet a, sous ce qu'il fait, une ligne « GSD · Planifier · 2ᵉ passage », et à droite comment va ce passage (« en cours » en vert, « renvoyée » ou « en échec » en rouge, « faite », « arrêtée », « quittée »). Le passage d'avant l'a renvoyé ici (un retour du workflow) : une seconde ligne en rouge, « Renvoyé par Vérifier : <raison> », la raison que l'étape a donnée ; elle part quand l'agent va plus loin. Rien pour un agent qui ne suit aucun workflow, qui en est sorti (« hors du workflow »), ou qui ne suit que « Tickets » (sa carte du Kanban a sa boucle).
- Un clic sur la ligne, ou Entrée, ouvre le diagramme du workflow en grand sur tout le panneau de travail, avec le chemin de cet agent (voir *Le diagramme d'un workflow* dans *Workflows*). Son infobulle dit « Voir le schéma de GSD et le chemin de <agent> », la raison d'un retour et ce qui vient ensuite (« Ensuite : Exécuter »).
- **« Workflows »**, la pastille en haut à droite de la toile d'un projet, avec le nombre d'agents qui suivent un workflow (« Tickets » compris), ouvre un panneau flottant sous elle :
  - les workflows du projet, tels que le projet les a (corrections comprises, ceux écartés retirés), puis « Tickets » tant qu'il a des tickets. Chacun : son nom, « 4 étapes · 2 agents le suivent » (« Un prompt relancé à chaque tour », « Le workflow des tickets d'Escouade, d'après le Kanban ») ou « Aucun agent ne le suit » ;
  - « Copier en Mermaid » (le workflow sans ses agents) et « Schéma » : le diagramme en grand, sans agent, la porte d'entrée à choisir ;
  - ses agents, ceux qui y sont avant ceux qui en sont sortis : point d'état, nom, et où il en est (« Planifier · 2ᵉ passage », « En cours · boucle 1/5 du ticket », « hors du workflow », « entre deux étapes »). Un clic ouvre le diagramme avec son chemin ;
  - en tête, « Réglages du workflow » (Réglages › Projets › Kanban › Workflow) et la fermeture (« Fermer (Échap) ») ;
  - un projet sans workflow dit ce qui a été cherché, avec « Décrire un workflow… » et « Analyser avec un agent… » (vers les réglages) ; un projet réglé sur « Pas de workflow pour ce projet » le dit, avec « Ouvrir les réglages du workflow ».
- Le séparateur d'étape de la conversation (« Voir l'étape sur le schéma du workflow ») ouvre la toile du projet et, par-dessus, le diagramme avec le chemin de l'agent.
- Le diagramme ouvert part avec la toile : il ne revient pas à sa prochaine ouverture.

### Accessibilité

- La toile est une région ; ses cartes sont atteignables au clavier dans l'ordre du plan, des outils, de la carte, de l'équipe et des livrables. Les états, les traits et « a lancé » sont dits en mots, pas seulement en couleur ou en dessin. `prefers-reduced-motion` fige les flux, le halo et les rubans.
- En français et en anglais.

### D'où viennent les données

- La toile lit l'état de la fenêtre.
- Les outils, les artefacts et les commandes de terminal viennent du **résumé de toile** que le cœur tient pour chaque agent et envoie avec lui, mis à jour à chaque appel d'outil : pour lui et chacun de ses sous-agents, les skills et plugins (nom, appels, en cours), ses 5 dernières commandes de terminal, et ses artefacts. La conversation d'un agent n'est chargée que le temps de sa question.
- **Les commandes gardées n'ont jamais un secret en clair** : chacune est masquée (`***`) avant d'être coupée à 300 caractères, gardée dans le fichier de l'agent et envoyée à la fenêtre. Sont masqués la valeur d'un nom qui dit un secret (`token=`, `GITHUB_TOKEN=`, `OPENAI_API_KEY=`, `--password`, `--api-key=`, `Authorization: Bearer …` dont le schéma reste, `X-Api-Key:`, `"password": …`), le mot de passe d'une adresse (`https://moi:***@hôte`) et de `curl -u moi:***`, une clé ou un jeton d'une forme connue (GitHub, GitLab, OpenAI et Anthropic, Slack, AWS, Google, npm, Hugging Face, un JSON Web Token, le mot après `Bearer`) et un bloc de clé privée. Une commande gardée en clair par une version d'avant est masquée à sa relecture. Masquer trop n'est pas grave : la commande n'est que montrée.
- Les étapes et les chemins viennent du suivi des workflows, comme l'onglet Workflow les lisait ; la toile lit les workflows de ses projets à son ouverture, et de nouveau quand leurs corrections changent.
- La CI distante et la PR d'un agent se relisent chaque minute tant qu'un contrôle ou un déploiement tourne.

## Limites

- Les cartes ne se déplacent pas à la main ; pas de mini-carte, pas de miniature de la page d'un artefact, pas de modification du plan depuis la toile.
- La toile ne s'ouvre pas dans un volet de l'écran partagé.
- Le résumé de toile garde 40 outils par fil, 30 sous-agents et 20 artefacts ; les plus anciens partent. Au-delà de 30 sous-agents, la pile dit « + 30+ terminés ».
- On écrit à un sous-agent seulement s'il tourne en arrière-plan.
- « Workflows » n'est que sur la toile d'un projet ; sur celle de tous les projets, les cartes gardent leur étape et leur diagramme.
- Le masquage des commandes reconnaît des formes : un secret sans nom qui le dise ni forme connue (`mysql -phunter2`, un mot de passe en argument nu) reste lisible.

## Sur la 2.3 (ESC-701)

- Le menu du bouton « Éditeur » n'a l'entrée « Éditeur » que si la fonctionnalité Éditeur est allumée, et son glisser vers un volet n'ouvre l'éditeur qu'alors ; « Workspace » y est toujours (voir *Fonctionnalités*).
- « Workflows » de la barre, son panneau et l'étape sur la carte d'un agent suivent l'interrupteur des Workflows : coupé, ils ne sont pas là.
- L'éditeur qu'ouvre « Éditeur » est celui de la 2.3, la vue de VS Code (voir *Éditeur*).
