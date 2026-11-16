---
title: "Voir les changements"
subject: "Éditeur"
sources: ["docs/EDITOR-STUDY.md","docs/TODO-2.3.md","src/lib/vscode/changes.ts","src/lib/vscode/scm.ts"]
tickets: ["ESC-476"]
decisions: [185]
compared: null
order: 29
---

## Ce que c'est

La vue VS Code de l'éditeur (voir *Éditeur*) montre ce qu'un fichier a changé par rapport à sa base : les marques des lignes changées entre les numéros et le texte, leur compte dans la barre de la vue, les fichiers changés dans l'arbre, et sur demande la comparaison avec la base ou avec le disque dans l'éditeur de diff de VS Code. Depuis la 2.3 (ESC-476, A7), c'est VS Code qui les dessine, sur la base que le cœur lit ; l'éditeur en CodeMirror des versions précédentes le faisait à sa façon.

## Comportement

### La base

- Chaque source a la sienne, la même que l'onglet « Fichiers » et l'ancien éditeur : HEAD pour le checkout du projet ; pour le worktree d'un agent, le commit où sa branche a quitté sa branche de base (« main »), de sorte que ses commits comptent comme des changements.
- Chaque dossier de l'espace de travail de la vue (le projet, le worktree de chaque agent montré) a sa base. Un fichier d'un worktree rangé dans le dossier du projet a la base de son worktree, pas celle du projet.
- La page ne lance jamais git : le texte de la base vient du cœur (la commande qui la lisait pour l'ancien éditeur), servi à VS Code par un schéma à lui (`escouade-base:`), en lecture seule ; rien ne s'y écrit.
- Hors d'un dépôt, pas de base : pas de marques, rien dans la barre. Un fichier absent de la base a toutes ses lignes ajoutées.

### Les marques et leur compte

- Entre les numéros et le texte, les marques du « quick diff » de VS Code : lignes ajoutées (vert, `--add`), modifiées (ambre, `--git-m`), supprimées (rouge, `--del`, un triangle entre deux lignes). Elles sont aussi dans la règle de défilement.
- Un clic sur une marque ouvre l'aperçu du changement de VS Code, dans le texte : la version de la base, « Rétablir », le changement suivant et le précédent.
- Elles suivent tout de suite ce qui est tapé, avant l'enregistrement ; un fichier changé sur le disque (par un agent) et relu ; la base qui bouge (un commit, un checkout, la branche de base qui avance) : le cœur le dit à la page, qui relit les bases.
- La barre de la vue dit, pour le fichier montré : « N lignes modifiées par rapport à main » (le compte en vert : lignes ajoutées ou modifiées, celles des marques), « Nouveau fichier · absent de main », ou « Identique à HEAD ». Suivi de « Comparer » quand il y a un changement.

### Comparer avec la base

- « Comparer » de la barre de la vue, « Comparer avec la base » au clic droit sur l'onglet du fichier ou sur un fichier de l'explorateur, et dans la palette de VS Code (« Escouade: Comparer avec la base ») : l'éditeur de diff de VS Code, la base à gauche et le fichier à droite, dans un onglet « app.ts (main) ↔ app.ts ».
- Le diff est côte à côte, ou en ligne quand la place manque ; son menu « … » passe de l'un à l'autre. Le côté de la base ne s'écrit pas ; le fichier se modifie et s'enregistre comme dans son onglet.

### Comparer avec le disque

- Un fichier modifié dans la vue se compare avec sa version du disque : « Comparer avec le fichier enregistré » au clic droit sur son onglet (quand il est modifié) et dans la palette (« Comparer le fichier actif avec le fichier enregistré » de VS Code).
- Le conflit d'un enregistrement (le fichier a changé sur le disque, voir *Éditeur*) mène à la même comparaison : « Comparer » de sa notification, la version du disque à gauche, la mienne à droite, avec « Remplacer » ou « Rétablir » de VS Code.

### Les fichiers changés dans l'arbre

- Dans l'explorateur de la vue, chaque fichier changé de la source a sa lettre après son nom et sa couleur : A ajouté (`--add`), M modifié (`--git-m`), D supprimé (`--del`). Chaque dossier qui en contient (la racine de la source mise à part) a un point et son nom en ambre (`--git-m`), quels que soient les fichiers dessous, comme l'éditeur de la 2.0 les marquait. Le survol dit « Modifié par rapport à main », ou « Des fichiers modifiés par rapport à main » sur un dossier.
- La ligne sélectionnée de l'explorateur, quand il a le clavier, prend la couleur de sélection de VS Code ; elle reprend celle de sa lettre quand le clavier est ailleurs.
- Les fichiers changés sont ceux de l'onglet « Fichiers » pour cette source (les fichiers non commités du checkout, ou du worktree de l'agent). Un fichier renommé y est un fichier supprimé et un fichier ajouté, comme dans l'onglet « Fichiers ».
- « Changements », sous l'explorateur : la liste des fichiers changés de chaque source montrée (rangés par dossier de l'espace de travail quand il y en a plusieurs), avec leur lettre et leur dossier. Un clic ouvre la comparaison avec la base. Un fichier supprimé, que l'explorateur ne montre plus, y est : il s'ouvre en comparaison, sa base à gauche et rien à droite. Sans changement : « Aucun fichier modifié par rapport à la base. »
- Les lettres et la liste suivent le disque et git : un fichier écrit par un agent, un commit, un fichier rétabli.

### Les sources

- Changer de source dans la barre de la vue (le checkout du projet ou le worktree d'un agent) ajoute son dossier à l'espace de travail avec sa base : ses marques, ses lettres, sa liste et ses comparaisons sont les siennes.

### Ce que la vue ne montre pas

- La vue « Contrôle de code source » de VS Code ne se montre jamais, ni ses boutons de validation (commit), de push, de synchronisation ou de branche : son entrée de la barre d'activité est cachée, son raccourci (`Ctrl+Maj+G`) retiré, et l'ouvrir autrement (« Afficher : Contrôle de code source » de la palette) montre l'explorateur. Aucune source de contrôle de la vue n'a de zone de saisie ni de commande d'envoi, et la palette n'a aucune commande qui valide, pousse ou synchronise. Les commits et les pushs restent ceux d'Escouade (onglet « Git », barre de l'agent).

## Couleurs

- Les marques, les lettres, les points des dossiers, les lignes et le texte ajoutés ou supprimés de l'éditeur de diff viennent des jetons d'Escouade (`--add`, `--git-m`, `--del`, `--line`) par la table du thème de la vue (voir *Éditeur*, *Le thème*), clair et sombre, et suivent leurs changements.

## Limites

- Les marques et la comparaison se lisent sur du texte : un fichier binaire n'en a pas.
- Une dernière ligne vide remplacée par une ligne écrite compte comme une ligne modifiée, comme dans VS Code.
- Le compte de la barre est celui du fichier de l'éditeur actif de la vue ; un fichier ouvert dans un autre projet ne s'y lit pas.
