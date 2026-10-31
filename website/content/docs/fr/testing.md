# Tester et valider un ticket

Quand un agent a fini, son ticket attend dans « À tester ». « ▶ Tester » lance sa fonctionnalité sur des ports à part, après t’avoir montré ce qui va tourner. « Valider » livre le travail, « Renvoyer » le rend à l’agent avec tes remarques.

![Le test d’un ticket : la fenêtre « Tester DEM-6 » montre la préparation terminée et les deux processus prêts](/images/test.jpg)

## Quand un ticket arrive dans « À tester »

Un ticket passe dans « À tester » quand tous ses critères sont atteints, ou quand l’agent a épuisé ses boucles (la carte porte alors « Objectif partiel »). Tu es prévenu (« prêt à tester »), et le badge de « Kanban » compte un ticket de plus.

La carte montre « Ce qui a été fait », les critères avec la note de l’agent au survol, le coût du ticket et « 3/3 critères ». Dessous : « ▶ Tester » quand l’agent a de quoi lancer son travail, le bouton de validation, et « Renvoyer ».

Pour regarder le travail avant de décider, ouvre l’agent (un clic sur la carte) : sa conversation, ses fichiers modifiés et leurs diffs se lisent comme ceux de n’importe quel agent (voir [Git](git) et [L’éditeur](editor)).

« Valider » et « Renvoyer » sont refusés tant que l’agent travaille encore : attends la fin de son tour.

## Tester avec « ▶ Tester »

### Où est le bouton

Le bouton « ▶ Tester » est sur la carte « À tester », dans l’en-tête de la conversation de l’agent et dans la partie « Lancement de test » du bilan des critères. Il apparaît quand l’agent a donné sa recette de lancement (voir plus bas), ou quand [isola](#avec-isola) lance les services du worktree. Il n’y est pas pour un agent archivé, un agent sans worktree, ni pendant que le ticket est en cours de validation : l’app refuse alors tout lancement de test (« Validation en cours : le lancement de test attendra sa fin. »).

Le protocole de l’agent lui demande de donner sa recette au plus tard quand tous les critères sont atteints. Si elle manque, l’en-tête de sa conversation propose « Préparer le lancement » : l’agent est invité à dire comment lancer son worktree, sur ses ports.

### La recette écrite par l’agent

Une recette dit comment lancer le travail de l’agent dans son worktree :

- des étapes de « Préparation » : des commandes lancées l’une après l’autre, chacune dans un dossier du worktree (`npm install`) ;
- des processus : un nom, une commande, un dossier, des variables d’environnement et l’adresse qui répond quand le processus est prêt (un serveur de développement) ;
- l’adresse à ouvrir dans le navigateur, celle qui montre directement la fonctionnalité (la page, l’écran, l’état à tester).

C’est l’agent qui l’écrit, et elle tourne dans ton shell, hors du mode de permission de Claude Code. Escouade te la montre donc avant de la lancer. À la première fois, et chaque fois que la recette change (une commande, une variable, un dossier ou l’adresse), la fenêtre « Tester DEM-6 » affiche :

- « Ces commandes ont été écrites par dem-6-limiter-les-tentatives. Elles tournent dans ton shell, hors du mode de permission de Claude Code. » ;
- « Préparation » : chaque commande et son « Dossier » (« la racine du worktree » quand il est vide) ;
- « Lancement » : chaque processus, sa commande, son « Dossier », ses « Variable » et son « Adresse » ;
- « Ouverture » : l’adresse qui s’ouvrira dans le navigateur.

Rien ne tourne avant que tu cliques sur « Lancer » ; « Annuler » ferme la fenêtre. Les caractères qui cacheraient une partie d’une commande (retour chariot, espace de largeur nulle, marque de direction…) sont écrits en toutes lettres (`⟨U+200B⟩`), et les longues suites d’espaces ou de lignes vides sont comptées (« ⟨24 espaces⟩ »).

Si l’agent envoie une autre recette pendant que tu lis, la fenêtre le dit (« La recette vient de changer : relis-la avant de lancer. ») et « Lancer » reste fermé une seconde, pour qu’un clic visé sur l’ancienne ne valide pas la nouvelle. L’approbation est gardée avec l’agent : une recette que tu as approuvée et qui n’a pas changé se lance directement la fois suivante. Et Escouade refuse de lancer une étape d’une recette que tu n’as pas approuvée.

### Les ports réservés

Chaque agent de ticket reçoit un bloc de 10 ports à lui, à partir de 4100 : 4100 à 4109, puis 4110 à 4119… Le bloc est le premier qu’aucun agent n’occupe et dont tous les ports sont libres sur ta machine. Ainsi, les serveurs de plusieurs tickets tournent côte à côte sans se gêner, ni gêner ton propre serveur de développement.

L’agent connaît son bloc. Il doit s’en servir sans l’écrire dans des fichiers versionnés (en arguments ou en variables d’environnement). Les commandes d’un test, celles des tests de validation et celles de préparation du worktree reçoivent `ESCOUADE_PORT_BASE` (le premier port) et `ESCOUADE_PORT_END` (le dernier).

### Pendant le test

La fenêtre liste les étapes, chacune avec son état (✓ prêt, … en cours, ✕ en échec, ■ arrêté, – non attendu) :

1. Les étapes de préparation tournent l’une après l’autre. Elles ne tournent qu’une fois par recette : tant qu’elle ne change pas et qu’Escouade reste ouverte, un nouveau test les saute. Une étape en échec arrête le test (« Préparation en échec (code 1) »).
2. Chaque processus démarre dans son propre terminal, en lecture seule. Ils apparaissent dans la section « Lancement » de la barre latérale, avec leurs logs (voir [Les commandes de lancement](launch)).
3. Pour chaque processus qui a une adresse, Escouade attend qu’elle réponde (n’importe quelle réponse HTTP compte, même une 404), pendant 3 minutes au plus : « en attente de localhost:4101… », puis « prêt · 2,4 s » ou « Pas de réponse de http://localhost:4101 après 3 min ».
4. La page s’ouvre sur l’adresse à ouvrir, à défaut sur celle du premier processus : dans la vue « Page » de l’agent par défaut, dans la fenêtre « Pages » du projet ou dans le navigateur du système, selon Réglages › Projets › Lancement (voir [La page du lancement](browser)).

Quand le test est prêt, la fenêtre propose « Rouvrir » (la page), « Voir les logs », « Tout arrêter » et « Fermer ». Une ligne en échec a son « Voir le log ». Un processus qui s’arrête pendant que le test est prêt met fin au test (« web s’est arrêté »).

Tant que les serveurs tournent, la carte du ticket affiche « ■ Arrêter », qui les arrête, et « Ouvrir », qui rouvre l’adresse. Les serveurs s’arrêtent aussi d’eux-mêmes quand le ticket est validé, quand son agent est archivé ou supprimé et quand le projet est fermé.

### Avec isola

Quand [isola](https://github.com/cyucelen/isola) est installé et que le worktree a un `.isola.toml`, isola lance les services du worktree sur ses propres ports et adresses. L’agent ne reçoit alors pas de bloc de ports, et il n’a pas de recette à écrire : il donne seulement l’adresse qui montre la fonctionnalité.

« ▶ Tester » montre alors le `.isola.toml` sous « Configuration isola (.isola.toml) » (isola lance les commandes de ce fichier dans ton shell, hors du mode de permission de Claude Code, et l’agent peut l’avoir écrit ou modifié) et l’adresse qui s’ouvrira. Comme pour une recette, rien ne tourne avant « Lancer » et un fichier modifié est montré de nouveau. Le test lance `isola up`, attend que chaque service réponde, puis ouvre l’adresse. « Tout arrêter » lance `isola down`.

## Valider un ticket

Le bouton de la carte porte le nom de ce que la validation fait. Cela se règle dans le premier groupe de Réglages › Kanban, « Quand je valide un ticket « À tester » » ; le bouton « Après validation : … » de l’en-tête y mène (voir [Réglages du Kanban](settings-board)).

| Choix du réglage | Bouton | Ce qui se passe | La carte « Terminé » dit |
|---|---|---|---|
| « Merger dans une branche » | « Valider et merger » | Ce qui reste dans le worktree est commité, puis mergé dans la branche cible avec la stratégie choisie : « Merge commit », « Squash » ou « Rebase ». | « ⤵ Mergé dans main · squash » |
| « Ouvrir une pull request » | « Valider + PR » | Commit, push de la branche du ticket (`ticket/<clé>`), puis une pull request vers la branche cible, en brouillon si « PR en brouillon » est activé. | « ⇡ PR #41 → main » |
| « Pousser la branche du ticket » | « Valider et pousser » | Commit et push de la branche du ticket, sans merge ni PR. | « ⇡ Poussé sur ticket/dem-3 » |
| « Laisser en l’état » | « Valider » | Rien n’est commité : les modifications restent dans le worktree, et l’agent aussi. | « ◇ Laissé dans le worktree » |

Un ticket « À tester » se valide même s’il est en « Objectif partiel ».

Une fois le ticket validé (sauf avec « Laisser en l’état »), son agent est archivé. Si « Supprimer le worktree une fois validé » est activé (par défaut), le worktree est supprimé après les commandes de démontage du projet. La branche du ticket part avec lui après un merge, ou quand le ticket n’apportait aucun changement ; elle reste quand elle a été poussée ou proposée en PR. Une branche existante que le ticket avait reprise n’est jamais supprimée. Le ticket passe dans « Terminé », avec son issue, son coût et son nombre de boucles. Un ticket importé met son statut à jour et peut laisser un commentaire (voir [Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards](integrations)).

Si la branche du ticket n’apporte aucun changement, le ticket est terminé quand même : « ∅ Aucune modification ».

### Les étapes d’une validation

La carte affiche l’étape en cours : « Validation… », puis, selon les réglages, « Tests… », « Commit… » et « Merge… » ou « Push… ».

1. **Tests.** Avec « Relancer les tests avant » activé et une « Commande de tests » renseignée, la commande tourne dans le worktree du ticket, avec les variables de ports, pendant 20 minutes au plus. Les serveurs d’un test éventuel sont arrêtés avant. Si un test échoue, la validation s’arrête : le ticket retourne « En cours » chez son agent, qui reçoit la fin de la sortie des tests (« Les tests (`npm test`) échouent : … ») et recommence à la boucle 1. Il revient dans « À tester » quand il a fini.
2. **Commit.** Tout ce qui reste dans le worktree est commité, sauf les fichiers copiés du projet (`.env`…), qui ne le sont jamais : si la branche en porte un, la validation s’arrête et le dit. Le message est écrit par Haiku au format Conventional Commits, avec la clé en fin de ligne (`feat: limiter les tentatives de connexion [DEM-6]`), dans la « Langue des textes rédigés par les agents ». S’il ne donne rien d’utilisable, le message est `feat: <titre> [<clé>]`. Avec « Message de commit généré » éteint, c’est `DEM-6 Limiter les tentatives de connexion`.
3. **Merge ou push.** Le merge se fait dans le dossier du projet quand la branche cible y est extraite (il est refusé si des fichiers suivis y ont des modifications non commitées), dans un worktree temporaire sinon, et il est refusé quand la branche cible est extraite dans le worktree d’un agent. Deux validations dans le même dépôt se font l’une après l’autre. Pour une pull request, Escouade utilise la CLI `gh` de GitHub : le titre est le message de commit, la description reprend celle du ticket, « Ce qui a été fait » et les critères en liste à cocher. Sans `gh`, ou s’il échoue, la page GitHub de comparaison s’ouvre dans le navigateur, déjà remplie, et la carte dit « ⇡ ticket/dem-6 poussée · PR à finaliser ». Avec un autre hébergeur que GitHub, la branche est poussée et la carte dit « ⇡ ticket/dem-6 poussée · PR à ouvrir sur ton hébergeur ».

Quand le merge supprime le worktree et que des fichiers de l’agent sont ouverts dans l’éditeur avec des modifications non enregistrées, Escouade demande d’abord « Valider DEM-6 ? », en disant ce qui serait perdu.

### Les conflits

Quand le merge rencontre un conflit, il est annulé : rien n’est à moitié mergé. Le réglage « En cas de conflit » décide de la suite.

| Réglage | Ce qui se passe |
|---|---|
| « Me demander » (par défaut) | La carte se bloque sur « Conflit avec main » et propose « L’agent résout » et « Annuler ». |
| « L’agent résout » | L’agent prend le conflit en charge tout de suite. |
| « Annuler » | La carte se bloque en nommant les fichiers (« Conflit avec main sur : a.ts, b.ts »), et propose « Réessayer ». |

« L’agent résout » fusionne la branche cible dans la branche du ticket (ou, avec la stratégie « Rebase », demande à l’agent de rebaser), laisse les conflits dans le worktree et renvoie le ticket « En cours », à la boucle 1, avec l’agent prévenu. « Annuler » sur la carte enlève le blocage : le ticket reste « À tester ».

### Quand une étape échoue

Une étape en échec (une branche cible introuvable, un conflit non résolu dans le worktree, un push refusé…) laisse le ticket « À tester », bloqué avec la première ligne de l’erreur, et tu es prévenu (« bloqué »). Le bouton de validation devient « Réessayer » : il relance la validation.

## Renvoyer un ticket à son agent

« Renvoyer » ouvre un champ « Ce qui ne va pas ». Décris le problème (le bouton « Renvoyer » reste grisé tant que le champ est vide ; `Échap` ou « Annuler » abandonnent).

Le ticket retourne « En cours », avec le même agent et le même worktree, à la boucle 1. L’agent reçoit « Retour de test sur DEM-6 : `<ton texte>`. Corrige, revérifie tous les critères, puis termine par le bilan. » Il boucle de nouveau jusqu’à atteindre les critères ou épuiser ses boucles, puis le ticket revient dans « À tester ». Toutes les boucles comptent dans le total affiché sur la carte « Terminé ».

Pour la façon dont l’agent boucle sur un ticket, voir [Le pilote auto](autopilot#la-boucle).
