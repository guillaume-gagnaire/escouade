---
number: 185
id: "les-changements-par-rapport-a-la-base-da"
title: "Les changements par rapport à la base dans la vue VS Code : le quick diff de VS Code sur une base servie par le cœur"
date: "2026-11-15"
state: "accepted"
origin: "ticket"
origin_ref: "ESC-476"
pages: ["editeur/voir-les-changements-comparer-avec-le-disque","editeur/editeur"]
tickets: ["ESC-476"]
documents: []
supersedes: []
superseded_by: null
changes: [{"page":"editeur/voir-les-changements-comparer-avec-le-disque","added":34,"removed":23},{"page":"editeur/editeur","added":5,"removed":5}]
accepted_at: "2026-11-16T09:36:05+01:00"
accepted_by: "Guillaume Gagnaire"
---

## Contexte

Depuis ESC-447, l'éditeur d'un projet est la vue VS Code. L'éditeur en CodeMirror de la 2.0 marquait les lignes changées par rapport à la base, disait « N lignes modifiées par rapport à main » au-dessus du texte, comparait un fichier avec sa base ou avec le disque et montrait les fichiers changés dans son arbre ; la vue ne le faisait plus (A7 de docs/EDITOR-STUDY.md §7). Il fallait choisir comment VS Code obtient la base sans que la page lance git, où se lit le compte des lignes, et ce qu'on garde de la gestion de sources de VS Code, qui vient avec ses boutons de commit et de push.

## Décision

- Les marques sont celles du « quick diff » de VS Code, pas des décorations faites à la main : chaque dossier de l'espace de travail (le checkout du projet, le worktree de chaque agent montré) a sa source de contrôle, dont la ressource d'origine d'un fichier est sa base. La base est la même que celle de l'onglet « Fichiers » et de l'ancien éditeur : HEAD pour le checkout, le point où la branche de l'agent a quitté sa base pour un worktree ; un fichier d'un worktree rangé dans le dossier du projet a la base de son worktree.
- Le texte de la base vient du cœur, par la commande qu'il avait déjà (`fs_base`), servi à VS Code sous un schéma à lui (`escouade-base:`) par un fournisseur de contenu en lecture seule ; la page ne lance jamais git et rien ne s'écrit dans ce schéma. Les lettres de l'arbre viennent de `git_files`, la liste de l'onglet « Fichiers » (A, M, D ; un renommage y est D puis A, comme dans « Fichiers »), par les décorations de fichiers de VS Code. Les dossiers qui en contiennent sont marqués par le fournisseur lui-même, d'un point ambre quels que soient les fichiers dessous, comme l'éditeur de la 2.0 (la propagation de VS Code prendrait la couleur du premier fichier rencontré). Les bases et les lettres se relisent quand l'observateur de fichiers ou la boucle git du cœur disent quelque chose du projet.
- Les sources de contrôle n'ont pas de racine donnée à VS Code (il compare la racine avec la casse du chemin, et nomme certains modèles `/C:/` quand le dossier est en `/c:/`) : la base d'un fichier est celle de sa propre source, trouvée par le fournisseur de fichiers. Pour la même raison, un changement de lettre est annoncé sous les deux écritures du lecteur.
- « N lignes modifiées par rapport à main » se lit dans la barre de la vue (celle d'Escouade, au-dessus du plan de travail), pas dans la barre d'état de VS Code : à côté de la source qu'il concerne, dans le style de l'aperçu de l'éditeur, avec « Comparer ». Le compte est celui des marques (l'information de diff de l'éditeur, une proposition d'API que l'extension de la page est seule à pouvoir lire).
- Comparer avec la base : l'éditeur de diff de VS Code, depuis « Comparer » de la barre, le menu de l'onglet, le menu de l'explorateur et la palette. Comparer avec le disque : « Comparer avec le fichier enregistré » de VS Code, au menu de l'onglet d'un fichier modifié et dans la palette ; le conflit d'enregistrement d'A2 y mène par son « Comparer ».
- Une liste « Changements » sous l'explorateur montre les fichiers changés de chaque source, un fichier supprimé compris (l'explorateur ne le montre plus), et les ouvre en comparaison.
- La vue « Contrôle de code source » de VS Code ne se montre jamais. Le service de gestion de sources vient entier avec le quick diff, qui ne marche pas sans le reste : retirer du registre le conteneur de cette vue, ou ses seules vues, fait aussi disparaître les marques. La vue est donc cachée. Son entrée de la barre d'activité ne s'affiche pas (une feuille de style), son raccourci `Ctrl+Maj+G` est retiré des raccourcis de VS Code, et son ouverture par la palette montre l'explorateur à la place. Aucune source de contrôle n'a de zone de saisie ni de commande d'envoi, et rien ne valide, ne pousse ni ne synchronise : ce sont les boutons d'Escouade qui restent.

## Conséquences

- Un paquet de plus, `@codingame/monaco-vscode-scm-service-override` 37.6.1, épinglé comme les autres. Sa racine apporte aussi la vue de la gestion de sources (sa partie `/common` seule ne dessine pas de marques), d'où la vue cachée plutôt qu'absente.
- La page a la proposition d'API `textEditorDiffInformation` pour sa propre extension (déclarée dans la configuration du produit) ; si une version de VS Code la retire, les marques restent et seul le compte de la barre manque.
- Les lettres de l'arbre suivent la liste de « Fichiers » (non commités) : pour un worktree dont l'agent a commité, les marques montrent aussi ses commits, ses lettres non. Faire suivre aux lettres la même base que les marques demanderait une nouvelle commande du cœur.
- La ligne sélectionnée de l'explorateur, quand il a le clavier, prend la couleur de sélection de VS Code plutôt que celle de sa lettre (règle de VS Code, gardée).
- En corrigeant, un défaut plus ancien est réglé : une source choisie juste après l'ouverture de la vue prenait la place du dossier du projet dans l'espace de travail ; la vue attend désormais que l'hôte d'extensions connaisse les dossiers déjà là.
- Les e2e A7 de l'ancien éditeur sont repassés sur la vue (aperçu, barre en anglais, place des libellés, phrase d'app.spec) ; dans choices-look, 53 et 56 sont repassés sur l'arbre et la recherche de VS Code, 54 (pas d'icônes) et 55 (largeur de 200 px) retirés avec l'ancien arbre.
