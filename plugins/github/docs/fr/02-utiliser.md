# Utiliser

Le plugin sert deux choses d'Escouade sur le même compte : l'hébergeur de code GitHub (les pull requests, leur CI, leurs relectures, leurs déploiements) et, sur le compte de github.com, la source GitHub Issues du Kanban.

## Les pull requests

**Ouvrir une PR.** « Ouvrir une PR » sur un agent, ou la validation d'un ticket avec « Ouvrir une pull request » comme action, fait pousser la branche par Escouade puis demande au plugin d'ouvrir la PR : son titre, son texte, sa branche et sa cible, en brouillon ou non, ses relecteurs et ses étiquettes. Quand elle ferme une issue, le texte finit par « Closes #42 ». Un relecteur ou une étiquette que GitHub refuse (l'auteur lui-même, un compte sans accès) ne fait pas échouer la PR, qui reste ouverte.

**La suivre.** La carte de l'agent et l'onglet de la PR dans son panneau latéral montrent son état, brouillon ou non, ses conflits, ses relectures, ses fils ouverts et sa CI. Toutes les PR qu'un projet suit sont lues en une seule requête GraphQL.

**La CI.** Les check runs et les statuts du dernier commit de la PR (ou de la branche, quand il n'y a pas de PR). Pour un job d'Actions, « L'agent corrige » donne à l'agent son journal ; relancer les jobs en échec relance chaque run une fois.

**Merger.** Le merge suit ce que le dépôt permet (merge, squash, rebase). « Merger quand c'est vert » demande l'auto-merge de GitHub si le dépôt l'autorise ; sinon Escouade attend elle-même que la PR soit approuvée et verte, tant que l'app tourne.

**Les relectures.** Le plugin lit les relectures et les commentaires de la PR, avec leurs fils. Quand tu fais relire une PR par un agent, c'est lui qui publie, en une fois, ce que tu as gardé : le texte, les commentaires sur les lignes et ton choix (« Commenter », « Demander des changements », « Approuver »). Il répond aussi sous un commentaire d'une ligne.

**Les déploiements.** Le plugin lit les déploiements d'un commit et ceux des environnements du dépôt : la carte d'un ticket terminé dit où il tourne, et la vue « Environnements » montre un couloir par environnement.

**La vue « Pull requests ».** Elle montre les 30 PR ouvertes du dépôt les plus récemment mises à jour, de toi ou d'autres.
## GitHub Issues

Sur le compte de github.com, le plugin est aussi une source du Kanban.

**Importer.** Dans la fenêtre « Importer des tickets », l'onglet GitHub montre les issues ouvertes du dépôt lié, sans les pull requests :

- « Rechercher une issue… » : un numéro (`42` ou `#42`) trouve l'issue ; un autre texte la cherche par la recherche de GitHub, 50 à la fois, avec le nombre total ;
- « Assignées à moi », puis une puce par étiquette du dépôt (douze au plus) ;
- sans recherche ni puce, la liste vient 100 issues à la fois.

**Les critères.** Avec « Extraire les critères d'acceptation », ce sont les éléments de liste sous un titre ou une ligne qui nomme les critères (« Critères d'acceptation », « Acceptance criteria », « Definition of done », « AC », « DoD »…), sinon les cases à cocher (`- [ ]`) de la description.

**L'issue suit son ticket.** Dans la « Correspondance des statuts », les états d'une issue sont « Ouverte », « Fermée », puis les étiquettes du dépôt (marquées ◆). « Fermée » ferme l'issue (terminée) ; une étiquette la rouvre si besoin, l'ajoute, et retire les autres étiquettes de la correspondance. Le commentaire s'écrit dans la discussion de l'issue.

**L'import automatique** prend les issues ouvertes qui portent l'étiquette choisie (`claude-ready` par défaut).

## Les limites

- GitHub Issues ne sert que sur le compte de github.com, pas sur un GitHub Enterprise.
- La liste des dépôts de GitHub Issues propose les 100 dépôts où tu as poussé le plus récemment (les tiens, ceux où tu collabores, ceux de tes organisations).
- Le plugin ménage la limite de l'API : il relit sous condition (une réponse `304` ne compte pas), ralentit quand la limite approche, et attend ce que GitHub demande quand il refuse pour sa limite secondaire (une minute s'il ne dit rien) avant de redemander une fois.
- Le journal d'un job d'Actions sur un Enterprise Server qui le range sur un autre hôte que lui n'est pas lu : rien ne nomme cet hôte dans les permissions du plugin.
- Il ne relance pas un déploiement et n'en approuve pas.
- Une réponse de GitHub ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
