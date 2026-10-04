# Journal des versions

Les changements visibles d'Escouade, version par version. Les notes de chaque release GitHub (et de la mise à jour intégrée) reprennent la section de sa version.

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
