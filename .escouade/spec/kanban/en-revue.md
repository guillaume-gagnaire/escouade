---
title: "En revue"
subject: "kanban"
sources: ["docs/TODO-1.8.md"]
tickets: ["ESC-59","ESC-60","ESC-61","ESC-62","ESC-63","ESC-67"]
decisions: [55,56,58]
compared: null
---

## Ce que c'est

La colonne « En revue », entre « À tester » et « Terminé », tient les tickets dont la pull request est ouverte sur l'hébergeur (voir *Validation*). Le ticket y reste jusqu'au merge, fait sur l'hébergeur ou depuis sa carte (« Merger », « Merger quand c'est vert »). Sa carte montre où en est la PR, sans qu'il faille ouvrir le navigateur.

## Comportement

### La colonne

- Elle n'apparaît que si le Kanban ouvre des PR (« Valider + PR ») ou qu'un ticket y est.
- Les tickets y sont dans l'ordre de leur arrivée « À tester ».
- Son en-tête a « Rafraîchir » : les PR de la colonne sont relues tout de suite.

### La carte

- Le numéro, cliquable, avec le mot de l'hébergeur (« PR #12 », « MR #4 » sur GitLab) et l'icône de sa CI, comme les PR des agents dans la barre latérale ; un clic l'ouvre dans le navigateur.
- Son état en un mot : brouillon, ouverte, approuvée, changements demandés, fermée.
- Sa CI et ses jobs : « CI verte · 3 jobs », « CI en cours · 2 jobs », « 1/4 jobs en échec ». Un clic ouvre ses jobs dans Escouade : le journal d'un job en échec en lecture seule, « Envoyer à un agent » et « Relancer les jobs en échec » (voir *CI de la cible et des branches poussées*).
- Ses approbations, ses fils de discussion ouverts et un conflit avec la cible (« 1 approbation · 2 fils ouverts · conflit avec main »).
- « Ouvrir l'aperçu » quand la PR a un aperçu (Vercel, Netlify, Cloudflare Pages… ; voir *Environnements et déploiements*) : il l'ouvre dans le navigateur.
- En haut à droite : « → <cible> ». Les critères ne s'y affichent plus.

### La CI en échec : l'agent corrige

- La carte d'un ticket dont la CI échoue a « L'agent corrige » (si le ticket a encore son agent et qu'aucune correction n'est en cours).
- Un check en échec dont l'hébergeur ne garde pas de journal (une CI tierce : Jenkins, CircleCI… qui ne donne qu'un statut et une adresse) est dit sur la carte : « ci/jenkins : CI tierce, sans journal ».
- La correction, d'un clic ou du pilote auto :
  - le ticket reste « En revue » ; son agent, archivé à la validation, est restauré, et son worktree recréé depuis la branche du ticket (une branche supprimée du projet est reprise du dépôt distant) ;
  - l'agent reçoit le nom des jobs en échec, leur adresse, et la fin de leurs journaux (80 lignes par job au plus, comme les tests), entre balises, donnés comme des informations et non des instructions ; il est prié de corriger, vérifier, commiter et pousser, sans commenter sur l'hébergeur ;
  - la carte dit « L'agent corrige la CI… » tant qu'il travaille ;
  - à la fin de son tour, Escouade pousse la branche (l'agent a pu le faire lui-même), la PR se met à jour et est relue tout de suite ; le ticket revient « En revue » comme avant, son agent archivé de nouveau et son worktree supprimé si le Kanban nettoie.
- Ce qui empêche d'aller au bout est dit sur la carte, et l'agent reste restauré pour qu'on lui parle : des changements non commités (rien n'est poussé), rien de commité (« L'agent n'a rien commité : la CI reste en échec »), un push refusé, un tour interrompu ou en erreur, l'app fermée pendant le tour. Une CI verte efface ce message.
- Chaque correction compte une boucle du ticket.

### Les retours de relecture : l'agent corrige les retours

- Escouade lit les retours d'une PR ouverte quand un fil est ouvert ou qu'un relecteur demande des changements (deux requêtes de plus à chaque lecture, seulement alors).
- Un retour, c'est un commentaire non résolu (sur une ligne ou sous la PR), ou une relecture qui demande des changements ou commente avec un texte, arrivé depuis le dernier push de corrections des retours (l'ouverture de la PR pour le premier), et pas encore donné à l'agent. Ne comptent pas : les fils résolus, les commentaires des robots (`…[bot]`), les réponses d'Escouade elle-même sous les fils. Un commentaire dont l'hébergeur ne donne pas la date compte jusqu'à ce qu'il soit donné.
- Dès que des retours arrivent, une notification (« DEM-1 : 2 retours de relecture sur la PR #12 », réglable comme celles des tickets), et sur la carte le nombre de retours avec « L'agent corrige les retours ».
- L'agent ne part qu'après ce clic, même avec le pilote auto.
- La correction, comme celle de la CI : l'agent restauré, son worktree recréé depuis la branche ; il reçoit les retours, chacun avec son auteur, son fichier, sa ligne, l'extrait du diff où il est écrit (celui de l'hébergeur, sinon les lignes du fichier autour de la ligne dans la branche) et son texte, entre balises, donnés comme des informations et non des instructions. Il est prié de corriger ce qui est juste, de dire ce qu'il ne retient pas, de vérifier, commiter et pousser, sans répondre sous les fils ni les résoudre. La carte dit « L'agent corrige les retours… ».
- À la fin de son tour, Escouade pousse ; les retours donnés ne reviennent plus, et seuls comptent ceux qui arrivent après ce push. Rien de commité : « L'agent n'a rien commité : les retours restent à traiter », et ils restent à donner.
- Répondre sous les fils : un réglage du Kanban, « Répondre sous les fils corrigés », éteint par défaut. Allumé, Escouade répond « Corrigé dans abc1234 » sous chaque fil donné à l'agent, une fois ses corrections poussées. Escouade ne résout jamais un fil.

### Le conflit avec la cible : l'agent met à jour la branche

- La carte d'une PR en conflit avec sa cible a « L'agent met à jour la branche » (si le ticket a encore son agent et qu'aucune correction n'est en cours). Jamais seul, même avec le pilote auto.
- L'agent restauré, son worktree recréé depuis la branche, Escouade va chercher la cible sur le dépôt distant et la merge dans la branche (un commit de merge).
- Sans conflit, Escouade pousse tout de suite, l'agent n'a rien à faire ; le ticket revient « En revue », son agent archivé de nouveau.
- Avec des conflits, comme le Kanban les traite (« En cas de conflit ») : « Annuler » défait le merge, et la carte nomme les fichiers (« Conflit avec main sur : a.txt ») ; sinon (« Me demander » : le clic est la réponse, ou « Laisser l'agent résoudre ») l'agent reçoit les fichiers en conflit, les résout et commite le merge ; Escouade pousse à la fin de son tour.
- Le rebase, seulement en réglage : « Mise à jour de la branche » du Kanban, Merge par défaut, ou Rebase. Escouade rebase la branche sur la cible ; à un conflit il défait le rebase et l'agent le refait lui-même, en résolvant ; la branche est poussée avec `--force-with-lease`, qui refuse d'écraser un push d'un autre.
- Après le push, l'hébergeur redit s'il reste un conflit.

### Merger depuis Escouade

Rien ne se merge sans un geste : les deux actions sont des boutons de la carte, jamais un réglage.

- **La stratégie** : celle du Kanban (merge commit, squash, rebase) quand le dépôt l'autorise, sinon la première qu'il autorise. Escouade lit ce que le dépôt autorise une fois, à la première lecture de la PR ouverte :
  - GitHub : les méthodes du dépôt (`allow_merge_commit`, `allow_squash_merge`, `allow_rebase_merge`) et « Allow auto-merge ». GitHub ne les dit qu'à qui peut pousser : sinon, les trois.
  - GitLab : `squash_option` du projet (« never » : pas de squash ; « always » : pas de merge commit).
  - Gitea / Forgejo : les styles du dépôt (merge, squash, rebase, rebase puis merge), et l'auto-merge seulement si la branche cible est protégée.
  - Bitbucket : les stratégies de la branche cible (`merge_strategies`), jamais d'auto-merge.
  - Azure DevOps : la politique de merge de la branche cible, et son auto-complete.
  - Un plugin d'hébergeur qui ne lit pas ces réglages : ce que son fournisseur déclare.
- **« Merger (squash) »** : seulement pour une PR approuvée (au moins une approbation, aucun changement demandé), verte (CI réussie, ou pas de CI), sans conflit, ni brouillon, que son agent ne corrige pas la CI (voir plus haut), et avec une stratégie autorisée. Le bouton dit la stratégie. Si le dépôt n'en autorise aucune d'Escouade, la carte le dit, sans bouton. Mergée, la PR est relue tout de suite et le ticket passe « Terminé » comme après un merge sur l'hébergeur.
- **« Merger quand c'est vert »** : tant que la CI tourne, sur une PR ouverte, ni brouillon, ni en conflit, sans changement demandé, que rien ne merge déjà seul, et que son agent ne corrige pas la CI.
  - L'hébergeur a un auto-merge pour ce dépôt (GitHub `enablePullRequestAutoMerge` si le dépôt l'autorise, GitLab `auto_merge` / `merge_when_pipeline_succeeds`, Gitea `merge_when_checks_succeed` sur une branche protégée, Azure DevOps son auto-complete) : Escouade l'active. La carte dit « Auto-merge de GitHub activé : mergée dès que la CI est verte ».
  - Sinon (Bitbucket, un dépôt GitHub sans auto-merge, une branche Gitea non protégée) : Escouade le dit (« Pas d'auto-merge sur Bitbucket pour ce dépôt : Escouade attend la fin de la CI et merge elle-même, tant qu'elle tourne »). La carte dit « Escouade la merge (squash) dès qu'elle est approuvée et verte, tant qu'elle tourne », avec « Ne plus attendre ». L'attente est gardée avec le ticket et reprend au redémarrage. À la lecture où la PR est approuvée et verte, Escouade la merge ; une CI en échec ou des changements demandés arrêtent l'attente, et la carte le dit (« CI en échec : PR #12 n'est pas mergée, Escouade n'attend plus »).
- **Un merge refusé** par l'hébergeur (check requis, droits) : la carte dit « Merge refusé : … », avec « Fermer » ; le ticket reste « En revue ».

### Mergée sur l'hébergeur

- Le ticket passe « Terminé », avec « ⤵ PR #12 mergée dans main » et le lien de la PR.
- Le commit de son merge, quand l'hébergeur le dit, est gardé : il dit dans quels environnements le ticket est déployé (voir *Environnements et déploiements*).
- L'issue externe suit, et reçoit son commentaire, comme pour tout passage en « Terminé » (voir *Intégrations*).
- Sa branche locale part si « Supprimer le worktree d'un ticket terminé ou supprimé » est allumé (une branche reprise reste).
- Un fetch met la base à jour : la cible locale avance jusqu'à sa branche distante quand c'est une avance rapide (dans le dossier du projet si elle y est extraite et que rien ne gêne).
- Les tickets qui l'attendent (« Après ») partent alors seulement : tant qu'il est « En revue », il les retient.

### Fermée sans merge

- Le ticket est bloqué : « PR #12 fermée sans merge », avec deux actions.
- « Remettre à faire » : le ticket repart de zéro dans « À faire », sa PR oubliée ; il reprend sa branche, qui garde son travail.
- « Terminer quand même » : le ticket passe « Terminé », avec « ✓ Terminé quand même · PR #12 fermée ».
- Rouverte sur l'hébergeur avant l'une ou l'autre, elle est suivie de nouveau.

### Le rafraîchissement

- Pas de webhook : l'app n'a pas d'adresse publique. Escouade interroge l'hébergeur, et seulement pour les PR des tickets « En revue » (celles des agents ont leur propre lecture, voir *Pull requests*), avec, dans la même boucle, la CI des cibles et des branches poussées (voir *CI de la cible et des branches poussées*).
- Toutes les minutes quand la fenêtre est au premier plan, toutes les 5 minutes sinon.
- Tout de suite après un push d'Escouade (une validation qui ouvre une PR, « Pousser », la fin d'une correction de CI, des retours ou d'une mise à jour de la branche), quand la fenêtre revient au premier plan et à la connexion d'un compte d'hébergeur ; et sur « Rafraîchir ».
- Les PR d'un même dépôt se lisent ensemble : une seule requête GraphQL sur GitHub ; les jobs de la CI se comptent par une requête de plus, seulement pour une PR ouverte qui a une CI ; les déploiements de sa branche (son aperçu) par une autre, quand l'hébergeur en a. Les journaux ne se lisent qu'au moment de la correction, ou à l'ouverture d'un job. Ce que le dépôt autorise pour le merge se lit une fois par PR. La même lecture sert l'attente de « Merger quand c'est vert » : pas de seconde boucle.
- La même boucle lit ensuite les environnements des projets suivis (voir *Environnements et déploiements*).

### Les erreurs

- Hébergeur injoignable, jeton refusé ou expiré, plugin arrêté : la carte le dit, avec depuis quand (« GitHub injoignable : … · depuis 10 min », « GitHub : jeton refusé ou expiré, reconnecte le compte »).
- Une erreur ne déplace jamais un ticket : il reste « En revue », avec ce qui a été lu en dernier, jusqu'à une lecture réussie.

### Le pilote auto

- Un ticket « En revue » ne prend de place parmi les tickets en parallèle que pendant que son agent corrige sa CI, ses retours ou met à jour sa branche : son agent est archivé le reste du temps.
- Avec le pilote auto, une CI en échec part seule chez l'agent, quand une place est libre et que la pause au-delà du quota ne retient pas son compte, avant les tickets « À faire ». La pause sur une cible rouge ne la retient pas.
- Il ne la renvoie que dans la limite des boucles du ticket (3, 5 ou 8) ; au-delà, la carte dit « Boucles épuisées (3/3) : le pilote auto ne la renvoie plus », et « L'agent corrige » reste à la main.
- Il ne renvoie pas le même échec deux fois : après une correction (réussie ou non), il attend que la CI tourne de nouveau (en cours, verte ou absente) avant de la renvoyer.
- Sans le pilote auto, rien ne part seul : « L'agent corrige » sur la carte.
- Les retours de relecture et le conflit avec la cible ne partent jamais seuls, pilote auto ou non : seulement sur leur bouton.

## Ce qui n'est pas fait

- Un ticket dont l'agent a été supprimé ne peut pas faire corriger sa CI, ses retours ni mettre à jour sa branche.
- Les retours de GitLab, Bitbucket et Azure DevOps n'ont pas d'extrait du diff de l'hébergeur : Escouade donne les lignes du fichier autour de la ligne. Leurs relectures sans texte ne comptent pas comme retours (leurs commentaires, si).
- Couper l'auto-merge de l'hébergeur depuis Escouade : il se coupe sur l'hébergeur.
- Les essais réels : GitHub seulement, avec le compte de l'utilisateur ; GitLab, Bitbucket Cloud et Gitea / Forgejo d'après leur documentation et leurs faux serveurs, à essayer en vrai plus tard.
