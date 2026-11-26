---
title: "Workspace"
subject: "Fenêtre"
sources: ["docs/superpowers/specs/2026-11-25-vue-workspace-design.md","src/components/workspace/WorkspaceView.svelte","src/lib/workspace-layout.ts","crates/escouade-plan/src/canvas.rs"]
tickets: []
decisions: []
compared: null
---

## Ce que c'est

Une toile où l'on voit d'un regard ce que font les agents d'un projet, comme une salle de contrôle : qui travaille, qui attend une réponse, ce qu'il utilise, ce qu'il a lancé, ce qui est sorti. Chaque agent non archivé y est une « boussole » : sa carte au centre, son plan au-dessus, ses skills et ses plugins, son équipe à droite, ses livrables en dessous. La vue ne remplace rien : c'est une autre façon de regarder le même état, d'où l'on peut aussi répondre à une question et écrire à un agent sans ouvrir sa conversation.

## Comportement

### Ouvrir et quitter

- Le bouton « Éditeur » de la barre de titre ouvre un menu : « Éditeur » et « Workspace », celle qui est ouverte cochée. Le glisser du bouton vers un volet ouvre toujours l'éditeur (voir *Écran partagé*).
- « Workspace » ouvre la toile du projet affiché à la place de la zone de travail ; la barre latérale reste. Choisir autre chose dans la barre latérale (un agent, le Kanban, les docs, un terminal…) quitte la toile.
- La « Vue d'ensemble » a un bouton « Workspace » : la toile des agents de tous les projets, à la place de sa liste.
- « Quitter le workspace », en haut à gauche et le premier dans l'ordre du clavier, ou Échap quand aucun champ n'a le focus et qu'aucune fenêtre n'est ouverte, revient à la vue d'avant, telle qu'elle était (le Kanban, les docs, les PR, un terminal, l'éditeur, ou la liste de la « Vue d'ensemble »).
- Double-clic sur une carte, ou Entrée quand elle a le focus : quitte la toile et ouvre ce qu'elle montre, la conversation d'un agent ou d'un sous-agent, la sortie d'une tâche de fond ou d'une commande de tests.

### La toile

- Un fond à points, à la couleur de la zone de travail teintée par celle du projet ; les points suivent la caméra.
- Se déplacer : glisser le fond, ou la molette (sauf sur une liste qui défile encore dans ce sens : celle du plan, l'encart d'une question). Zoomer : Ctrl/⌘ + molette ou pincement, autour du pointeur, de 30 % à 250 % ; au clavier `+`, `-`, et `0` pour ajuster.
- Une barre flottante en haut au centre : `−`, le pourcentage, `+`, « Ajuster », puis « 3 agents · 1 question ».
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

### Accessibilité

- La toile est une région ; ses cartes sont atteignables au clavier dans l'ordre du plan, des outils, de la carte, de l'équipe et des livrables. Les états, les traits et « a lancé » sont dits en mots, pas seulement en couleur ou en dessin. `prefers-reduced-motion` fige les flux, le halo et les rubans.
- En français et en anglais.

### D'où viennent les données

- La toile lit l'état de la fenêtre.
- Les outils, les artefacts et les commandes de terminal viennent du **résumé de toile** que le cœur tient pour chaque agent et envoie avec lui, mis à jour à chaque appel d'outil : pour lui et chacun de ses sous-agents, les skills et plugins (nom, appels, en cours), ses 5 dernières commandes de terminal, et ses artefacts. La conversation d'un agent n'est chargée que le temps de sa question.
- La CI distante et la PR d'un agent se relisent chaque minute tant qu'un contrôle ou un déploiement tourne.

## Limites

- Les cartes ne se déplacent pas à la main ; pas de mini-carte, pas de miniature de la page d'un artefact, pas de modification du plan depuis la toile.
- La toile ne s'ouvre pas dans un volet de l'écran partagé.
- Le résumé de toile garde 40 outils par fil, 30 sous-agents et 20 artefacts ; les plus anciens partent. Au-delà de 30 sous-agents, la pile dit « + 30+ terminés ».
- On écrit à un sous-agent seulement s'il tourne en arrière-plan.
