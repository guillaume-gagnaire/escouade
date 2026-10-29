---
title: "Validation"
subject: "Kanban"
sources: ["docs/SPEC.md"]
tickets: ["ESC-143","ESC-259","ESC-263"]
decisions: []
compared: null
---

## Ce que c'est

Valider un ticket « À tester » termine son travail. Escouade lance les tests, commite ce qui reste, puis merge, ouvre une pull request, pousse ou ne fait rien, selon l'action choisie. Le worktree et l'agent sont ensuite rangés, sans jamais perdre de travail. « Renvoyer » rend plutôt le ticket à son agent.

## Comportement

### Les actions

« Valider et merger », « Valider + PR », « Valider et pousser », « Valider ».

### Les étapes

1. La validation est refusée tant que l'agent travaille : ses fichiers seraient commités en cours d'écriture.
2. Les lancements de test de l'agent s'arrêtent ; aucun ne part tant que le ticket est en validation.
3. Les tests, facultatifs : la commande du Kanban, 20 min au plus, avec `ESCOUADE_PORT_BASE` défini. En échec, le ticket repart chez son agent avec les 80 dernières lignes.
4. Les décisions que son agent a proposées sont acceptées (voir *Décisions*) ; une qui ne peut s'écrire n'arrête rien.
5. Les modifications restantes sont commitées. Le message suit Conventional Commits et porte la clé ; il est écrit par le modèle des textes rédigés (voir *Langues*), `feat: <titre> [<clé>]` à défaut. Les fichiers copiés en sont exclus.
6. Sans aucun commit de plus que la cible (aucune modification, ou des modifications défaites), le ticket passe « Terminé » avec « ∅ Aucune modification », sans merge, PR ni push.
7. Refus si un fichier copié du projet est commité dans la branche : « … est commité dans la branche », ou « … est dans l'historique de la branche » s'il en a été retiré depuis.
8. Puis, selon l'action : le merge (merge commit, squash ou rebase ; dans le dossier du projet si la cible y est extraite, sinon dans un worktree temporaire), la PR, le push, ou rien.

En cas de conflit, selon le réglage : me demander (« L'agent résout » / « Annuler »), l'agent résout, ou annuler.

### La pull request d'un ticket

- La branche du ticket est poussée. Quand le dépôt distant est sur GitHub et que `gh` est installé, `gh pr create` ouvre la PR, avec pour corps la description, l'avancement et les critères en cases à cocher ; en brouillon selon le réglage.
- Sans `gh`, ou s'il échoue, la page de comparaison de GitHub s'ouvre dans le navigateur, titre et corps remplis, à finaliser.
- Sur Azure Repos, avec un compte Azure DevOps, la PR est ouverte par l'API d'Azure ; celle d'un ticket venu d'Azure Boards est liée à son élément de travail (voir *Intégrations*).
- Chez un autre hébergeur : le push seul.
- Ensuite, la PR se suit comme celle de tout agent (voir *Pull requests*).

### Après la validation

- Avec une branche fixe des docs, la spec et les décisions que son agent a écrites y sont versées maintenant, jamais avant. Après un merge qui a modifié quelque chose, les pages qu'il touche sont comparées au code en arrière-plan (voir *Spec tenue à jour*).
- Une fois le ticket validé (merge, PR, push ou aucune modification), son worktree est supprimé après les commandes de démontage du projet, selon le réglage (ci-dessous).
- Sa branche part avec lui après un merge, ou quand il n'y avait aucune modification. Poussée ou proposée en PR, elle reste. Une branche reprise, qui existait avant le ticket, est celle de l'utilisateur : elle n'est jamais supprimée, quel que soit le réglage et quoi que devienne le ticket.
- Si le worktree ne peut être supprimé, l'issue finit par « · worktree gardé ».
- L'agent est archivé. Avec « Valider », qui laisse la branche en l'état, il ne l'est que quand son worktree est supprimé.

### Les worktrees des tickets finis

- Le réglage « Supprimer le worktree d'un ticket terminé ou supprimé » est allumé par défaut. Éteint, worktrees et agents restent.
- Allumé, sont supprimés après les commandes de démontage du projet : le worktree d'un ticket terminé, quelle que soit l'action de validation ; ceux d'un ticket supprimé, dans n'importe quelle colonne ; et ceux des agents que le ticket a eus avant (renvoyé « À faire », repris par un autre).
- Jamais avec une perte. Un worktree est gardé :
  - s'il a des changements non commités (fichiers nouveaux compris, fichiers ignorés exclus) ;
  - si sa branche a des commits qui ne sont ni dans la branche cible (merge et squash compris) ni dans aucune autre branche locale ou distante (poussée, proposée en PR) ;
  - si git ne répond pas.
- C'est dit. « Valider » sans action ajoute à l'issue « · worktree gardé : 2 fichier(s) non commité(s) » (ou « … : 1 commit(s) ni mergé(s) ni poussé(s) ») et laisse l'agent tel quel. L'agent archivé d'un ticket supprimé le dit sur sa carte de la liste « Archivés », jusqu'à sa restauration. La confirmation de suppression d'un ticket en cours prévient.
- Un worktree supprimé emporte sa branche quand son travail est dans la cible (mergée ou vide). Poussée, proposée ou reprise, la branche reste, comme celle qu'un agent non archivé a encore.
- Un worktree qu'un autre agent non archivé utilise n'est jamais touché.
- Supprimer un ticket « Terminé » laissé en l'état archive aussi son agent.

### Renvoyer

« Renvoyer » rend le ticket au même agent, avec « Ce qui ne va pas ». Il est refusé lui aussi tant que l'agent travaille.

## Réglages

Dans l'onglet « Kanban » du projet : l'action après validation (« ⚙ Après validation : … » dans l'en-tête), la façon de merger (merge commit, squash ou rebase), la PR en brouillon, la conduite en cas de conflit, la commande de tests, et « Supprimer le worktree d'un ticket terminé ou supprimé » (allumé par défaut).
