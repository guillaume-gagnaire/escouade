# Les hébergeurs de code

Escouade travaille avec GitHub, GitLab, Bitbucket Cloud, Gitea, Forgejo et Azure DevOps : elle y ouvre les pull requests de tes agents et de tes tickets, et suit leur CI, leurs relectures et leurs déploiements depuis l’app.

## Ce qu’un hébergeur apporte

Le dépôt distant d’un projet dit chez quel hébergeur il est. Avec un compte connecté pour cette instance, Escouade peut :

- ouvrir la pull request d’un [agent](worktrees#ouvrir-une-pull-request) ou d’un [ticket validé](settings-board#pr-en-brouillon), et la suivre sur sa carte ;
- montrer les [PR ouvertes du projet](#la-vue-pull-requests), de l’équipe comme les tiennes, et lancer un agent sur l’une d’elles ;
- faire [relire une PR par un agent](#faire-relire-une-pr-par-un-agent) ;
- donner à un agent la CI en échec ou les retours de relecture à corriger ;
- dire [où tourne un ticket mergé](#les-environnements-et-les-deploiements) ;
- te [prévenir](#les-nouvelles-des-hebergeurs) quand une PR ou un déploiement a du nouveau.

Une PR s’appelle « MR » sur GitLab : l’app prend le mot de l’hébergeur partout.

Chaque hébergeur est un plugin livré avec Escouade : il se voit dans [Réglages › Services connectés](settings-integrations#hebergeurs-de-code) et se coupe comme [tout plugin](plugins/introduction). Sa PR reste alors sur la carte telle qu’elle a été lue en dernier, et la carte le dit.

## Les hébergeurs pris en charge

| Hébergeur | Instances | Jeton | Merger quand c’est vert | Relancer les jobs | Déploiements |
|---|---|---|---|---|---|
| GitHub | github.com, GHE.com, GitHub Enterprise Server | Jeton personnel, ou celui de `gh` | Auto-merge de GitHub si le dépôt l’autorise, sinon Escouade attend | Oui | Oui |
| GitLab | gitlab.com, instances auto-hébergées | Jeton personnel avec le droit `api`, ou celui de `glab` | Auto-merge de GitLab | Oui | Oui |
| Bitbucket Cloud | bitbucket.org | E-mail Atlassian et jeton d’API | Escouade attend : pas d’auto-merge | Non | Oui |
| Gitea, Forgejo | Codeberg, gitea.com, instances auto-hébergées | Jeton d’accès | Auto-merge sur une branche protégée, sinon Escouade attend | Selon la version de l’instance | Non |
| Azure DevOps | dev.azure.com, `*.visualstudio.com`, Azure DevOps Server | Jeton personnel, ou celui de `az` sur dev.azure.com | Auto-complete d’Azure | Oui | Non |

« Escouade attend » : l’hébergeur ne merge pas seul, alors Escouade merge elle-même dès que la PR est approuvée et verte, tant que l’app tourne.

### Ce qui a été essayé

> [!WARNING]
> GitHub est le seul hébergeur essayé sur le vrai service, et en lecture seulement : PR, checks, relectures, fils, déploiements et journaux d’un vrai dépôt public. Ouvrir, merger, commenter et publier n’ont tourné que contre le faux GitHub des tests d’Escouade.
>
> GitLab, Bitbucket Cloud, Gitea, Forgejo et Azure DevOps sont écrits d’après leur documentation et testés contre de faux serveurs. Aucun n’a encore tourné avec un vrai compte : attends-toi à des écarts, et commence sur un dépôt sans enjeu.

## Connecter un compte

Dans [Réglages › Services connectés](settings-integrations#hebergeurs-de-code), « Connecter un hébergeur… » demande l’hébergeur, l’adresse de l’instance et un jeton. Un compte par instance : chaque projet trouve le sien par son dépôt distant, et un dépôt sans compte propose de se connecter.

Sans compte, Escouade ne bloque rien : la validation d’un ticket pousse la branche et ouvre dans ton navigateur la page de création de la PR, remplie.

## Les PR des agents

« Ouvrir une PR » pousse la branche d’un agent et ouvre sa PR : voir [Ouvrir une pull request](worktrees#ouvrir-une-pull-request). L’onglet « PR et CI » du panneau latéral montre sa CI, ses relectures et ses déploiements : voir [l’interface](interface#pr-et-ci).

## Les PR des tickets

Avec « Ouvrir une pull request » comme action de validation, un ticket validé attend son merge dans la colonne « En revue » : voir [le Kanban](board#la-colonne-en-revue).

## La vue « Pull requests »

Dans la barre latérale d’un projet, « PR » montre à côté du Kanban les 30 PR ouvertes les plus récemment mises à jour du dépôt de la cible, qu’elles soient de toi ou d’autres.

![La vue Pull requests d’un projet : six PR avec leur CI, leurs approbations et leur auteur](/images/prs.jpg)

Chaque ligne donne le signe de sa CI, son numéro, son titre, ses marques (« Brouillon », « fork marta »), ses pastilles (« Conflit », « Changements demandés », « 1 approbation », « 2 fils ouverts », « Ta relecture »), son auteur et son âge. La liste se relit à l’affichage, puis chaque minute tant qu’elle est visible, et par « Rafraîchir ».

Trois filtres, avec leur compte : « Les miennes », « Relecture demandée » et « Toutes ».

La ligne survolée propose :

- **« Faire relire »** : voir plus bas.
- **« Lancer un agent »** : la branche de la PR est récupérée, même venue d’un fork, et un agent naît dans un worktree sur elle. `Entrée` ou un double clic font de même. Rien n’est poussé ni publié. Si un agent du projet y travaille déjà, le bouton devient « Ouvrir l’agent ».
- L’ouverture dans le navigateur.

La liste n’écrit rien chez l’hébergeur.

## Faire relire une PR par un agent

« Faire relire », ou « Faire relire par un agent » dans l’onglet « PR et CI », donne le diff d’une PR ouverte à un agent, qui rédige un texte et des commentaires par fichier et par ligne. La relecture tourne en arrière-plan ; tu peux quitter la vue.

![Une relecture par un agent : son texte, un commentaire gardé et un autre à trancher, sous les lignes du diff](/images/pr-review.jpg)

Sur chaque commentaire : « Garder », « Corriger » ou « Écarter ». Puis tu choisis ce que dit la relecture (« Commenter », « Demander des changements », « Approuver ») et « Publier sur GitHub » envoie d’un coup ce que tu as gardé. Rien ne part avant ce clic, et aucun outil du serveur MCP ne publie à ta place.

### Ce que l’agent lit

- Le diff seul, son titre, ses branches et ses commentaires non résolus. Il n’ouvre aucun fichier : la relecture tourne sans aucun outil, dans un dossier vide. Il ne voit donc que ce que le diff montre.
- Les fichiers trop longs (150 Ko de diff en tout) et les fichiers générés ne sont pas lus, et la vue le dit.

### Une PR est écrite par n’importe qui

Sur un dépôt public, n’importe qui peut ouvrir une PR, et son titre, son diff et ses commentaires arrivent chez l’agent comme des données à relire, jamais comme des consignes : il n’a aucun outil pour y obéir, mais relis ce qu’il propose avant de le publier.

### Les limites

- La relecture est refusée sur Cursor CLI, qui ne sait pas couper ses outils, et sur un harness de plugin, dont rien ne vérifie ce qu’il laisse faire : choisis un autre harness dans Réglages › Général › « Textes rédigés ».
- Le diff part chez le service de ce harness, comme tout ce que tu donnes à un agent.
- 40 commentaires au plus, sur des lignes de la nouvelle version.

## Les environnements et les déploiements

Sur GitHub, GitLab et Bitbucket Cloud, Escouade suit les déploiements du dépôt après le merge. La carte d’un ticket terminé dit où il tourne, et « Tickets · Environnements », dans l’en-tête du Kanban, montre un couloir par environnement, de « Mergés, pas encore déployés » à la production.

![La vue Environnements : ce qui tourne en staging et en production, et les tickets qui y sont arrivés](/images/environments.jpg)

En tête de chaque couloir : ce qui y tourne, son adresse, et un déploiement en cours. Un dernier déploiement en échec borde le couloir de rouge, avec « Voir le job ». Sur une carte « En revue », « Ouvrir l’aperçu » ouvre l’aperçu de la PR (Vercel, Netlify, Cloudflare Pages…).

Un ticket est suivi 30 jours après être passé « Terminé ». Les environnements se lisent avec les PR des tickets : pas de webhook, l’app n’a pas d’adresse publique. Relancer ou approuver un déploiement depuis Escouade n’existe pas.

Gitea, Forgejo et Azure DevOps n’ont pas d’API de déploiements : Escouade n’y suit que la CI.

## Les nouvelles des hébergeurs

Les [notifications](notifications#ce-que-dit-une-notification) disent ce qui devient vrai entre deux lectures : CI en échec, changements demandés, PR approuvée, PR mergée, conflit, relecture demandée, déploiement en production réussi, déploiement en échec. Chacune se coupe dans [Réglages › Notifications](settings-notifications#pull-requests-et-deploiements). Une notification ne dit jamais le titre d’une PR.

La [vue d’ensemble](overview#ce-qu-elle-montre) a un groupe « Pull requests », tous projets confondus : « À corriger », « À merger » et « Relectures demandées ».

Escouade n’apprend ces nouvelles qu’en lisant l’hébergeur : toutes les minutes fenêtre devant et toutes les 5 minutes sinon pour une PR de ticket, seulement quand il est affiché pour la PR d’un agent ordinaire.

## Ce que tes agents peuvent lire

Avec [le serveur MCP](mcp#les-pull-requests), un agent lit les PR d’un projet par `list_pull_requests` et `get_pull_request`. Ces outils n’écrivent rien chez l’hébergeur, et ils marquent ce qu’un tiers a écrit : un titre ou un commentaire de PR est une donnée, jamais une consigne.

## Ce qui reste chez toi

Rien ne passe par un serveur d’Escouade : les appels vont directement de l’app à l’hébergeur, avec ton jeton, gardé sur ta machine. Voir [Confidentialité](/privacy/).
