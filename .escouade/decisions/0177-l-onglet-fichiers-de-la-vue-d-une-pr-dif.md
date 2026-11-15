---
number: 177
id: "l-onglet-fichiers-de-la-vue-d-une-pr-dif"
title: "L'onglet Fichiers de la vue d'une PR : diff de git, arbre, « Vu » par PR et par commit, ouvrir dans l'éditeur"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-467"
pages: ["git-fichiers/pull-requests"]
tickets: ["ESC-467"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"git-fichiers/pull-requests","added":23,"removed":8}]
accepted_at: "2026-11-15T17:32:23+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

PR-3 des PR dans Escouade (design/pr-2-3, direction A). La vue d'une PR (décision 171, ESC-457) avait un onglet Fichiers grisé : on ne lisait pas le diff d'une PR sans quitter Escouade. PR-1 (ESC-452) donnait déjà au cœur les lignes de la PR par git. CodeMirror n'est plus dans l'app ; le diff de Monaco aurait pesé et mal laissé la place aux fils sous une ligne (PR-4, PR-5).

## Décision

L'onglet Fichiers s'allume. Le diff vient toujours de git (refs/escouade/review/<n>/read/… de pr_contents ; pour un commit, la nouvelle commande pr_commit_files : son diff depuis son premier parent, seulement pour un commit que l'hébergeur liste pour la PR), jamais d'un texte de l'hébergeur. Rendu à nous, parti du diff unifié de l'app, coloré par shiki (déjà là pour les blocs de code), dessiné par tranches de 100 lignes à l'approche de l'écran, avec un emplacement sous chaque ligne pour les fils. Un arbre des fichiers modifiés à gauche, filtrable, qui suit le défilement. Un binaire, un fichier généré (lockfiles, minifiés, linguist-generated du .gitattributes de la racine au commit lu) et un fichier de plus de 400 lignes de diff sont repliés jusqu'à « Afficher ». « Vu » est gardé dans l'état de l'interface (prViewed, 100 PR au plus) : pour la PR entière par l'empreinte du diff du fichier, qui redevient non vu quand un commit le change ; pour un commit, à part. Le choix toute la PR / un commit est dans la barre, et chaque commit de l'onglet Commits y mène. « Ouvrir dans l'éditeur » (fichier ou ligne) passe par app.openEditor, dans le worktree de l'agent qui tient la branche, sinon dans le dossier du projet.

## Conséquences

Une PR se relit fichier par fichier dans Escouade, même grosse (300 fichiers, 20 000 lignes : seul ce qui est près de l'écran est dessiné). Le diff reste unifié, sans serveur de langage ; les fils sous leur ligne viendront avec PR-4. Dans le dossier du projet, le fichier ouvert n'est pas forcément à la version de la PR. L'ouverture à la ligne passe par la même entrée que les conversations (openEditor avec une ligne, prise par la vue VS Code depuis ESC-464).
