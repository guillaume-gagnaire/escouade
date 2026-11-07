# Utiliser

Le plugin est l'hébergeur de code Gitea / Forgejo d'Escouade (Codeberg est un Forgejo) : les pull requests de tes agents et de tes tickets, leur CI et leurs relectures. Il n'apporte pas de source au Kanban.

## Les pull requests

**Ouvrir une PR.** « Ouvrir une PR » sur un agent, ou la validation d'un ticket avec « Ouvrir une pull request » comme action, fait pousser la branche par Escouade puis demande au plugin d'ouvrir la PR : son titre, son texte, sa branche et sa cible, ses relecteurs, ses étiquettes (celles du dépôt dont le nom correspond) et l'issue qu'elle ferme. Gitea et Forgejo n'ont pas de champ « brouillon » à écrire : une PR en brouillon a un titre qui commence par `WIP:`.

**La suivre.** La carte de l'agent et l'onglet de sa PR montrent son état, brouillon ou non, ses relectures, ses fils et sa CI. La vue « Pull requests » montre les 30 PR ouvertes du dépôt les plus récemment mises à jour.

**La CI.** Les statuts combinés du dernier commit. Ceux que posent les Actions sont rattachés à leur job, dont le journal se lit (« L'agent corrige » le donne à l'agent) et dont les jobs en échec se relancent, selon la version de l'instance (voir plus bas).

**Merger.** Avec chaque stratégie que le dépôt permet. « Merger quand c'est vert » programme l'auto-merge de l'instance, qui ne le fait que sur une **branche protégée** : le plugin lit la branche cible d'abord, et refuse en le disant quand elle ne l'est pas. Sinon Escouade attend elle-même que la PR soit approuvée et verte, tant que l'app tourne.

**Les relectures.** Le plugin lit les relectures (avec leurs lignes) et les commentaires, publie la relecture que tu as validée, et répond dans un fil ou le résout là où l'instance le permet.

## Ce qui dépend de la version

Le plugin demande sa version à l'instance (`/api/v1/version`) une fois par compte. Ce que sa version n'a pas, il le dit au lieu d'échouer ou de répondre faux, et Escouade n'en montre rien.

| | Gitea | Forgejo |
| --- | --- | --- |
| Déploiements | aucun (pas d'API) | aucun (pas d'API) |
| Actions : jobs et journaux | 1.25 et après | 12 et après |
| Actions : relancer les jobs en échec | 1.26 et après | non |
| Répondre dans un fil, le résoudre | 1.26 et après | non |

Une instance plus ancienne garde ses statuts (avec leur adresse), sans job ni journal. **Ces versions minimales viennent des sources de Gitea et de Forgejo, pas d'un essai.**

## Les limites

- Écrit d'après les documents d'API de Forgejo 16 et de la branche principale de Gitea, et testé contre un faux serveur : pas encore essayé sur un vrai Codeberg ni un vrai Gitea.
- Pas de déploiements : Escouade n'y suit que la CI.
- L'auto-merge ne marche que sur une branche protégée.
- Pas de source de tickets : la ligne du compte dit « Pas de tickets : PR et CI seulement ».
- Une réponse de l'instance ne peut pas dépasser 8 Mio ni prendre plus de 15 secondes.
