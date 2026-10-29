---
title: "Tickets"
subject: "Kanban"
sources: ["docs/SPEC.md"]
tickets: ["ESC-95","ESC-141","ESC-153"]
decisions: []
compared: null
---

## Ce que c'est

Un ticket décrit un travail à confier à un agent : un titre, une description, des critères d'acceptation, et au besoin des images et des fichiers. Il peut attendre d'autres tickets, et fixer le harness, le compte, le modèle et l'effort de son agent.

## Comportement

### Ce qu'il contient

- Une clé `<PRÉ>-<n>` : les 3 premières lettres du projet, `TIC` à défaut. Un numéro n'est jamais réutilisé.
- Un titre, une description, des critères d'acceptation. Critères par défaut : « Implémentation conforme au ticket », « Tests verts ».
- « Boucles max » : 3, 5 ou 8.
- « Branche » : « Nouvelle branche ticket/<clé> » par défaut, ou « Reprendre une branche existante… » (voir *Agent d'un ticket*).
- « Après » : les tickets qu'il attend (ci-dessous).
- « Harness » (voir *Harness*), « Compte », « Modèle », « Effort » (ci-dessous).

### La fenêtre du ticket

- Elle s'ouvre par « + » en tête de « À faire » (« Nouveau ticket »), par un clic sur une carte « À faire », ou par « Modifier » au clic droit (« Modifier <clé> »).
- C'est une grande fenêtre : 760 px de large, 88 % de la hauteur au plus et 860 px, un corps qui défile, un pied fixe en bas.
- Le titre est sur toute la largeur. « Description » a 12 lignes (10 quand la fenêtre de l'app fait moins de 820 px de haut) ; elle grandit avec le texte jusqu'à 60 % de la hauteur, puis défile. « Critères d'acceptation » a 6 lignes (5). Dessous, « Boucles max » et « Branche » à gauche, « Après » (recherche et liste) à droite.
- Le focus est sur le titre à l'ouverture. Il revient au « + » ou à la carte à la fermeture.
- `Ctrl+Entrée` (`Cmd+Entrée`) ajoute ou enregistre depuis n'importe quel champ quand le titre est rempli, sans écrire de ligne dans un champ de texte et sans valider deux fois, même sur une touche maintenue. Entrée dans le titre valide aussi. Le pied rappelle « Ctrl+Entrée pour ajouter » (« … pour enregistrer » en modification), avec « Annuler » et « Ajouter » / « Enregistrer ».
- Le ticket est créé dans le projet d'où la fenêtre s'est ouverte, même si un autre est sélectionné entre-temps.
- La fenêtre ne se ferme pas pendant l'enregistrement. Un refus la laisse ouverte avec ce qui était saisi ; la raison est dans un toast.
- Les raccourcis de l'app sont coupés derrière elle. Échap dans le sélecteur de branche ou son menu ne ferme que ceux-là.
- Fermer un ticket modifié (titre, description, images, fichiers, critères, boucles, « Après ») par Échap, ×, un clic à côté ou « Annuler » demande « Abandonner les modifications ? » (« Ce que tu as saisi dans ce ticket ne sera pas enregistré. », bouton « Abandonner »). La fenêtre disparaît derrière la confirmation ; « Annuler » la ramène avec tous ses champs, de même qu'un « Quitter Escouade ? » annulé. Sans changement, elle se ferme tout de suite.

### Les images

- Une image se colle (`Ctrl+V`), se dépose n'importe où dans la fenêtre (ses champs, son en-tête, son pied), ou se choisit par « Joindre un fichier », à côté du libellé « Description » (infobulle « Joindre un fichier (ou colle-le, glisse-le dans la fenêtre) » ; le choix n'a pas de filtre).
- Les vignettes (64 px) sont sous la description, chacune avec une croix « Retirer <nom> ». Avec plusieurs images, le rang s'ajoute au nom (« Retirer image.png (2/3) »), car les captures s'appellent toutes image.png. Après une croix, le focus va à la croix suivante, sinon à la précédente, sinon à la description. La description cède trois lignes aux vignettes.
- Formats et limites du chat : PNG, JPEG, GIF, WebP ; 5 Mo par image, 18 Mo en tout ; 20 images au plus par ticket. Le type est lu dans le contenu : un fichier qui n'est d'aucun de ces formats (un « .png » qui contient autre chose) n'est pas refusé, il devient un fichier du ticket. Seuls la 21e image et le dépassement des 18 Mo sont refusés, par un toast.
- Un texte collé reste un texte, même accompagné d'une image de lui (les cellules d'un tableur).
- À l'ouverture d'un ticket, ses images sont lues (« Chargement de <nom>… » en attendant). Une image dont le fichier a disparu est nommée dans un toast et retirée du ticket à l'enregistrement suivant ; ce retrait n'est pas une modification, la fenêtre se ferme sans demander. Les images ne sont envoyées que si elles ont changé.

### Les autres fichiers

- Tout fichier qui n'est pas une image que le ticket montre (autre format, image de plus de 5 Mo, PDF, texte…) se joint par les mêmes gestes. Il apparaît en puce sous les vignettes, dans la liste « Fichiers du ticket » : icône, nom, taille, croix « Retirer <nom> », avertissement au-delà du seuil des fichiers joints.
- Il est toujours copié, jamais lu là où il est : aussitôt dans un brouillon, puis dans le dossier du ticket à l'enregistrement (voir *Conversation*). « Ajouter » et « Enregistrer » attendent la fin des copies.
- 20 fichiers au plus (« <nom> n'est pas ajouté : un ticket prend 20 fichiers au plus. »), et la taille maximale des fichiers joints (« <nom> dépasse <max> »).
- La croix d'une copie du brouillon l'efface ; celle d'un fichier que le ticket a ne l'efface qu'à l'enregistrement. « Abandonner » efface les copies que la fenêtre a faites ; « Annuler » les garde. Le focus suit la croix comme pour les images ; la description cède deux lignes aux puces.

### Où vivent images et fichiers

- Les images sont des fichiers de `<données>/tickets/<id>/`, les autres fichiers des copies dans `<données>/tickets/<id>/files/`. Ils sont effacés avec le ticket, ou quand l'historique de son projet fermé est effacé ; fermer le projet les garde avec son historique, et leur taille compte dans celle de l'historique (voir *Fermer un projet*).
- L'agent du ticket reçoit les images en blocs image avant le texte de son premier message, au démarrage et à la reprise d'un agent qui ne les a jamais reçues. Elles restent dans sa session pour les boucles suivantes.
- Il reçoit une ligne `Fichier joint : <nom>, <chemin>` par fichier, à la fin de son premier message, et lit le dossier des fichiers du ticket sans demander ; ce dossier ne lui est ouvert que quand le ticket a des fichiers.
- Le serveur MCP joint, liste, lit et transmet les images d'un ticket aux mêmes limites et avec les mêmes refus que la fenêtre. Il dit combien le ticket a de fichiers, mais ne les touche pas (voir *Serveur MCP*).

### Sur la carte

- Un trombone avec le nombre d'images et de fichiers ensemble : « 2 images jointes », ou « 1 image jointe, 2 fichiers joints », au survol et dans la description de la carte pour un lecteur d'écran.
- Clic droit sur une carte « À faire » : « Modifier », « Passer en tête », « Supprimer ».
- « Supprimer » demande « Supprimer DEM-1 ? » : « Le ticket et sa description sont supprimés. », plus « Il ne sera plus importé depuis Jira. » pour un ticket importé. Avec des images : « Le ticket, sa description et ses 2 images sont supprimés. » (« … et son image … » pour une seule). Avec des fichiers, images comprises : « Le ticket, sa description et ses 3 pièces jointes sont supprimés. » (« … et sa pièce jointe … » pour une seule). Bouton « Supprimer ».

### Compte, modèle et effort

- Ils se règlent dans la fenêtre, à côté de « Harness », en création comme en modification, ou par le serveur MCP (création, modification, lancement). Chacun vaut « Auto » par défaut et dit ce qu'il donne : « Auto · Principal », « Auto · Opus 5.5 (Kanban) », « Auto · Moyen (compte) ».
- « Compte » : « Auto » (le compte courant du harness du ticket pour son projet), puis les comptes de ce harness que le projet peut utiliser (pour un projet privé, ceux qui lui sont permis). Changer de harness le remet sur « Auto ».
- « Modèle » : « Auto » (celui du Kanban, sinon le défaut du compte), puis les modèles qu'annonce le compte choisi (le compte courant sur « Auto »), comme le menu de la zone de saisie. Un compte qui n'a pas le modèle choisi le remet sur « Auto ».
- « Effort » : « Auto », puis les niveaux du modèle ; grisé avec sa raison pour un modèle qui n'en prend pas. Un modèle qui n'a pas le niveau choisi le remet sur « Auto ».
- Chaque remise sur « Auto » est dite sous les champs : « « Modèle » remis sur « Auto » : Pro n’a pas Opus. »
- L'agent part sur ce compte, ce modèle et cet effort à chaque démarrage : pilote auto, « Lancer », serveur MCP, ticket renvoyé ou repris. Le modèle et l'effort du ticket passent avant ceux du Kanban. Un ticket renvoyé puis relancé les garde.
- Un ticket fixé sur un compte y reste. Ni « Changer de compte à » ni la bascule à la limite, vers un autre compte ou un autre harness, ne le déplacent. À la limite, il attend la remise à zéro et sa carte le dit (« En attente du quota de Pro, reprise à 15:30 ») ; au seuil, sa conversation le dit. Il ne démarre que quand ce compte est libre, même si un autre l'est.
- Un compte supprimé, désactivé ou déconnecté fait repasser le ticket en « Auto ». Sa carte le dit (« Le compte Pro n’existe plus : repassé en « Auto ». », jusqu'au prochain enregistrement), une notification aussi, et il démarre sur le compte courant.
- Un compte que le projet ne peut pas utiliser (réservé à d'autres projets, ou partagé alors qu'un projet privé a les siens) est refusé avec sa raison, par la fenêtre comme par le serveur MCP.
- Sur la carte, dans toutes les colonnes :
  - le compte, quand il n'est pas sur « Auto » : « Pro », au survol « Compte de ce ticket : Pro (ses agents y restent) » ;
  - le modèle et l'effort, dans une étiquette à côté de la clé : « Opus · Max », au survol et pour un lecteur d'écran « Modèle de ce ticket : Opus, effort Max (avant celui du Kanban) ». L'effort n'y est pas quand le modèle de l'agent (celui du ticket, sinon du Kanban, sinon de l'app) n'en prend pas, comme Haiku : « Haiku » seul. Un effort seul sur un Kanban en Haiku ne donne pas d'étiquette.

### Les dépendances (« Après »)

- « Après » liste les autres tickets du projet pas encore « Terminé » (« DEM-n Titre », du plus ancien au plus récent), avec « Rechercher une clé » : la clé interne, ou la clé externe d'un ticket importé. Les tickets cochés restent visibles ; « Aucun ticket pour cette clé ». La section n'apparaît que s'il y a un candidat.
- Cocher un ticket qui attend déjà celui-ci, directement ou non, est refusé : « DEM-3 attend déjà DEM-5 (directement ou non). ». Le cœur refuse aussi une boucle, et ne garde que des tickets du projet, chacun une fois, jamais le ticket lui-même.
- Supprimer un ticket le retire des tickets qui l'attendaient.
- Sur la carte « À faire » : « ⏸ après DEM-3 » (« ⏸ après DEM-3 et DEM-4 ») tant qu'un ticket attendu n'est pas « Terminé ».
- « Lancer » demande alors « Lancer DEM-5 ? » (« DEM-5 attend DEM-3, pas encore terminé. Le lancer quand même ? », bouton « Lancer quand même »). Un ticket lancé à la main part sans attendre.
