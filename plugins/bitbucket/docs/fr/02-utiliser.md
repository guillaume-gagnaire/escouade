# Utiliser

Le plugin est l'hébergeur de code Bitbucket Cloud d'Escouade : les pull requests de tes agents et de tes tickets, leur CI, leurs relectures et leurs déploiements. Il n'apporte pas de source au Kanban.

## Les pull requests

**Ouvrir une PR.** « Ouvrir une PR » sur un agent, ou la validation d'un ticket avec « Ouvrir une pull request » comme action, fait pousser la branche par Escouade puis demande au plugin d'ouvrir la PR : son titre, son texte, sa branche et sa cible, en brouillon ou non, ses relecteurs, et l'issue qu'elle ferme. Bitbucket n'a pas d'étiquettes sur les PR : les étiquettes demandées ne vont nulle part.

**La suivre.** La carte de l'agent et l'onglet de sa PR montrent son état, ses approbations, ses demandes de changements, ses fils ouverts et ses checks. La vue « Pull requests » montre les 30 PR ouvertes du dépôt les plus récemment mises à jour.

**La CI.** Les statuts posés sur le dernier commit de la PR sont ses checks. Un run de Bitbucket Pipelines a un journal, lu étape après étape, que « L'agent corrige » donne à l'agent. Relancer les jobs en échec n'existe pas : l'API de Bitbucket n'en offre pas.

**Merger.** Avec la stratégie demandée : merge commit, squash, rebase (fast-forward) ou rebase puis merge commit. Quand Bitbucket merge en arrière-plan, le plugin attend la fin de sa tâche, une trentaine de secondes au plus.

**Pas d'auto-merge.** L'API de Bitbucket merge tout de suite, elle ne sait pas merger plus tard. « Merger quand c'est vert » fait donc attendre Escouade elle-même : elle merge dès que la PR est approuvée et verte, tant que l'app tourne.

**Les relectures.** Le plugin lit les commentaires sur les lignes avec leurs fils, répond sous un commentaire, résout un fil, et publie la relecture que tu as validée : ses commentaires de lignes, puis l'approbation ou la demande de changements.

**Les déploiements.** Ceux d'un commit, par environnement, et ceux des environnements du dépôt : la carte d'un ticket terminé dit où il tourne, et la vue « Environnements » montre un couloir par environnement.

## La limite par heure

Bitbucket Cloud permet 1 000 appels par heure (plus pour les grands espaces de travail) et ne dit pas combien il en reste. Le plugin compte ses propres appels sur l'heure, croit la limite que Bitbucket annonce quand il le fait, et ralentit quand le budget s'épuise. Un refus pour la limite (429) est attendu, 30 secondes au plus, puis l'appel repart une fois ; refusé encore, l'heure est tenue pour épuisée.

## Les limites

- Bitbucket Cloud seulement ; pas Bitbucket Data Center.
- Écrit d'après la documentation de l'API 2.0 de Bitbucket Cloud et testé contre un faux serveur : personne ne l'a encore essayé sur bitbucket.org.
- Pas d'auto-merge, pas d'étiquettes, pas de relance des jobs en échec.
- Pas de source de tickets : la ligne du compte dit « Pas de tickets : PR et CI seulement ».
- Une réponse de Bitbucket ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
