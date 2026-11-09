# Utiliser

Le plugin sert deux choses d'Escouade sur le même compte : l'hébergeur de code Azure DevOps (Azure Repos et Azure Pipelines) et la source Azure Boards du Kanban.

## Les pull requests

**Ouvrir une PR.** « Ouvrir une PR » sur un agent, ou la validation d'un ticket avec « Ouvrir une pull request » comme action, fait pousser la branche par Escouade puis demande au plugin d'ouvrir la PR : son titre, sa description (Azure en refuse plus de 4 000 caractères), sa branche et sa cible, en brouillon ou non, ses relecteurs, ses étiquettes, et l'élément de travail qu'elle ferme une fois complétée.

**La suivre.** La carte de l'agent et l'onglet de sa PR montrent son état, brouillon ou non, ses conflits, les votes de ses relecteurs (une approbation compte à partir de « Approuvé avec suggestions ») et ses fils ouverts. La vue « Pull requests » montre les 30 PR ouvertes du dépôt les plus récemment mises à jour.

**Les checks.** Les évaluations des stratégies de build et de statut de la branche, les statuts posés sur la PR, et les builds d'Azure Pipelines du commit de merge qu'aucune stratégie ne tient. « L'agent corrige » donne à l'agent le journal des tâches en échec ; relancer un build en échec le relance.

**Merger.** Par les stratégies que la stratégie de branche de la cible permet : merge, squash, rebase, semi-linéaire. Une stratégie que la branche ne permet pas est refusée, et le message dit celles qu'elle permet. « Merger quand c'est vert » active l'auto-complete d'Azure.

**Les relectures.** Le plugin lit les fils de commentaires écrits par des personnes (pas ceux d'Azure : votes, pushs), répond dans un fil, le résout, et publie la relecture que tu as validée : ses commentaires, puis ton vote.

**Pas de déploiements.** Les environnements d'Azure Pipelines ne disent rien de simple d'une branche : Escouade n'y suit que la CI.

## Azure Boards

Chaque compte Azure DevOps est aussi une source du Kanban.

**Importer.** Dans la fenêtre « Importer des tickets », l'onglet Azure Boards montre les éléments de travail ouverts du projet lié, les derniers modifiés d'abord. Une requête WIQL donne ceux qui correspondent (1 000 au plus), lus ensuite 50 à la fois :

- « Rechercher un élément de travail (#12) ou par titre… » : un numéro (`12` ou `#12`) trouve l'élément ; un autre texte le cherche dans le titre ;
- « Assignés à moi », puis une puce par type d'élément du projet (User Story, Bug, Task, Product Backlog Item, Issue… selon son processus), puis ses tags (douze au plus) ;
- ne sont pas proposés les éléments dans un état terminé ou supprimé (catégories Completed et Removed de leur processus), ni ceux des types qui ne sont pas d'un backlog (tests, revues, retours).

**Les critères.** Avec « Extraire les critères d'acceptation » : le champ Acceptance Criteria quand il est rempli (ses éléments de liste, sinon ses lignes), sinon la description comme pour Jira ; pour un bug à la description vide, ses étapes de reproduction. Les champs d'Azure Boards sont en HTML : le plugin les convertit en Markdown.

**L'élément suit son ticket.** Dans la « Correspondance des statuts », les états sont ceux des types d'éléments du projet, selon son processus (Basic : To Do, Doing, Done ; Agile : New, Active, Resolved, Closed ; Scrum, CMMI ou un processus hérité : les siens), rangés par catégorie. Par défaut, « En cours » et « À tester » prennent un état en cours (Active, Doing, In Progress…), « Terminé » l'état résolu quand le processus en a un. L'état part en un seul appel ; quand le type de l'élément n'a pas cet état, il prend l'état de son type dans la même catégorie. Le commentaire s'écrit en Markdown dans la discussion de l'élément.

**L'import automatique** prend les éléments ouverts qui portent le tag choisi (`claude-ready` par défaut).

## Les limites

- Écrit d'après la documentation de Microsoft (API REST 7.1) et testé contre un faux serveur : pas encore essayé sur une vraie organisation.
- La CLI `az` ne sert que sur dev.azure.com.
- Azure ne dit pas les droits d'un jeton : un droit manquant se voit au premier refus.
- Pas de déploiements.
- Quand Azure limite les requêtes (429, ou 503 avec `Retry-After`), le plugin attend une fois, 30 secondes au plus ; il ralentit aussi quand Azure annonce sa limite.
- Une réponse d'Azure ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
