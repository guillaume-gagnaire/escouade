---
title: "En revue"
subject: "kanban"
sources: ["docs/TODO-1.8.md"]
tickets: ["ESC-59"]
decisions: []
compared: null
---

## Ce que c'est

La colonne « En revue », entre « À tester » et « Terminé », tient les tickets dont la pull request est ouverte sur l'hébergeur (voir *Validation*). Le ticket y reste jusqu'au merge, que l'utilisateur fait sur l'hébergeur. Sa carte montre où en est la PR, sans qu'il faille ouvrir le navigateur.

## Comportement

### La colonne

- Elle n'apparaît que si le Kanban ouvre des PR (« Valider + PR ») ou qu'un ticket y est.
- Les tickets y sont dans l'ordre de leur arrivée « À tester ».
- Son en-tête a « Rafraîchir » : les PR de la colonne sont relues tout de suite.

### La carte

- Le numéro, cliquable, avec le mot de l'hébergeur (« PR #12 », « MR #4 » sur GitLab) et l'icône de sa CI, comme les PR des agents dans la barre latérale ; un clic l'ouvre dans le navigateur.
- Son état en un mot : brouillon, ouverte, approuvée, changements demandés, fermée.
- Sa CI et ses jobs : « CI verte · 3 jobs », « CI en cours · 2 jobs », « 1/4 jobs en échec ».
- Ses approbations, ses fils de discussion ouverts et un conflit avec la cible (« 1 approbation · 2 fils ouverts · conflit avec main »).
- En haut à droite : « → <cible> ». Les critères ne s'y affichent plus.

### Mergée sur l'hébergeur

- Le ticket passe « Terminé », avec « ⤵ PR #12 mergée dans main » et le lien de la PR.
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

- Pas de webhook : l'app n'a pas d'adresse publique. Escouade interroge l'hébergeur, et seulement pour les PR des tickets « En revue » (celles des agents ont leur propre lecture, voir *Pull requests*).
- Toutes les minutes quand la fenêtre est au premier plan, toutes les 5 minutes sinon.
- Tout de suite après un push d'Escouade (une validation qui ouvre une PR, « Pousser ») et quand la fenêtre revient au premier plan ; et sur « Rafraîchir ».
- Les PR d'un même dépôt se lisent ensemble : une seule requête GraphQL sur GitHub ; les jobs de la CI se comptent par une requête de plus, seulement pour une PR ouverte qui a une CI.

### Les erreurs

- Hébergeur injoignable, jeton refusé ou expiré, plugin arrêté : la carte le dit, avec depuis quand (« GitHub injoignable : … · depuis 10 min », « GitHub : jeton refusé ou expiré, reconnecte le compte »).
- Une erreur ne déplace jamais un ticket : il reste « En revue », avec ce qui a été lu en dernier, jusqu'à une lecture réussie.

### Le pilote auto

Un ticket « En revue » ne prend pas de place parmi les tickets en parallèle : son agent est archivé à la validation. Quand l'agent devra corriger (CI, retours), son worktree sera recréé depuis la branche, et il ne comptera que pendant qu'il travaille.

## Ce qui n'est pas fait

- « L'agent corrige » (CI en échec), « L'agent traite les retours », « L'agent met à jour la branche » (conflit), « Merger » et « Merger quand c'est vert » depuis la carte.
- Les essais réels : GitHub seulement, avec le compte de l'utilisateur ; GitLab, Bitbucket Cloud et Gitea / Forgejo d'après leur documentation et leurs faux serveurs, à essayer en vrai plus tard.
