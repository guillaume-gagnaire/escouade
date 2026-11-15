---
number: 176
id: "dans-la-vue-vs-code-escouade-garde-les-t"
title: "Dans la vue VS Code, Escouade garde les touches qui mènent ailleurs dans l'app ; on demande avant toute fermeture avec des modifications"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-464"
pages: ["editeur/editeur","fenetre/raccourcis","editeur/ouvrir-un-fichier"]
tickets: ["ESC-464","ESC-447"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"editeur/editeur","added":47,"removed":7},{"page":"fenetre/raccourcis","added":5,"removed":11},{"page":"editeur/ouvrir-un-fichier","added":4,"removed":2}]
accepted_at: "2026-11-15T15:05:43+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis ESC-447 (décision 170), la vue VS Code est le seul éditeur de projet, mais ses touches et celles d'Escouade se recouvrent (Ctrl+P, Ctrl+K, Ctrl+N, Ctrl+J, Ctrl+,, Ctrl+Tab, Ctrl+Maj+L…) : les deux réagissaient, ou l'une était perdue sans le dire. Le clavier restait parfois dans une vue cachée, les liens des conversations ne menaient pas à la ligne, et fermer la vue, son volet, un projet ou l'app perdait les modifications non enregistrées sans rien demander. Le ticket A3 (docs/EDITOR-STUDY.md §7) demande une règle écrite et appliquée.

## Décision

1) La règle des touches quand la vue a le focus : Escouade garde celles qui mènent ailleurs dans l'app ou ouvrent une de ses fenêtres (Ctrl+P, Ctrl+1..9, Ctrl+N, Ctrl+Maj+N, Ctrl+J, Ctrl+T, Ctrl+,, Ctrl+Maj+A, Ctrl+Maj+B, le zoom, Ctrl+\, la dictée, et l'écran partagé ouvert Ctrl+Maj+\, Ctrl+Maj+W, Ctrl+Alt+flèche) ; toutes les autres sont à VS Code (Ctrl+S, Ctrl+F, Ctrl+Maj+F, Ctrl+Maj+P, Ctrl+K et ses accords, Ctrl+B, Ctrl+Tab, Ctrl+Maj+L, Ctrl+W…). Une touche n'est jamais aux deux : celles qu'Escouade garde ne parviennent pas à VS Code et sont retirées de ses raccourcis, leurs commandes restent dans sa palette par leur nom. C'est la même idée que dans un terminal : l'outil garde ses touches, l'app garde sa navigation. 2) Ctrl+P est la palette d'Escouade sur l'index du cœur, partout, la vue comprise ; le fichier choisi s'ouvre dans un onglet de la vue, à sa ligne. 3) Chaque fichier demandé à la vue (lien d'une conversation, Ctrl+P, panneau des fichiers) est une demande prise une fois : ouvert dans la source demandée, à sa ligne et sa colonne (en caractères), le clavier dedans. 4) La vue montrée prend le clavier ; cachée, elle ne le garde jamais ; un volet activé au clavier le donne à son contenu. 5) Fermer avec des fichiers modifiés demande, comme VS Code (Enregistrer, Ne pas enregistrer, Annuler) : l'onglet par la question de VS Code ; la vue (« Conversation »), son volet, un projet (après sa confirmation) et « Quitter » par celle d'Escouade. Un fichier qui ne s'enregistre pas ne ferme rien. Passer à un autre contenu ne ferme pas la vue : ses modifications restent et « Quitter » les compte.

## Conséquences

VS Code perd dans la vue Go to File, Toggle Panel, New Untitled File, Go to Symbol in Workspace, Split Editor et ses touches de zoom (accessibles par sa palette) ; Escouade y perd Ctrl+K (recherche des conversations), Ctrl+Tab (agent suivant) et Ctrl+Maj+L (disposition), qui marchent hors de la vue. Les compteurs de fichiers non enregistrés (Quitter, Recharger, mise à jour, suppression d'un worktree) comptent ceux de la vue. « Quitter » propose « Enregistrer et quitter » au lieu de « Quitter quand même ». Un fichier sans titre de VS Code n'est compté par aucune question. Les e2e A3 sont actifs (vscode-keys.spec.ts, et les deux d'editor-look) ; ceux d'A4 et A7 restent en attente.
