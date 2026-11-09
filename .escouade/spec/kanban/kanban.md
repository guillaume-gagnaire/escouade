---
title: "Kanban"
subject: "Kanban"
sources: ["docs/SPEC.md","design/kanban-envs-2-2/README.md"]
tickets: ["ESC-95","ESC-123","ESC-141","ESC-143","ESC-153","ESC-190","ESC-243","ESC-59","ESC-63","ESC-390","ESC-393"]
decisions: [129]
compared: null
order: 30
---

## Ce que c'est

Le Kanban d'un projet range ses tickets en quatre colonnes : « À faire », « En cours », « À tester », « Terminé », et une cinquième, « En revue », entre « À tester » et « Terminé », quand il ouvre des pull requests. Chaque ticket lancé est confié à un agent, dans un worktree à lui, qui boucle jusqu'à atteindre ses critères d'acceptation. L'utilisateur teste puis valide : merge, pull request, push ou rien. Le pilote auto lance les tickets seul, dans la limite des places.

## Comportement

### Ouvrir le Kanban

- L'onglet « Kanban », en haut de la barre latérale, passe à cette vue (voir *Barre latérale*). Sa pastille jaune compte les tickets « À tester ».
- Le Kanban remplace la zone principale ; l'éditeur se ferme. Choisir un agent, un terminal ou une commande revient aux agents.
- Tickets et réglages sont enregistrés avec l'état de l'app.

### L'en-tête

Deux rangées (direction A, choisie sur aperçu le 2026-11-08, ESC-390), où rien ne se chevauche à aucune largeur de fenêtre :

- **Première rangée** : « Kanban » (« Environnements » dans cette vue), puis « <projet> · N tickets · k en boucle », coupé avec son infobulle quand il est long ; à droite « ⤓ Importer » (voir *Intégrations*) et l'interrupteur « Pilote auto » (« Pilote auto · off » éteint, voir *Pilote auto*).
- **Seconde rangée**, sous un filet : à gauche les vues du Kanban, soulignées comme des onglets, « Tickets N » et « Environnements N » (voir *Environnements et déploiements*) ; « Environnements » n'existe que pour un projet dont l'hébergeur de code lit des déploiements. À droite, dans la vue des tickets :
  - la CI de la branche cible, en pastille : « main ✗ depuis 2 h », « main ✓ », « main · CI en cours » ; un clic ouvre ses jobs, leurs journaux et « Relancer les jobs en échec » (voir *CI de la cible et des branches poussées*). Pas de pastille sans hébergeur lu, sans compte, ou sans CI sur la cible ;
  - les places, sur une ligne : « n places libres », « Toutes les places sont prises », ou « Quota atteint — reprise à HH:MM » ;
  - « Après validation : <résumé> ▾ », qui ouvre les réglages du Kanban, sur la page « Kanban » du projet.
- Quand aucun ticket ne peut démarrer, une phrase en rouge prend la place des places libres :
  - « Aucun harness trouvé — aucun ticket ne démarre » ;
  - « <cible> n'a encore aucun commit — aucun ticket ne démarre » : dans un dépôt neuf, son premier commit lance la file ;
  - « Branche cible <cible> introuvable — aucun ticket ne démarre ».
  Dans les deux derniers cas, les cartes « À faire » disent « En attente de la branche cible ».
- Dans une zone de travail de 1000 px ou moins (une fenêtre de 1024 px, par exemple), « Après validation : » ne laisse que son résumé (entier dans l'infobulle) ; le bouton garde pour nom « Après validation : <résumé> » à toutes les largeurs, pour un lecteur d'écran (ESC-393). De 860 px ou moins, « Importer » garde son icône seule et les onglets perdent leurs comptes. Un texte trop long se coupe, entier dans son infobulle.
- Pilote auto en pause : une ligne sous l'en-tête le dit, avec « Reprendre maintenant », et les places libres ne s'affichent pas (voir *Pilote auto*). En pause sur une cible rouge : « Pilote auto en pause : la CI de main est rouge depuis 2 h », avec « Voir les jobs ».

### La branche cible

Le premier ticket fixe la branche cible : celle du projet, même sans commit. Sur une HEAD détachée, il est refusé : « Le projet n'est sur aucune branche : choisis la branche cible dans les réglages du Kanban. »

### La vie d'un ticket

1. Il est créé dans « À faire » : à la main, par le serveur MCP, ou importé d'une source (voir *Tickets*, *Intégrations*).
2. Le pilote auto ou « Lancer » le fait partir : son agent travaille dans un worktree, le ticket passe « En cours » (voir *Pilote auto*, *Agent d'un ticket*).
3. Ses critères atteints, ou ses boucles épuisées, il passe « À tester ». On peut lancer l'app de sa branche pour l'essayer (voir *Lancement de test*).
4. Validé, il passe « Terminé » ; renvoyé, il repart chez son agent (voir *Validation*). Validé par une pull request ouverte, il passe « En revue », puis « Terminé » une fois mergée sur l'hébergeur (voir *En revue*). Validé par « Valider et pousser », sa carte « Terminé » montre la CI de sa branche (voir *CI de la cible et des branches poussées*).

Un ticket peut se partager avec l'équipe par un fichier du dépôt (voir *Tickets partagés*) : « En revue » s'y écrit `in-review`. Les enchaînements que suivent les agents du projet sont décrits dans *Workflows*.

## Réglages

Dans la page « Kanban » du projet :

- l'action après validation : « Valider et merger », « Valider + PR », « Valider et pousser » ou « Valider » ; la façon de merger (merge commit, squash ou rebase), la PR en brouillon, ses relecteurs et ses étiquettes, la conduite en cas de conflit (voir *Validation*) ;
- le nombre de places : le maximum de tickets « En cours » non bloqués, de 1 à 6, 2 par défaut ;
- « Pause quand la CI de la cible est rouge », éteint par défaut (voir *Pilote auto*) ;
- la commande de tests, facultative ;
- le modèle, l'effort et le mode des agents de ticket ;
- la branche cible ;
- « Supprimer le worktree d'un ticket terminé ou supprimé », allumé par défaut.

Ailleurs :

- « Préfixe des branches des tickets » du projet (`ticket/`) ;
- les fichiers ignorés par git copiés dans le worktree, `.env*` par défaut ;
- « Pause au-delà du quota », réglage de l'app (voir *Pilote auto*).
