---
number: 143
id: "le-diagramme-d-un-workflow-part-d-un-poi"
title: "Le diagramme d'un workflow part d'un point d'entrée : le possible en léger, le chemin de l'agent tracé et numéroté dessus"
date: "2026-11-10"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-406"
pages: ["kanban/workflows"]
tickets: ["ESC-406"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"kanban/workflows","added":24,"removed":0}]
accepted_at: "2026-11-10T09:56:35+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Le diagramme de l'onglet Workflow (ESC-245) dessinait le workflow entier, jusqu'à trois portes en tête, avec la progression d'un agent (faites, en cours, ensuite, retours pris). Le chemin réellement suivi ne s'y lisait pas dans l'ordre (passages, retours, tours de boucle), et ce qui ne peut plus arriver depuis la porte prise était dessiné comme le reste. Demande de l'utilisateur du 2026-11-10 : « je veux toujours partir d'un point d'entrée, et après le diagramme montre où ça peut aller et où ça va vraiment ». Trois directions lui ont été montrées en PNG (design/workflow-path : A, le tracé sur la carte ; B, le fil déroulé ; C, la carte et son parcours écrit) ; il a choisi A.

## Décision

Direction A, « le tracé sur la carte ». Le diagramme part toujours d'une entrée : la porte du premier passage de l'agent lu (sans ses arguments, sinon la commande de son étape), sinon un choix de toutes les portes d'entrée, la première par défaut. Il ne dessine que ce que cette entrée atteint par les transitions et les retours (la sortie si son étape est atteinte, le tour suivant seulement vers une étape atteinte) ; le reste se replie en « n étapes hors de ce chemin », ouvrable. Le possible est léger et pointillé ; le chemin de l'agent est en trait plein et épais, chaque passage numéroté sur son étape dans l'ordre du parcours, les retours pris en rouge avec la raison donnée par l'étape, les tours de boucle comptés (« 1/3 »), l'étape en cours et la suivante dites en mots. Les autres agents restent des pastilles. Le chemin se lit aussi dans une liste ordonnée pour un lecteur d'écran, dans la colonne et en grand. Le chemin vient des passages que le suivi garde déjà (ordre, porte, raison, issue) : rien de nouveau n'est gardé dans le cœur. « Copier en Mermaid » copie toujours le workflow entier.

## Conséquences

Un agent entré en cours de workflow ne voit plus les étapes d'avant sa porte, sauf en ouvrant le repli. Sans agent, toutes les étapes sont dessinées comme des possibles (bord pointillé), ce qui change l'allure du diagramme vu seul. Les étapes « faites » n'ont plus de coche quand elles portent des numéros. Le retour pris montre la raison de l'agent à la place de la condition du workflow. Les directions B et C sont écartées ; un parcours écrit toujours visible (C) pourra s'ajouter si les numéros ne suffisent pas sur de longs chemins (au-delà de quatre passages d'une étape, seuls les trois derniers sont montrés, avec « +n »). Le calcul reste dans le front (src/lib/workflow-diagram.ts) : pas de changement du cœur ni du format du suivi.
