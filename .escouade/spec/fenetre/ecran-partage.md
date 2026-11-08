---
title: "Écran partagé"
subject: "fenetre"
sources: ["docs/SPLIT.md","design/split-view/index.html"]
tickets: ["ESC-392"]
decisions: [128]
compared: null
---

# Écran partagé

La zone de travail d'un projet se partage en **volets**, côte à côte ou l'un au-dessus de l'autre, **quatre au plus**, pour voir en même temps par exemple un agent et un terminal, deux agents, ou l'éditeur à côté d'une conversation. Étude : `docs/SPLIT.md` ; aperçus : `design/split-view/` (direction B, choisie le 2026-11-08).

## Ce qui va dans un volet

- Un **agent** (sa conversation, avec tout ce qu'elle montre seule : sous-agent, tâche de fond, vue du harness, page du lancement, plan).
- Un **terminal**.
- Le **journal d'un lancement**.
- L'**éditeur** du projet.
- Le Kanban, les PR et les Docs n'y vont pas : les ouvrir prend toute la zone, sans défaire la disposition, qui revient quand on les quitte.
- Un même contenu n'est jamais montré deux fois : lâché sur un autre volet, il y passe ; au milieu d'un volet, les deux volets échangent.

## Ouvrir à côté

- **En glissant** une carte d'agent, une ligne de terminal ou de lancement depuis la barre latérale, ou le bouton « Éditeur » de la barre de titre, sur la zone de travail. Le geste commence au-delà de quelques pixels (un clic reste un clic). Sur le volet survolé (ou sur le contenu affiché seul) s'ouvre une boussole de cinq cases : haut, gauche, droite, bas (ouvrir à côté dans ce sens) et milieu (remplacer). La zone se lit à la position du pointeur dans le volet ; sa case s'allume et la place qu'il prendra se dessine, avec le nom du contenu et le sens. Une copie compacte de la carte suit le pointeur ; un bandeau rappelle les règles. Échap, ou lâcher hors de la zone de travail, annule.
- Une case est **barrée** quand il n'y a pas la place : quatre volets déjà, ou un volet qui ne garderait pas 320 px de large (côte à côte) ou 200 px de haut (l'un sur l'autre) de chaque côté. Lâcher sur une case barrée ne fait rien ; le milieu remplace toujours.
- **Au menu contextuel** d'une carte d'agent ou d'une ligne de terminal : « Ouvrir à côté », « Ouvrir en dessous ».
- **Ctrl+clic** (⌘+clic) sur une carte ou une ligne : ouvrir à côté.
- **Ctrl+\\** (la touche de code `Backslash`, `*` sur un clavier AZERTY) : sans écran partagé, met côte à côte ce qui est affiché et l'agent suivant du projet (ou, à défaut, un de ses terminaux) ; avec, revient à un seul volet, l'actif.
- Un clic simple dans la barre latérale remplace le contenu du **volet actif**.

## La poignée et le sens

Entre deux volets, 8 px de cadre et une poignée : tirée à la souris, aux flèches quand elle a le focus (Maj : plus vite ; Début, Fin : aux bornes ; Entrée : moitié-moitié), double-clic : moitié-moitié. La part de chaque côté reste entre 15 % et 85 %. **Ctrl+Maj+\\** tourne la séparation du volet actif (côte à côte ↔ l'un sur l'autre).

## Le volet actif

Un seul volet est actif : le dernier où l'on a appuyé, celui qui vient de s'ouvrir, ou celui atteint par **Ctrl+Alt+flèche** (le volet de ce côté). Chaque volet est un panneau à lui sur le cadre ; l'actif est entouré d'un anneau de la couleur du projet, les autres sont à peine plus sombres. Le suivent :

- le focus du champ de saisie (seul le composer du volet actif le prend) et celui d'un terminal (seul celui du volet actif le prend à son montage) ;
- la dictée quand aucun champ n'a le focus ;
- les réponses au clavier aux questions et autorisations (Ctrl+Entrée, Alt+chiffre) : celles des autres volets se répondent à la souris ;
- Ctrl+Tab, le tiroir des fichiers (par-dessus la conversation, jamais posé à côté dans un volet), la source de l'éditeur et Ctrl+S, Ctrl+Maj+F, Ctrl+P de l'éditeur quand il est le volet actif, la barre de statut.

**Ctrl+Maj+W** ferme le volet actif ; le × en haut à droite d'un volet (au survol) aussi. Ces raccourcis passent aussi quand un terminal a le focus.

## Notifications et barre latérale

Tout agent montré dans un volet, fenêtre au premier plan, est **vu** : il ne clignote pas. La barre latérale surligne l'élément du volet actif et entoure ceux des autres volets.

## Ce qui est retenu

La disposition (l'arbre des volets, sens, parts, contenus, volet actif) est retenue **par projet** avec l'état de l'interface et revient au redémarrage. Un contenu qui n'existe plus (agent archivé ou supprimé, terminal fermé ou arrêté par la fermeture de l'app, lancement retiré) quitte son volet ; les autres prennent la place, et le dernier restant occupe seul la zone de travail.

## Ailleurs

Le remote control suit une conversation, pas l'écran. Vue d'ensemble, Stats et Plugins prennent toute la zone comme avant. Le « Côte à côte » existant (conversation et fichiers, Ctrl+Maj+L) reste tel quel quand un seul contenu est affiché ; ses boutons n'apparaissent pas dans un volet.
