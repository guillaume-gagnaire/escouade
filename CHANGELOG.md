# Journal des versions

Les changements visibles d'Escouade, version par version. Les notes de chaque release GitHub (et de la mise à jour intégrée) reprennent la section de sa version.

## [1.5.2] — 2026-10-07

### Ajouts

- Réglages › Réseau › Certificats : « Ignorer la vérification des certificats TLS », pour un proxy d'entreprise qui déchiffre le trafic avec son propre certificat (erreur « UnknownIssuer », par exemple en se connectant à Jira). L'option vaut pour les intégrations, les quotas, les mises à jour et les processus Claude Code, avec les commandes que lancent les agents. Désactivée par défaut, elle est à réserver à un réseau de confiance.

### Modifications

- Les intégrations, les quotas et les mises à jour font aussi confiance aux certificats installés sur le système (le plus souvent, celui du proxy de l'entreprise) : pour eux, l'option ci-dessus n'est alors pas nécessaire.
- La recherche et l'installation des mises à jour suivent les réglages réseau du moment (proxy et certificats).

## [1.5.1] — 2026-10-06

### Ajouts

#### Worktrees : préparation et démontage

- Nouveau groupe « Worktrees » dans l'onglet « Projets » des réglages : des commandes lancées dans chaque nouveau worktree, l'une après l'autre (installer les dépendances, générer du code…), pour l'agent d'un ticket comme pour un agent créé à la main. Chacune a son shell et son sous-dossier.
  - Pendant la préparation, l'agent affiche « Préparation… », Claude Code ne démarre pas encore et ses messages attendent qu'elle soit finie : Claude ne travaille jamais sur un worktree à moitié installé.
  - Archiver ou supprimer l'agent, fermer le projet ou quitter Escouade arrête la préparation.
  - Un échec s'affiche dans la conversation avec les dernières lignes de la commande ; l'agent d'un ticket le lit à la suite de son premier message.
- Des commandes de démontage, lancées dans le worktree juste avant qu'Escouade le supprime (agent supprimé avec son worktree, ticket validé), pour défaire ce que la préparation a créé ailleurs (base de données, conteneurs…).
- « ✦ Remplir automatiquement » : Claude lit le projet en arrière-plan (sans rien modifier ni exécuter) et propose les commandes, à relire avant d'enregistrer.
- Les commandes disposent de `ESCOUADE_PROJECT_DIR`, `ESCOUADE_WORKTREE_DIR`, `ESCOUADE_BRANCH`, et des ports réservés d'un ticket.

#### isola

- Quand [isola](https://github.com/cyucelen/isola) est installé et que le projet a un `.isola.toml`, isola lance les services de chaque worktree sur ses propres ports :
  - l'agent d'un ticket ne reçoit plus de bloc de ports, et on ne lui demande plus de recette, seulement l'adresse de la fonctionnalité ;
  - « ▶ Tester » lance `isola up`, attend que chaque service réponde, puis ouvre la fonctionnalité dans le navigateur ;
  - « Tout arrêter », la validation, l'archivage et la fermeture du projet arrêtent les services (`isola down`) ; supprimer le worktree supprime aussi ses données (`isola destroy`).

### Modifications

- Une fois un ticket validé par « Valider + PR » ou « Valider et pousser », son worktree est supprimé lui aussi (sa branche reste). Le réglage s'appelle désormais « Supprimer le worktree une fois validé ».
- Restaurer un agent archivé dont le worktree a été supprimé le recrée depuis sa branche, avec ses fichiers copiés et sa préparation.

### Corrections

- Un ticket validé sans aucune modification de code passe « Terminé » avec « ∅ Aucune modification », au lieu de rester bloqué sur « Rien à merger ».

## [1.5.0] — 2026-10-04

### Ajouts

#### Éditeur : une arborescence à la manière de VS Code, et la création de fichiers

- L'arborescence de l'éditeur prend l'allure de l'explorateur de VS Code. On y retrouve :
  - des chevrons, des guides d'indentation et une icône colorée par type de fichier ;
  - le nom en couleur d'un fichier modifié (M) ou ajouté (A), et un point sur les dossiers qui le contiennent ;
  - un dossier qui ne contient qu'un autre dossier sur la même ligne que lui (`src/lib/editor`).
- Le fichier affiché défile jusqu'à l'écran.
- Au clavier : ↑ ↓ Début Fin pour se déplacer, → pour déplier un dossier ou y entrer, ← pour le replier ou remonter au parent.
- En tête de l'arborescence : « Nouveau fichier », « Actualiser » et « Tout réduire ».
- Nouveau fichier : le nom se tape directement dans l'arborescence, et l'icône suit l'extension.
  - Le fichier va dans le dossier de la dernière ligne cliquée, ou à côté du fichier affiché. Au clic droit, « Nouveau fichier… » le crée dans le dossier, à côté du fichier, ou à la racine depuis l'espace libre.
  - `dossier/nom.ts` crée les dossiers manquants.
  - Entrée crée le fichier vide et l'ouvre, le curseur dedans ; Échap annule.
  - Un nom refusé affiche la raison sous le champ : fichier déjà présent (sans tenir compte de la casse), chemin invalide, ou nom que Windows modifierait ou réserve (`a.`, `?`, `CON`…).
  - Un fichier créé mais ignoré par git s'ouvre quand même, avec un message : l'arborescence ne l'affiche pas.
- Clic droit sur une ligne : « Copier le chemin » et « Copier le chemin relatif ».

### Corrections

- Un dossier nommé comme un membre des objets JavaScript (`constructor`, `toString`…) s'ouvre désormais dans l'arborescence.

## [1.4.0] — 2026-10-04

### Ajouts

#### Intégrations : Jira, Trello et GitHub Issues

- Onglet « Intégrations » des réglages : connecte un compte Jira Cloud (site, e-mail, jeton d'API), Trello (clé d'API et jeton ; « Obtenir un jeton » ouvre la page d'autorisation de Trello) ou GitHub (un jeton, ou à défaut celui de `gh`). La connexion est vérifiée tout de suite, et un refus s'affiche sous le formulaire. Les jetons restent dans `~/.escouade/integrations.json`, à part des réglages, et la fenêtre de l'app ne les voit jamais.
- Par projet, une source par service : un projet Jira, un tableau Trello ou un dépôt GitHub (celui du projet en tête de liste).
- « ⤓ Importer », dans l'en-tête du Kanban :
  - une pastille par source liée, une recherche et les filtres du service (Jira : Assignés à moi, Sprint actif, À faire ; Trello : Mes cartes et une par liste ; GitHub : Assignées à moi et une par étiquette) ;
  - les tickets à cocher, avec leur type, leur priorité, leur assigné, leur statut et le nombre de critères détectés ; ceux déjà importés sont grisés, « Déjà dans le Kanban » ;
  - le nombre de boucles max, puis « Importer » : les tickets arrivent à la fin de « À faire ».
- Un ticket importé reprend le titre et la description d'origine (le texte Jira converti en markdown), avec le lien vers le ticket. Ses critères d'acceptation viennent de la liste sous « Critères d'acceptation », « Acceptance criteria » ou « Definition of done », sinon de ses cases à cocher (Trello : de la checklist de ce nom, sinon de toutes) ; à défaut, ce sont les critères par défaut. Sa carte porte la pastille du service et la clé d'origine (`ATL-1287`, `#42`), qui ouvre le ticket dans le navigateur.
- Synchro des statuts : quand un ticket importé change de colonne, son pendant suit (transition Jira, liste Trello, issue GitHub ouverte, fermée ou étiquetée). Choisir une source pré-remplit la correspondance : « En cours » et « À tester » vers l'état qui ressemble à « en cours », « Terminé » vers celui qui ressemble à « à tester ». Elle se règle par colonne et par source.
- Commentaires sur le ticket d'origine, à l'arrivée dans les colonnes cochées (« À tester » et « Terminé » par défaut) : pris par un agent, prêt à tester (critères et ce qui a été fait), terminé (issue de la validation), revenu « À faire ». En option, un résumé à chaque boucle.
- Une synchro ratée ne bloque jamais le ticket : sa carte montre ⚠ et la raison, jusqu'à la suivante qui réussit.
- Import automatique (désactivé par défaut) : toutes les 5, 15 ou 60 minutes, les tickets ouverts des sources liées qui portent l'étiquette choisie (`claude-ready` par défaut) arrivent dans « À faire ». Un ticket supprimé du Kanban n'est jamais ramené.

### Modifications

- Une seule fenêtre de réglages pour toute l'app, en onglets : Claude Code, Notifications, Projets, Kanban, Intégrations, Terminaux, Réseau, À propos. Elle remplace « Réglages du tableau » et « Commandes de lancement ».
  - « Enregistrer » enregistre d'un coup ce qui a changé, dans tous les onglets et pour tous les projets ; un point marque les onglets modifiés. « Annuler », Échap ou × jettent tout.
  - « ⚙ Après validation » ouvre l'onglet Kanban du projet, le ⚙ de la section Lancement son onglet Projets. Au clic droit sur un onglet de projet, « Réglages du projet… » remplace « Commandes de lancement… ».
- Le tableau s'appelle désormais « Kanban » (sélecteur « Agents | Kanban »).

## [1.3.1] — 2026-10-04

### Corrections

- Quand un ticket passe « Terminé », les terminaux de test de son agent quittent la section « Lancement », avec leurs logs, et sa fenêtre « Tester » se ferme.
- Un lancement relancé pendant l'arrêt du précédent ne démarre plus si sa commande a disparu entre-temps (agent archivé ou supprimé, ticket terminé, projet fermé…).

## [1.3.0] — 2026-10-04

### Ajouts

#### Tableau : des tickets que des agents prennent seuls

- Sélecteur « Agents | Tableau » en haut de la barre latérale, avec une pastille jaune qui compte les tickets « À tester ». Le tableau d'un projet remplace la zone principale : À faire, En cours, À tester, Terminé.
- Tickets avec une clé tirée du projet (DEM-1, DEM-2…), un titre, une description et des critères d'acceptation, un par ligne. Sans critère, deux par défaut : « Implémentation conforme au ticket » et « Tests verts ». On règle aussi le nombre de boucles max : 3, 5 ou 8. Un ticket « À faire » se modifie, passe en tête, se supprime ou se lance à la main (« Lancer »).
- Pilote auto : des agents se créent seuls pour prendre les tickets, dans l'ordre, jusqu'au nombre d'agents en parallèle choisi (1 à 6, 2 par défaut). Rien ne part tant qu'un agent attend la fin de sa limite d'usage.
- Chaque agent de ticket a son worktree `ticket/<clé>`, créé depuis la branche cible, et un bloc de 10 ports réservé à partir de 4100. Il reçoit aussi les fichiers du projet que git ignore (`.env*` par défaut, liste réglable dans les commandes de lancement) : ces copies ne peuvent jamais être commitées.
- L'agent boucle jusqu'au but. À chaque fin de tour, il fait le bilan de ses critères. S'il en manque, il repart avec les critères manquants et leurs notes. Tous atteints, le ticket passe « À tester ». À la limite de boucles, il y passe aussi, avec « Objectif partiel ».
- Un ticket bloqué libère sa place : interruption, erreur, ou bilan manquant après un rappel. « Reprendre » le relance.
- Avancement : l'agent tient une liste courte des fonctionnalités en place. Les cartes « En cours » en montrent les 3 dernières (« +n » s'il y en a d'autres). Les cartes « À tester » montrent la liste complète sous « Ce qui a été fait ».
- Les cartes « En cours » montrent la boucle en cours (« Boucle 2/5 »), les critères ✓ / ○ et une barre des critères atteints. Elles montrent aussi l'activité de l'agent en direct (« Lit src/db.ts », « Lance npm test »…) ou « Question en attente de ta réponse ».
- La carte d'un agent de ticket, dans la barre latérale, porte « ▸ DEM-1 · boucle 2/5 ».
- Validation réglable par projet (« ⚙ Après validation ») :
  - quatre actions : merger dans une branche (merge commit, squash ou rebase), ouvrir une pull request, pousser la branche du ticket, ou laisser en l'état ;
  - en option, relancer les tests avant : en échec, le ticket repart chez son agent avec la fin de la sortie ;
  - un message de commit généré au format Conventional Commits, avec la clé du ticket ;
  - la suppression du worktree après merge ;
  - en cas de conflit : me demander, l'agent résout, ou annuler.
- Pull request : par `gh` quand il est installé, avec la description, l'avancement et les critères. Sinon, la page GitHub de la pull request s'ouvre pré-remplie.
- « Renvoyer » rend un ticket « À tester » à son agent avec ce qui ne va pas.
- Garde-fous de la validation :
  - les fichiers copiés du projet (`.env`…) ne partent jamais dans un commit, une branche ou un dépôt distant ;
  - rien n'est validé ni renvoyé pendant que l'agent travaille ;
  - un seul merge à la fois par dépôt ;
  - refus clair quand la branche n'a rien de nouveau (« Rien à merger / proposer / pousser »).
- Reprise : un ticket coupé par l'arrêt de l'app repart au démarrage suivant, et une validation coupée laisse « Validation interrompue ». Archiver ou supprimer l'agent d'un ticket le renvoie « À faire », après confirmation.
- Dans la conversation, le bilan de l'agent s'affiche comme une carte « Bilan des critères » : ✓ / ○, notes, avancement, puis la recette de lancement s'il y en a une.
- Notifications « DEM-1 prêt à tester » et « DEM-1 bloqué : <raison> » ; un clic ouvre le tableau du projet. Les fins de tour d'un agent de ticket en cours ne notifient plus (ses questions, si).
- L'en-tête du tableau donne les places libres et l'état du quota. Il dit aussi pourquoi rien ne démarre : Claude Code introuvable, ou branche cible absente ou sans commit. Le premier commit d'un projet neuf relance la file.
- « Réglages du tableau » : action à la validation, branche cible, stratégie, brouillon de PR, tests, message de commit, conflits, nombre d'agents en parallèle et leur modèle.

#### Lancement de test, sur des ports à part

- Pour tout agent à worktree, « Préparer le lancement » lui demande sa recette sur ses ports réservés : préparation, processus à lancer, adresse de la fonctionnalité à ouvrir.
- « ▶ Tester » ouvre une fenêtre qui fait tout :
  - elle prépare (une fois par recette) et lance chaque serveur ;
  - elle attend que chacun réponde en HTTP (3 min max) ;
  - elle ouvre le navigateur directement sur la fonctionnalité développée ;
  - « Rouvrir », « Voir les logs » et « Tout arrêter » restent à portée.
- Les étapes de la recette apparaissent dans la section « Lancement », sous le nom de l'agent, avec leurs logs. Elles s'arrêtent à la validation, à l'archivage ou à la suppression de l'agent, et à la fermeture du projet. Une nouvelle recette arrête les lancements de l'ancienne.

#### Mises à jour

- L'app cherche une nouvelle version toutes les cinq minutes, plus seulement au lancement.
- Les notes de version viennent de ce journal.

### Corrections

- Les liens de la conversation, des terminaux et du contrôle à distance s'ouvrent de nouveau dans le navigateur.

## [1.1.0] — 2026-10-03

### Ajouts

- Mode éditeur dans un projet :
  - arborescence, onglets et code coloré ;
  - enregistrement par `Ctrl+S` ;
  - lignes modifiées marquées par rapport à la version de référence ;
  - avertissement quand un fichier change sur le disque ;
  - fins de ligne conservées, et jamais d'écriture hors du dossier du projet ou du worktree.
- L'éditeur s'ouvre depuis l'en-tête d'un agent (« Éditeur »), le clic droit sur un agent, « Parcourir » sur la branche du projet et la barre d'onglets. Il s'ouvre aussi depuis les fichiers non commités (bouton `</>`) et les fichiers modifiés cités dans la conversation.
- Les modèles affichent la version que Claude Code utilise (par exemple Sonnet 5.5).
- Quitter avec des fichiers non enregistrés demande confirmation.

### Modifications

- L'éditeur intégré remplace l'ouverture des fichiers et des projets dans un éditeur externe (réglage et menus retirés).

### Corrections

- Une release créée depuis la page GitHub prend la version de son tag, et une release incomplète ne bloque plus les mises à jour.
