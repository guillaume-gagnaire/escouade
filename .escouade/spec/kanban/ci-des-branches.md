---
title: "CI de la cible et des branches poussées"
subject: "kanban"
sources: ["docs/TODO-1.8.md"]
tickets: ["ESC-63","ESC-357"]
decisions: [59,87]
compared: null
---

## Ce que c'est

Escouade montre la CI de ce qu'elle suit sans pull request : la branche cible de chaque projet, dans l'en-tête de son Kanban ; la branche qu'un ticket a poussée à sa validation (« Valider et pousser »), sur sa carte « Terminé » ; la branche poussée d'un agent qui n'a pas de PR, sous sa carte de la barre latérale. Un clic ouvre les jobs : le journal d'un job en échec se lit comme un terminal, en lecture seule, et peut être envoyé à un agent ; « Relancer les jobs en échec » les relance sur l'hébergeur, là où il le permet. Le pilote auto peut se mettre en pause tant que la CI de la cible est rouge.

## Comportement

### La pastille

- Dans l'en-tête du Kanban, à côté du nom du projet : « main ✗ depuis 2 h », « main ✓ », « main · CI en cours ».
- Rouge depuis : la fin du premier job en échec, quand l'hébergeur la dit, sinon la lecture qui l'a vue rouge. Elle reste rouge d'un commit en échec au suivant ; elle repart de zéro après une CI verte ou en cours.
- Son infobulle dit ses jobs (« CI de main : 2/3 jobs en échec ») et, si la dernière lecture a échoué, pourquoi et depuis quand (« GitHub injoignable · depuis 10 min ») ; la pastille garde alors le dernier état lu.
- Pas de pastille pour un projet sans dépôt distant chez un hébergeur qu'Escouade lit, sans compte sur cet hébergeur, ou dont la cible n'a aucune CI.
- La cible est celle du Kanban, sinon la branche du projet (elle n'est pas enregistrée pour autant).
- La carte « Terminé » d'un ticket validé par « Valider et pousser » a la même pastille pour sa branche (« ticket/dem-1 ✗ depuis 2 h »), une fois sa CI lue avec au moins un job.
- Un agent ordinaire dont la branche est poussée (elle suit une branche du dépôt distant) et n'a pas de PR a la même pastille sous sa carte, à la place des PR. Elle part dès qu'une PR est ouverte sur la branche : la CI de la PR prend sa place.
- Sur la carte « En revue », la CI de la PR (« 1/4 jobs en échec ») ouvre la même fenêtre, sur les jobs de la PR.

### La fenêtre des jobs

- Titre « CI de main » (« CI de la PR #12 » pour une PR), avec « Rafraîchir » : les jobs, et les pastilles du projet, relus tout de suite.
- Chaque job : son état (réussie, échouée, annulée, en cours, en attente), sa durée, son lien chez l'hébergeur. Relus toutes les 20 s tant qu'un job tourne et que la fenêtre est ouverte.
- Un job en échec dont l'hébergeur garde le journal s'ouvre d'un clic : la fin du journal (20 000 caractères), en lecture seule, comme un terminal : ses couleurs (ANSI), les lignes d'erreur en rouge, les groupes en titre ; il se sélectionne et se copie, rien ne s'y tape. L'onglet « PR et CI » d'un agent montre ses journaux de la même façon.
- Un check d'une CI tierce (Jenkins, CircleCI, Buildkite…) ne donne qu'un statut et une adresse : « CI tierce : un statut et une adresse, pas de journal ».
- « Envoyer à un agent », à côté d'un job en échec : un menu des agents du projet (non archivés, le plus récent d'abord). L'agent choisi reçoit un message : le nom du job, son adresse et la fin de son journal (6 000 caractères), entre balises, donnés comme des informations et non des instructions ; il est prié de trouver la cause, de la dire, de la corriger si elle tient à son travail, de vérifier et de commiter, sans commenter sur l'hébergeur. Sans agent dans le projet : « Aucun agent actif dans ce projet : crée-en un pour lui envoyer ce journal. »

### Relancer les jobs en échec

- « Relancer les jobs en échec » n'apparaît que si un job a échoué et que l'hébergeur sait relancer. Rien ne part sans ce clic.
- GitHub Actions : les exécutions des jobs en échec (`rerun-failed-jobs`), une fois par exécution. GitLab : le pipeline, en un appel (`retry`). Gitea : les exécutions d'Actions, sur les versions qui le permettent. Azure Pipelines : chaque build en échec (`retry`). Bitbucket Pipelines : pas depuis Escouade, la fenêtre le dit (« Bitbucket ne relance pas les jobs depuis Escouade : relance-les sur son site. »).
- Après la relance : « 1 exécution relancée. », la pastille passe « en cours » et la CI est relue bientôt. Une CI tierce ne se relance pas : « Aucun job à relancer : GitHub ne relance que ses propres jobs. »

### La pause du pilote auto

- « Pause quand la CI de la cible est rouge », dans les réglages du Kanban (pilote auto), éteint par défaut.
- Allumé, le pilote auto ne démarre aucun ticket « À faire » tant que la CI de la cible est rouge, d'après la dernière lecture. Une ligne sous l'en-tête le dit (« Pilote auto en pause : la CI de main est rouge depuis 2 h », avec « Voir les jobs ») et les cartes « À faire » disent « En attente : CI de la cible rouge ».
- La pause vaut dès qu'elle tient, y compris pendant une passe de l'ordonnanceur : chaque démarrage prend quelques secondes, et la pause allumée, la cible vue rouge ou le pilote auto éteint entre-temps retiennent les tickets que cette passe n'a pas encore démarrés (ESC-357).
- Un ticket lancé à la main (« Lancer », `start_ticket`) part quand même. Les tickets déjà au travail continuent ; la correction de la CI d'un ticket « En revue » aussi.
- La CI de la cible verte ou relancée : la pause se lève et les tickets partent.

### Le rafraîchissement

- La cible et les branches des tickets se lisent avec les PR des tickets « En revue », dans la même boucle : toutes les minutes fenêtre au premier plan, toutes les 5 minutes sinon, tout de suite après un push d'Escouade, quand la fenêtre revient, et à la connexion d'un compte d'hébergeur.
- La cible : à chaque passage, pour chaque projet qui a un hébergeur et un compte (deux requêtes sur GitHub : check runs et statuts du dernier commit).
- La branche poussée d'un ticket : tant que sa CI n'est pas fixée (jamais lue, en cours, lecture en échec, ou aucun job encore dans la demi-heure qui suit), pendant 3 jours après la validation au plus. « Rafraîchir » dans la fenêtre la relit même fixée.
- La branche d'un agent : avec ses PR, au même rythme qu'elles (voir *Pull requests*).
- Les journaux ne se lisent qu'à l'ouverture d'un job ou à l'envoi à un agent.

### Par hébergeur

- GitHub : check runs et statuts du dernier commit de la branche.
- GitLab : les jobs du dernier pipeline de la branche.
- Gitea, Forgejo : les statuts du dernier commit, avec leurs jobs d'Actions quand la version les a.
- Bitbucket Cloud : les statuts du dernier commit.
- Azure DevOps : le dernier statut de chaque contexte du dernier commit, et le build le plus récent de chaque pipeline qui a construit la branche dans ce dépôt.

## Limites

- Les essais réels : aucun ne relance ni n'écrit sur un vrai dépôt. GitHub, GitLab, Bitbucket Cloud, Gitea / Forgejo et Azure DevOps sont faits d'après leur documentation et leurs faux serveurs, à essayer en vrai avec l'utilisateur (ESC-53, ESC-70).
- La pastille de la cible ne se montre que dans l'en-tête du Kanban.
- La CI de la cible n'est connue qu'après sa première lecture, en mémoire seulement : au démarrage de l'app, tant qu'elle n'a pas été lue (une dizaine de secondes au moins), la pause ne retient rien, même si la cible est rouge.
