---
title: "Éditeur"
subject: "Éditeur"
sources: ["docs/SPEC.md"]
tickets: []
decisions: []
compared: null
order: 25
---

## Ce que c'est

Le mode éditeur prend la zone principale d'un projet pour lire et modifier le code d'une source : la branche du projet ou le worktree d'un agent. Il remplace l'ouverture dans un éditeur externe. Le code s'affiche dans CodeMirror 6, aux couleurs du design, avec à gauche une colonne de fichiers. La navigation, la recherche, l'ouverture rapide et les changements ont leurs pages (voir *Navigation de code*, *Recherche dans les fichiers*, *Ouvrir un fichier*, *Voir les changements*).

## Comportement

### Ouvrir l'éditeur

- « Ouvrir dans l'éditeur », au clic droit sur un agent ou dans « … » de l'en-tête de sa conversation : son worktree, ou le projet s'il n'a pas de worktree.
- « Parcourir » sur la ligne de la branche, dans le pied de la barre latérale, et « Éditeur » dans la barre d'onglets : la branche du projet.
- Le bouton `</>` et le clic droit sur un fichier non commité.
- Dans la conversation : le chemin du fichier des lignes Edit, Write et MultiEdit (ouvert à la première ligne modifiée), et les fichiers listés par la carte « Tâche terminée ».
- « Ouvrir un fichier » (`Ctrl+P`, voir *Ouvrir un fichier*).
- Un fichier hors du dossier de la source ne s'ouvre pas (message).
- Choisir un agent dans la barre latérale bascule l'éditeur sur sa source. « Nouvel agent », un terminal, une commande de lancement, le Kanban, une notification, la vue d'ensemble ou un résultat de « Rechercher dans les conversations » ferment l'éditeur pour montrer ce qu'ils ouvrent ; ses onglets sont gardés.

### Source et onglets

- Sélecteur de source : la branche du projet ou le worktree d'un agent, avec son nombre de modifications.
- L'éditeur et les terminaux refusent un agent d'un autre projet.
- Un agent dont le worktree a été supprimé (après une pull request ou un push) : l'arborescence, la recherche et l'ouverture d'un fichier disent « Le worktree de l'agent n'existe plus ». Rien n'est recréé à son chemin.
- Un onglet modifié porte un point « non enregistré ».
- Deux onglets de même nom montrent en gris le plus court dossier qui les distingue (« index.ts · editor », « index.ts · board »). L'infobulle donne le chemin complet ; le bouton de fermeture porte le même nom (« Fermer index.ts · board »).

### Colonne de gauche

- Deux vues, « Fichiers » et « Rechercher dans les fichiers », par les boutons en haut de la colonne (« Vue de la colonne »). Chacune est gardée : défilement de l'arbre, résultats dépliés.
- Une poignée entre la colonne et le code la redimensionne (« Largeur de la colonne des fichiers ») : glisser, ← → par 16 px, Début / Fin jusqu'aux bornes, double-clic pour revenir à 240 px. De 160 px à la moitié de l'éditeur.
- La largeur choisie vaut pour tous les projets et reste sur ce poste. Une fenêtre trop étroite la dessine plus petite sans l'oublier.
- Un relâchement perdu (bouton relâché hors de la fenêtre, fenêtre qui perd le focus) termine le glissement.

### Arborescence

- À la manière de l'explorateur de VS Code : les fichiers connus de git (ignorés exclus), dossiers d'abord ; chevrons, guides d'indentation, icône colorée par type de fichier.
- Un fichier modifié ou ajouté a son nom coloré et sa lettre M ou A ; ses dossiers ont un point coloré.
- Un dossier qui ne contient qu'un dossier partage sa ligne (`src/lib/editor`).
- Le fichier affiché est amené à l'écran.
- Les fichiers ignorés par git qui correspondent aux « Fichiers copiés dans les worktrees » du projet (voir *Git & fichiers*) sont montrés en grisé, avec l'infobulle « Ignoré par git ». Les motifs sont lus depuis la racine de l'arborescence et, pour un projet dans un sous-dossier de son dépôt, depuis ce dossier. À la racine du projet comme dans un worktree, ces fichiers s'ouvrent, se modifient et se retrouvent avec `Ctrl+P` ; ils restent hors des commits.
- Aucun autre fichier ignoré n'apparaît. Un dossier entièrement ignoré n'est pas parcouru.
- En tête : « Nouveau fichier », « Nouveau dossier », « Actualiser », « Tout réduire ».
- Clavier : ↑ ↓ Début Fin ; → déplie un dossier ou y entre ; ← le replie ou remonte au parent ; Entrée ouvre un fichier ou (dé)plie un dossier ; `F2` renomme la ligne ; `Suppr` (`⌘⌫` sous macOS) la supprime. Avec une touche de modification en plus, ni `F2` ni `Suppr` n'agissent.
- Clic droit sur une ligne :
  - « Nouveau fichier… », « Nouveau dossier… » ;
  - « Ouvrir un terminal ici » : dans le dossier, ou celui du fichier, de la source affichée ; le terminal est nommé d'après le dossier ;
  - « Afficher dans l'Explorateur » (« Afficher dans le Finder » sous macOS, « Ouvrir le dossier contenant » sous Linux) : le gestionnaire de fichiers s'ouvre, le fichier ou le dossier sélectionné ;
  - « Renommer… » (F2), « Supprimer » (Suppr) ;
  - « Copier le chemin », « Copier le chemin relatif ».
- Clic droit sur l'espace libre : le premier groupe seulement, à la racine. La racine ne se renomme ni ne se supprime. « Afficher dans l'Explorateur » y ouvre le dossier de la source (celui du projet, ou le worktree de l'agent), comme un terminal ; il s'appelle « Ouvrir le dossier » sous Linux.
- Un worktree disparu est refusé : « Ouverture du gestionnaire de fichiers impossible : Le worktree de l'agent n'existe plus ».

### Créer un fichier ou un dossier

- « Nouveau fichier » crée dans le dossier de la dernière ligne cliquée, sinon dans celui du fichier affiché. Au clic droit : dans le dossier, à côté du fichier, ou à la racine sur l'espace libre.
- Le nom se tape dans l'arbre, icône comprise ; `dossier/nom.ts` crée les dossiers.
- Entrée crée le fichier vide et l'ouvre, le curseur dedans. Échap annule. Quitter le champ crée si le nom est valide ; passer à une autre fenêtre ne décide rien.
- Un refus du disque s'affiche sous le champ ; Entrée réessaie.
- Refusés, avec la raison sous le champ :
  - un nom déjà pris, casse ignorée (« « .env » existe déjà à cet endroit (ignoré par git). » pour un fichier copié montré grisé) ;
  - un chemin qui commence ou finit par `/`, `.` ou `..` ;
  - sous Windows, un nom qu'il modifierait ou réserve (`a.`, `?`, `CON`…).
- Un fichier créé qui est ignoré par git s'ouvre, avec un message : l'arborescence ne le montre pas.
- « Nouveau dossier » : le même champ, sans icône de fichier. Refusé quand quoi que ce soit est déjà là. Le dossier créé s'affiche vide jusqu'à ce qu'un fichier y soit (il est oublié au changement de source) et devient le dossier du prochain « Nouveau fichier ».

### Renommer

- Le champ prend la place de la ligne, prérempli, l'extension hors de la sélection (le nom entier pour un dossier ou `.env`). Une ligne qui regroupe des dossiers (`src/lib/editor`) renomme le plus profond.
- `dossier/nom.ts` déplace dans des dossiers créés au besoin. Un déplacement refusé par le disque retire ceux de ces dossiers restés vides.
- Un gros dossier se déplace sans retenir le reste de l'app.
- Refusés :
  - un nom pris par autre chose, un dossier vide compris, sauf un simple changement de casse d'un fichier ou d'un dossier que git ne suit pas ; pour ce que git suit : « git suit « src/App.ts » sous ce nom : pour n'en changer que la casse, passe par git mv dans un terminal. » ;
  - un dossier dans lui-même ;
  - les noms que Windows modifierait ;
  - le chemin d'un fichier ouvert avec des modifications non enregistrées : « « x » est ouvert avec des modifications non enregistrées. ».
- Le déplacement ne remplace jamais ce qui apparaîtrait entre-temps à l'arrivée.
- Les onglets du fichier, ou des fichiers du dossier, suivent tout de suite, avec leur texte non enregistré, leur historique d'annulation, leur curseur et leur comparaison. L'historique de navigation, les fichiers récents de `Ctrl+P` et les dossiers dépliés suivent aussi.
- Un fichier non enregistré apparu au nouveau chemin pendant le renommage laisse les onglets à l'ancien nom : « Les onglets restent à l'ancien nom : « x » est ouvert avec des modifications non enregistrées. ».
- Toasts : « env.txt n'est plus ignoré par git : il peut être commité. » (un fichier copié renommé hors des motifs de git) ; « debug.log est ignoré par git : l'arborescence ne le montre pas. ».

### Supprimer

- Vers la corbeille du système, après « Supprimer « nom » ? » : « Il part dans la corbeille. », « Le dossier part dans la corbeille. », « Le dossier et son fichier partent dans la corbeille. », « Le dossier et ses N fichiers partent dans la corbeille. » (comptés dans l'arborescence affichée). Sous macOS, sans demander de piloter le Finder.
- Les fichiers concernés non enregistrés sont demandés d'abord, l'un après l'autre : « Enregistrer », « Ne pas enregistrer », ou annuler, qui arrête tout.
- Un fichier dans lequel on tape pendant la suppression garde son onglet et son texte, sous « Ce fichier a été supprimé. ».
- Un lien est supprimé, pas sa cible.
- Refus de la corbeille : « Suppression impossible : un fichier est peut-être ouvert ailleurs ou protégé » ou « Suppression impossible : le fichier est introuvable ou inaccessible » ; le texte du système va au journal. Un autre refus (un dossier protégé, ci-dessous) s'écrit de même : « Suppression impossible : <raison> ».

### Dossiers protégés

Ils ne se renomment ni ne se suppriment, et rien n'y est créé depuis la source du projet : ni dossier, ni nouveau fichier, ni fichier recréé en enregistrant. Un fichier de worktree déjà là, ouvert par un lien, s'enregistre.

- Les worktrees des agents : `.claude/worktrees` à toute profondeur, le « Dossier des worktrees » du projet quand il est dans la source (voir *Git & fichiers*), ceux que `git worktree list` donne pour le dépôt, et ceux des agents de l'app.
- Le dossier de chaque projet de l'app qui se trouve dans la source.
- Un dossier qui contient l'un d'eux.

Messages : « packages/web est le dossier d'un projet », « packages contient le dossier d'un projet », « sub contient le worktree d'un agent ». La règle est vérifiée sur le chemin écrit et sur le chemin réel (liens suivis, noms courts de Windows comme `WORKTR~1`).

### Le code

- Coloration, recherche dans le fichier (`Ctrl+F`), indentation détectée, marques des lignes modifiées (voir *Voir les changements*).
- `Alt+clic` (Option+clic) ajoute un curseur.
- « Ln, Col » de la barre d'état compte en caractères (un emoji en vaut un), comme toute colonne de la navigation et de la recherche.

### Enregistrer

- `Ctrl+S` ou « Enregistrer » : fins de ligne et BOM conservés, écriture atomique.
- Un fichier changé sur le disque se recharge s'il n'est pas modifié. Sinon, un bandeau « Ce fichier a changé sur le disque. » propose « Recharger », « Comparer » (voir *Voir les changements*) ou « Garder ma version ».
- « Garder ma version » n'écrit que par-dessus la version du disque pour laquelle le bandeau est apparu, ou celle qu'un enregistrement refusé a trouvée. Si l'agent a écrit depuis, rien n'est enregistré et le texte reste : « Le fichier a encore changé sur le disque : rien n'est enregistré, tes modifications sont toujours là. ». Le bouton attend alors une seconde avant d'écraser cette nouvelle version.
- Le disque revenu à la version ouverte : « Rien n'a été enregistré : le fichier est revenu à la version que tu avais ouverte. ».
- Un fichier supprimé du disque : bandeau « Ce fichier a été supprimé. » avec « Fermer » ou « Le recréer en enregistrant ».

### Fichiers non enregistrés

- Fermer un onglet, ou quitter, avec des fichiers non enregistrés demande confirmation.
- « Quitter Escouade ? », annulé, ramène la fenêtre qu'il avait remplacée : la modale « Commit » avec son message ; une autre, comme les réglages, se rouvre. Sauf une fenêtre réglée entre-temps : un commit lancé ou fait (un commit refusé par git revient), une confirmation dont le choix tourne, une mise à jour qui n'est plus prête.
- Supprimer un agent ou fermer un projet prévient des fichiers non enregistrés qui seront perdus.
- Une confirmation qui en enchaîne une autre ouvre une nouvelle fenêtre, qui prend le focus.
- Un fichier non enregistré, un renommage ou une suppression en cours retiennent le redémarrage pour mise à jour (voir *Livraison et mises à jour*).

## Réglages

- Largeur de la colonne des fichiers : par sa poignée, pour tous les projets (ci-dessus).
- Taille du texte : Réglages › Application › « Affichage » › « Taille du texte de l'éditeur » (voir *Réglages*).
- Les « Fichiers copiés dans les worktrees » et le « Dossier des worktrees » du projet décident des fichiers grisés et des dossiers protégés (voir *Git & fichiers*).

## Limites

- Sur un partage réseau sans corbeille, Windows peut supprimer pour de bon alors que la confirmation parle de corbeille.
- Sous macOS, « Remettre » n'est pas proposé dans la corbeille pour un fichier supprimé depuis l'éditeur.
