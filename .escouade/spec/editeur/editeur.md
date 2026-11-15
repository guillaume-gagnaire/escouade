---
title: "Éditeur"
subject: "Éditeur"
sources: ["docs/SPEC.md","docs/EDITOR-STUDY.md","docs/TODO-2.3.md"]
tickets: ["ESC-447","ESC-463","ESC-464","ESC-472"]
decisions: [170,172,176,182]
compared: "2026-10-30T05:39:53+01:00"
gaps: [{"text":"La spec dit que le double-clic sur la poignée ramène la colonne des fichiers à 240 px. Le code revient à 200 px (TREE_NARROW) quand la zone de l'éditeur fait 904 px ou moins, et à 240 px seulement au-delà.","code":["src/lib/editor/layout.ts","src/components/editor/EditorView.svelte"]}]
order: 25
---

## Ce que c'est

Le mode éditeur prend la zone principale d'un projet, ou un volet de l'écran partagé, pour lire et modifier le code d'une source : la branche du projet ou le worktree d'un agent. Depuis la 2.3 (ESC-447), c'est la vue de VS Code : son plan de travail (`@codingame/monaco-vscode-api`, épinglé à une version, VS Code 1.138), avec son explorateur, ses onglets, son éditeur, sa recherche et sa palette, aux couleurs, aux polices et dans la langue d'Escouade, sur des fichiers lus et écrits par le cœur. Il remplace l'éditeur en CodeMirror des versions précédentes, sans réglage pour choisir.

Ce que l'ancien éditeur faisait et que la vue ne fait pas encore revient par les tickets de l'éditeur de la 2.3 (voir `docs/EDITOR-STUDY.md`, §7) : A5 (extensions), A7 (marques des changements par rapport à la base, comparaison avec le disque), A1 (serveurs de langage). Les pages *Navigation de code*, *Recherche dans les fichiers* et *Voir les changements* décrivent l'ancien éditeur jusqu'à ces tickets.

## Comportement

### Ouvrir l'éditeur

- « Ouvrir dans l'éditeur », au clic droit sur un agent ou dans « … » de l'en-tête de sa conversation : son worktree, ou le projet s'il n'a pas de worktree.
- « Parcourir » sur la ligne de la branche, dans le pied de la barre latérale, et « Éditeur » dans la barre d'onglets : la branche du projet.
- Le bouton `</>` d'un fichier non commité, « Voir le code » d'un écart de la spec, un chemin de fichier dans la conversation, « Ouvrir un fichier » (`Ctrl+P`, voir *Ouvrir un fichier*) : le fichier s'ouvre dans un onglet de la vue, le curseur et le clavier dedans.
- Un fichier hors du dossier de la source ne s'ouvre pas (message).
- Une seule vue VS Code par fenêtre : seule dans la zone de travail ou dans un volet (voir *Écran partagé*). Elle se déplace, se redimensionne, se ferme et se rouvre en gardant ses onglets et leurs modifications. Ses groupes d'onglets peuvent se mettre côte à côte dans la colonne. Un clic dans la vue, ou le focus qui y entre, active son volet ; son survol montre le ×.
- La barre au-dessus : « Conversation », le sélecteur de source, « VS Code ». Le menu de la source s'ouvre par-dessus la vue.
- À la première ouverture dans la fenêtre, « Chargement de la vue VS Code… » ; si elle ne démarre pas : « La vue VS Code n'a pas démarré : <raison> ».

### Le thème, les polices et la langue

- **Le thème** : la vue prend les couleurs d'Escouade, celles de ses jetons (`src/app.css`) tels que la fenêtre les affiche, sur l'éditeur, la coloration du code, l'arborescence, les onglets, les barres, les menus, la palette, les infobulles, la recherche dans le fichier et ses occurrences, les erreurs et les avertissements. Chaque couleur de VS Code vient d'un jeton, parfois à une transparence : la sélection, les occurrences et la ligne de l'occurrence sont à la couleur du projet.
- Elle suit tout de suite, sans se recharger ni perdre ses onglets, sa recherche ou ses modifications : « Thème » (Clair, Sombre, ou Système et ses changements), la couleur du projet montré (une fois sa transition finie), « Couleurs » (le thème d'un plugin, comme *Contraste élevé*, et le retour à celles d'Escouade). Un jeton dont la valeur change, d'où que ce soit, change la vue.
- Le contraste reste lisible dans les deux thèmes, pour chaque couleur de projet : le code et sa coloration atteignent 4,5:1 sur l'éditeur, sa ligne courante, une sélection et une occurrence ; un commentaire garde 4,5:1 sur l'éditeur et 3:1 sous une sélection ou une occurrence ; les libellés des onglets, de l'arborescence, des barres, des menus, de la palette et des infobulles 4,5:1 ; le curseur, la bordure de l'occurrence, les traits d'erreur et d'avertissement 3:1.
- **Les polices** : l'interface de la vue dans celle de l'app (Inter), le code dans celle du code d'Escouade (JetBrains Mono), à l'épaisseur normale, avec ses ligatures comme ailleurs dans l'app. La taille du code est « Taille du texte de l'éditeur », suivie tout de suite quand elle change ; « Zoom de l'interface » agrandit la vue avec le reste.
- **La langue** : l'interface de VS Code (menus, palette, explorateur, questions, messages) est en français quand Escouade est en français, en anglais sinon. VS Code lit ses textes quand il se charge : la vue se charge dans la langue d'Escouade de ce moment-là, une fois par fenêtre.
- Quand la langue d'Escouade change, la vue ouverte le dit dans sa barre : « VS Code change de langue au rechargement de la fenêtre. », avec « Recharger ». « Recharger » demande d'abord pour les fichiers modifiés dans la vue (voir *Fermer avec des modifications non enregistrées* ; « Annuler » ne recharge rien), puis recharge la fenêtre : la vue revient dans la nouvelle langue, sur le même projet, ses onglets de ce projet rouverts, celui qui était montré au premier plan. Sans « Recharger », la vue garde sa langue jusqu'au prochain démarrage ou rechargement.

### Les liens des conversations

- Le chemin d'un fichier qu'un outil d'un agent a modifié (Edit, Write…) s'ouvre dans la source de l'agent : son worktree s'il en a un, sinon le projet. Le fichier s'ouvre à la première ligne changée, le curseur au début de la ligne (ou à sa colonne quand elle est donnée, comptée en caractères), la ligne amenée au milieu de la vue si elle était hors de vue.
- Chaque lien cliqué est une demande : le même fichier à la même ligne, redemandé après que le curseur a bougé, y ramène. Une vue redessinée (volet déplacé, rouvert) ne ramène pas le curseur à la dernière ligne demandée.
- Un fichier binaire s'ouvre en haut, comme VS Code l'affiche.

### Le clavier et le focus

- La vue montrée prend le clavier : son éditeur s'il a un onglet ouvert, sinon son explorateur. Dans un volet qui n'est pas l'actif, elle ne le prend pas.
- `Ctrl+Alt+flèche` vers le volet de la vue lui donne le clavier ; vers un autre volet, le clavier va à son contenu (le champ de message d'un agent, un terminal). Un clic dans la vue active son volet ; le clavier qui y entre aussi.
- Une fenêtre d'Escouade ouverte depuis la vue (`Ctrl+P`, `Ctrl+,`…) rend le clavier à la vue en se fermant.
- La vue cachée (« Conversation », son volet fermé, un autre contenu à sa place) ne garde jamais le clavier : ce qui est tapé ensuite ne va pas dans un fichier qu'on ne voit plus. « Conversation » donne le clavier au champ de message de la conversation qui revient.
- Les touches tapées dans la vue ne répondent à aucune demande d'agent (`1`, `2`… sur la carte d'une demande, `Échap` d'une vue de sous-agent ou de tâche) : elles restent à la vue.

### Les touches de la vue et celles d'Escouade

Quand la vue a le focus, la règle est : **Escouade garde les touches qui mènent ailleurs dans l'app ou ouvrent une de ses fenêtres ; toutes les autres sont à VS Code.** Une touche n'est jamais aux deux, et ne se perd pas en silence : celles qu'Escouade garde sont retirées des raccourcis de VS Code (ses menus et sa palette ne les affichent plus), et les commandes de VS Code qu'elles lançaient restent dans sa palette (`Ctrl+Maj+P`) par leur nom, en français comme en anglais.

Escouade garde :

| Touche | Escouade | Ce que VS Code en faisait |
|---|---|---|
| `Ctrl+P` | Ouvrir un fichier, sur l'index du cœur, dans un onglet de la vue | Go to File (sa palette garde aussi `Ctrl+E`) |
| `Ctrl+1..9` | Les projets | Focus Editor Group N |
| `Ctrl+N` / `Ctrl+Maj+N` | Nouvel agent / menu « Harness » | New Untitled Text File / New Window |
| `Ctrl+J` | Prochain agent en attente | Toggle Panel |
| `Ctrl+T` | Nouveau terminal | Go to Symbol in Workspace |
| `Ctrl+,` | Réglages d'Escouade | Open Settings |
| `Ctrl+Maj+A` | Vue d'ensemble | — |
| `Ctrl+Maj+B` | Panneau des fichiers | Run Build Task |
| `Ctrl+=` / `Ctrl+-` / `Ctrl+0` | Zoom de l'interface entière | Zoom du plan de travail |
| `Ctrl+\` | Partager l'écran | Split Editor |
| `Ctrl+Maj+\`, `Ctrl+Maj+W`, `Ctrl+Alt+flèche`, l'écran partagé ouvert | Tourner, fermer le volet, passer au volet voisin | Jump to Bracket, Close Window, Add Cursor Above/Below (hors écran partagé, ces touches restent à VS Code) |
| Le raccourci de la dictée (`Ctrl+Maj+Espace`) | La dictée | Trigger Parameter Hints |

VS Code garde tout le reste, dont `Ctrl+S` (enregistrer), `Ctrl+F` et `Ctrl+H` (chercher et remplacer dans le fichier), `Ctrl+Maj+F` (chercher dans les fichiers), `Ctrl+Maj+P` et `F1` (sa palette ; `Ctrl+Maj+P` n'imprime jamais la page), `Ctrl+K` et ses accords (`Ctrl+K Ctrl+C`…), `Ctrl+B` (sa barre latérale), `Ctrl+Tab` (ses onglets), `Ctrl+Maj+L` (sélectionner toutes les occurrences), `Ctrl+W` / `Ctrl+F4` (fermer l'onglet), `Échap`. Leurs actions d'Escouade (`Ctrl+K` : rechercher dans les conversations, `Ctrl+Tab` : agent suivant, `Ctrl+Maj+L` : disposition) marchent hors de la vue.

### Fermer avec des modifications non enregistrées

Rien ne se ferme sans demander quand un fichier est modifié dans la vue, comme VS Code : « Enregistrer » (« Tout enregistrer » pour plusieurs), « Ne pas enregistrer », « Annuler ». Le titre nomme le fichier (« Enregistrer les modifications de « f1.ts » ? ») ou leur nombre, avec leurs noms. « Enregistrer » écrit par le cœur puis ferme ; un fichier qui ne s'enregistre pas (un conflit, un refus du cœur) laisse tout ouvert : « Un fichier n'a pas été enregistré : rien n'est fermé. Regarde-le dans la vue VS Code. » « Ne pas enregistrer » relit les fichiers depuis le disque, puis ferme. « Annuler » ne ferme rien.

- **Un onglet** : la question de VS Code, dans sa langue (« Voulez-vous enregistrer les modifications apportées à f1.ts ? », « Enregistrer », « Ne pas enregistrer », « Annuler » ; en anglais, « Do you want to save the changes you made to f1.ts? », Save, Don't Save, Cancel).
- **La vue**, par « Conversation » : les fichiers du projet modifiés dans la vue. Sans fichier modifié, elle se ferme tout de suite.
- **Son volet**, par son × ou `Ctrl+Maj+W` : les fichiers du projet modifiés dans la vue.
- **Un projet**, une fois sa fermeture confirmée : ses fichiers modifiés dans la vue ; « Annuler » le garde. Fermé, ses dossiers et ses onglets quittent la vue.
- **La fenêtre rechargée pour la langue** (« Recharger » de la barre de la vue) : tous les fichiers modifiés dans la vue.
- **Escouade**, par « Quitter » : « Quitter Escouade ? », « N fichiers ne sont pas enregistrés dans l'éditeur : leurs modifications seront perdues si tu ne les enregistres pas. », « Enregistrer et quitter », « Quitter sans enregistrer », « Annuler ». Fermer la fenêtre par son × la range dans la barre des tâches : rien ne se perd, la vue garde ses onglets.
- « Recharger l'interface » (barre des tâches) compte aussi les fichiers modifiés dans la vue : le système demande avant de recharger, la page rechargée les perdant.
- Passer à un autre contenu (le Kanban, un autre agent, un autre projet) ne ferme pas la vue : elle garde ses modifications, et « Quitter » les compte.
- Supprimer un agent et son worktree, merger son ticket en nettoyant, supprimer un dossier d'agents : leur question dit combien de fichiers modifiés dans la vue seront perdus.

### Mouvement réduit

- Avec `prefers-reduced-motion`, le curseur de la vue ne clignote pas, suivi en direct. Le thème change sans transition propre à la vue : elle prend la couleur du projet une fois celle de la fenêtre arrivée.

### Sources

- Sélecteur de source : la branche du projet ou le worktree d'un agent, avec son nombre de modifications. Chaque source choisie devient un dossier de l'espace de travail de VS Code, nommé d'après le projet, ou d'après l'agent et son worktree (« agent-1 · .claude/worktrees/agent-1 »).
- L'éditeur et les terminaux refusent un agent d'un autre projet. Un worktree supprimé : « Le worktree de l'agent n'existe plus ».

### Les fichiers, par le cœur

- La page ne lit ni n'écrit jamais le disque elle-même : tout passe par les commandes de l'éditeur (arborescence, état d'un fichier, lecture et écriture en octets, création, renommage, suppression vers la corbeille), jamais hors du dossier de la source. Une adresse hors des sources servies n'est pas demandée au cœur.
- L'explorateur montre les fichiers connus de git et les « Fichiers copiés dans les worktrees » du projet (voir *Git & fichiers*) ; aucun autre fichier ignoré. Un worktree d'agent dans le dossier du projet est sa propre source.
- Un fichier se lit en octets, texte ou non (64 Mo au plus). VS Code garde son encodage, son BOM et ses fins de ligne ; un fichier binaire s'affiche comme VS Code les affiche.
- Créer, renommer, supprimer : par l'explorateur de VS Code. La suppression envoie à la corbeille du système.
- Écriture atomique, jamais par-dessus un fichier en lecture seule ni un dossier.

### Dossiers protégés

Ils ne se renomment ni ne se suppriment, et rien n'y est créé depuis la source du projet : ni dossier, ni nouveau fichier. Un fichier de worktree déjà là, ouvert par un lien, s'enregistre.

- Les worktrees des agents : `.claude/worktrees` à toute profondeur, le « Dossier des worktrees » du projet quand il est dans la source (voir *Git & fichiers*), ceux que `git worktree list` donne pour le dépôt, et ceux des agents de l'app.
- Le dossier de chaque projet de l'app qui se trouve dans la source.
- Un dossier qui contient l'un d'eux.

La règle est vérifiée sur le chemin écrit et sur le chemin réel (liens suivis, noms courts de Windows comme `WORKTR~1`).

### Changements sur le disque et conflit

- Le cœur pousse à la vue les chemins que son observateur de fichiers a vus changer (plus de 512 entre deux lectures : « tout a pu changer »). Un fichier ouvert et non modifié qu'un agent a changé se relit tout de suite ; les fichiers ajoutés ou supprimés apparaissent ou disparaissent ensuite dans l'explorateur.
- Cela vaut quel que soit le nom par lequel le dossier du projet a été ouvert : nom court de Windows (`RUNNER~1`), lien ou jonction, `/var` de macOS (lien vers `/private/var`). Le dossier que sert la vue et celui que surveille l'observateur sont tous deux nommés comme le disque les nomme (liens suivis, noms longs, sans le `\\?\` de Windows) ; un dossier qui n'existe pas encore, ou sur un partage, garde le nom donné (ESC-463).
- Un fichier modifié dans la vue et changé sur le disque n'est pas relu. `Ctrl+S` montre alors le conflit de VS Code, dans sa langue (« L'enregistrement de … a échoué : le contenu du fichier est plus récent… », avec « Remplacer » ; en anglais « The content of the file is newer… », « Overwrite ») et n'écrit rien. « Remplacer » écrit par-dessus.
- Le cœur refuse aussi une écriture quand le fichier a changé entre la vérification de VS Code et l'écriture (`changed`), ou a disparu (`deleted`).

## Réglages

- Taille du texte : Réglages › Général › Avancé › « Taille du texte de l'éditeur », suivie tout de suite par la vue ouverte (voir *Réglages*).
- « Thème », « Couleurs » et « Langue de l'interface » (Réglages › Général) : suivis par la vue, la langue au rechargement de la fenêtre (voir *Le thème, les polices et la langue*).
- Les « Fichiers copiés dans les worktrees » et le « Dossier des worktrees » du projet décident des fichiers montrés et des dossiers protégés (voir *Git & fichiers*).

## Limites

- La vue ne change de langue qu'au rechargement de la fenêtre ; rechargée, elle rouvre les onglets du projet montré, pas ceux d'un autre projet.
- Pas encore de serveurs de langage (A1) ni d'extensions (A5) : la vue n'a que les extensions déclaratives qu'elle embarque (langages, grammaires, thèmes). Une extension qui prendrait des touches (A5) passera par la même règle.
- Un fichier sans titre (« Nouveau fichier texte sans titre » de la palette de VS Code) n'est compté par aucune question d'Escouade.
- L'installeur grossit d'environ 3,3 Mo pour la vue, plus 0,35 Mo pour sa traduction française (1,9 Mo avant compression) ; ces fichiers ne se chargent qu'à la première ouverture de la vue, la traduction seulement en français.
- Sur un partage réseau sans corbeille, Windows peut supprimer pour de bon.
