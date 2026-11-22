---
title: "Kanban"
subject: "Kanban"
sources: ["docs/SPEC.md","design/kanban-envs-2-2/README.md","Demande de l'utilisateur du 2026-11-21 (ESC-620) : « Ajoute la possibilité de monter plus haut que 6 tickets en parallèle (typiquement sur serveur, on pourrait monter plus haut). »","crates/escouade-model/src/parallel.rs, src-tauri/src/parallel.rs (ESC-620)"]
tickets: ["ESC-95","ESC-123","ESC-141","ESC-143","ESC-153","ESC-190","ESC-243","ESC-59","ESC-63","ESC-390","ESC-393","ESC-479","ESC-620"]
decisions: [129,281]
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

## Les tickets en parallèle (ESC-620)

« En parallèle » est le maximum de tickets « En cours » non bloqués (et de tickets « En revue » dont l'agent corrige la CI), 2 par défaut. Une seule borne, dans le modèle (`escouade_model::parallel`), vaut partout : le Kanban qui démarre les tickets, le pilote auto, `start_ticket` et `set_board` du serveur MCP, la fenêtre (Réglages › Kanban et l'assistant « Nouveau projet »), l'API distante.

- **Sur un poste**, de 1 à **16**.
- **Sur un serveur** (`escouade-server`, et l'espace d'un serveur vu depuis un client : c'est le cœur du serveur qui répond), de 1 à **64**.
- **Un plafond imposé** l'abaisse, jamais ne la relève : la stratégie du poste géré ou le `enforced` du `.escouade.yml` d'équipe, par la clé `projects.maxParallel` (voir *Réglages imposés*). Entre les deux, le plus bas tient ; celui de l'équipe ne vaut que pour ses projets.
- **La valeur réglée est gardée** quand un plafond arrive : le Kanban ne démarre pas plus de tickets que la borne, `get_board` montre la valeur ramenée dans la borne et dit la borne (`maxParallelLimit`). Le plafond retiré, la valeur réglée revient.

### Dans la fenêtre

- Le réglage est un champ de nombre, « De 1 à N » à côté (N : 16, 64, ou le plafond). Un nombre tapé au-delà est ramené à la borne quand on quitte le champ.
- Un plafond imposé se dit sous le réglage, avec un cadenas et sa provenance : « Plafond de 8, imposé par la stratégie de ce poste », « Plafond de 4, imposé par l'équipe Acme ». La ligne n'est pas grisée : on règle en dessous.
- **Au-delà de 6**, un avertissement discret sous le réglage, d'après la machine qui fait tourner le Kanban (le serveur pour l'espace d'un serveur) : « Au-delà de 6 : ce poste (16 cœurs, 32 Go) tient environ 4 agents qui compilent et lancent des tests (≈ 6 Go chacun, mesuré sur le dépôt d'Escouade). Le quota du compte part aussi plus vite. » Le nombre d'agents tenus est le plus petit de ce que permettent les cœurs et la mémoire, à la place d'un agent mesurée (voir ci-dessous). C'est une indication, **jamais un refus**.

### La place d'un agent

Mesurée le 2026-11-22 sur le dépôt d'Escouade (Windows 11, 32 cœurs, 64 Go), la mémoire privée de chaque arbre de processus relevée toutes les 2 s : Claude Code lui-même (environ 0,3 Go), une compilation de l'app (`npx tauri build --debug --no-bundle`, le crate `escouade` refait : 4,7 Go en moyenne, 14 à 17 Go au pic) et un e2e (l'app et sa WebView : 0,6 Go en moyenne, 1 Go au pic). Un agent prend donc environ **6 Go** et **2 cœurs** (1,2 mesuré : temps CPU sur temps écoulé), arrondis vers le haut : les constantes `AGENT_MB` et `AGENT_CORES` du modèle. Les pics de plusieurs agents se rencontrent rarement ; la moyenne sert à l'avertissement. Sur la machine de la mesure, l'avertissement dit environ 10 agents.

### Les refus

- `set_board` refuse une valeur hors de la borne, sans rien changer, et dit pourquoi : « maxParallel va de 1 à 16 sur un poste (reçu 20). », « maxParallel vaut au plus 8, le plafond imposé par la stratégie de ce poste (reçu 9). »
- `start_ticket` est refusé quand les places de la borne sont prises (« Le maximum de tickets en parallèle du projet … est atteint (12, …) »).
- La fenêtre et l'API distante (`board_set`) ramènent une valeur changée dans la borne.

### Les quotas

Rien ne change au partage par compte : chaque ticket part sur un compte de son harness, et la pause au-delà du quota du pilote auto tient quel que soit le nombre de places ; plus de places ne font que dépenser le quota plus vite.

## Réglages

Dans la page « Kanban » du projet :

- l'action après validation : « Valider et merger », « Valider + PR », « Valider et pousser » ou « Valider » ; la façon de merger (merge commit, squash ou rebase), la PR en brouillon, ses relecteurs et ses étiquettes, la conduite en cas de conflit (voir *Validation*) ;
- le nombre de places, « En parallèle » : de 1 à 16 sur un poste, à 64 sur un serveur, sous un plafond imposé, 2 par défaut (voir *Les tickets en parallèle*) ;
- « Pause quand la CI de la cible est rouge », éteint par défaut (voir *Pilote auto*) ;
- la commande de tests, facultative ;
- le modèle, l'effort et le mode des agents de ticket ;
- la branche cible ;
- « Supprimer le worktree d'un ticket terminé ou supprimé », allumé par défaut.

Ailleurs :

- « Préfixe des branches des tickets » du projet (`ticket/`) ;
- les fichiers ignorés par git copiés dans le worktree, `.env*` par défaut ;
- « Pause au-delà du quota », réglage de l'app (voir *Pilote auto*).

## Le Kanban d'un projet d'équipe (TS-6)

Un projet coché comme celui d'une équipe peut partager son Kanban avec elle (voir *Le Kanban d'équipe*) : ses tickets sont ceux de l'équipe, en direct, avec des clés tirées d'un compteur du serveur ; un ticket se prend par le serveur avant que son agent démarre, et sa carte dit qui le tient et depuis quand. Un ticket « Ne pas partager » reste sur ce poste. Les tickets partagés par fichiers (*Tickets partagés*, ESC-190) restent pour les projets hors équipe.
