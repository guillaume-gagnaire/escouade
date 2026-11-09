# Réglages : Services connectés et Tickets importés

La page « Services connectés » connecte les hébergeurs de code et les comptes de Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues et Azure Boards, et règle la synchronisation et l’import automatique, pour tous les projets.

La page « Tickets importés » de chaque projet le lie à ses sources et fait correspondre leurs statuts. Un point ambre sur « Services connectés », dans la barre des réglages, dit un jeton expiré ou illisible.

## Ouvrir les pages

Ouvre les réglages avec `Ctrl+,` (`⌘,` sous macOS), puis choisis « Services connectés », ou, sous « Projets », la page « Tickets importés » du projet. Les tickets importés et leur synchronisation sont expliqués dans [les intégrations](integrations).

Services connectés a deux parties, chacune avec son titre et une ligne qui dit à quoi elle sert :

- **« Hébergeurs de code »** : « PR et MR des agents, CI et déploiements : un compte par instance, que chaque projet trouve par son remote. » (voir [plus bas](#hebergeurs-de-code)) ;
- **« Sources du Kanban »** : « Import et synchro des tickets : un ticket importé suit sa colonne là-bas (état, commentaires). » Le reste de la page est sous elle.

Un hébergeur n’est pas une source : connecter GitLab ou GitHub comme hébergeur sert aux PR et à la CI ; ses tickets ne viennent dans le Kanban qu’une fois une source liée au projet. GitHub Issues, GitLab Issues et Azure Boards reprennent pour cela le compte de leur hébergeur, sans rien redemander.

Les sources réunissent trois sortes de réglages :

- « Comptes connectés » est commun à toute l’app et s’enregistre tout de suite.
- « Sources liées à … » et « Correspondance des statuts » sont dans la page « Tickets importés » de chaque projet. Ils s’enregistrent au geste.
- « Synchronisation » et « Import automatique » sont communs à tous les projets, et s’enregistrent au geste.

Sans projet ouvert, « Tickets importés » affiche « Aucun projet ouvert. ».

## Comptes connectés

Un compte par service : Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues et Azure Boards. Chaque service affiche « Non connecté » avec un bouton « Connecter… », ou « Connecté · » suivi du compte avec un bouton « Déconnecter ». Sous le titre, l’app rappelle : « Les jetons restent sur cette machine, à part des réglages, et ne servent qu’aux appels de ces services. »

GitLab Issues n’a pas de compte à lui : sa ligne dit « Compte de l’hébergeur GitLab : il se connecte et se déconnecte dans Hébergeurs de code. », et son bouton « Connecter GitLab… » ouvre le formulaire de l’hébergeur (voir [GitLab Issues](#gitlab-issues)). Azure Boards de même avec l’hébergeur Azure DevOps : « Compte de l’hébergeur Azure DevOps : il se connecte et se déconnecte dans Hébergeurs de code. », « Son jeton doit avoir le droit Work Items (lecture et écriture). » et « Connecter Azure DevOps… » (voir [Azure Boards](#azure-boards)).

« Connecter… » ouvre le formulaire du service, dont les champs sont décrits ci-dessous. Les jetons et les clés d’API de Linear et de ClickUp sont masqués à la saisie ; la clé d’API de Trello ne l’est pas.

### Jira

| Champ | À renseigner |
|---|---|
| « Site » | L’adresse de ton site Atlassian, par exemple `atlas.atlassian.net`. Sans `https://`, Escouade l’ajoute. Seul https est accepté (http seulement vers cette machine). |
| « E-mail » | L’e-mail du compte auquel appartient le jeton, par exemple `ada@atlas.dev`. |
| « Jeton d’API » | Un jeton d’API Atlassian. |

Le lien « Créer un jeton d’API Atlassian » ouvre `https://id.atlassian.com/manage-profile/security/api-tokens`. Une fois connecté, le compte s’affiche avec l’e-mail et le site, par exemple « ada@atlas.dev · atlas.atlassian.net ». Ce que Jira appelle l’endroit des tickets est un « Projet ».

### Trello

| Champ | À renseigner |
|---|---|
| « Clé d’API » | La clé d’API de Trello. |
| « Jeton » | Un jeton pour cette clé. |

Le lien « Obtenir une clé d’API » ouvre `https://trello.com/power-ups/admin`. Le lien « Obtenir un jeton pour cette clé », grisé tant que la clé est vide, ouvre la page d’autorisation de Trello pour la clé saisie : accès en lecture et en écriture, sans expiration, au nom d’« Escouade ». L’endroit des tickets est un « Tableau ».

### GitHub Issues

| Champ | À renseigner |
|---|---|
| « Jeton (vide : celui de gh) » | Un jeton GitHub. Laissé vide, Escouade demande son jeton à la CLI GitHub (`gh auth token`) : `gh` doit être installée et connectée (`gh auth login`). |

Le lien « Créer un jeton GitHub » ouvre `https://github.com/settings/tokens`. Un compte qui passe par `gh` s’affiche avec « · via gh ». L’endroit des tickets est un « Dépôt ».

### Linear

| Champ | À renseigner |
|---|---|
| « Clé d’API » | Une clé d’API personnelle de Linear. |

Le lien « Créer une clé d’API Linear » ouvre `https://linear.app/settings/account/security`. L’endroit des tickets est une « Équipe ».

### ClickUp

| Champ | À renseigner |
|---|---|
| « Clé d’API » | Une clé d’API personnelle de ClickUp (elle commence par `pk_`). |

Le lien « Créer une clé d’API ClickUp » ouvre `https://app.clickup.com/settings/apps`. Une fois connecté, le compte s’affiche avec ton nom chez ClickUp, par exemple « Connecté · Ada Lovelace ». L’endroit des tickets est une « Liste ». La connexion d’une application ClickUp par OAuth n’est pas proposée : une clé personnelle suffit et reste sur ta machine.

### GitLab Issues

Aucun champ : GitLab Issues prend les comptes de l’hébergeur GitLab (gitlab.com et les instances auto-hébergées, un compte par instance), avec leur jeton personnel (droit `api`) ou celui de la CLI `glab`. Le jeton reste dans le trousseau du système, comme celui de l’hébergeur : c’est le même. « Connecter GitLab… » ouvre le formulaire de « Hébergeurs de code », hébergeur GitLab déjà choisi : adresse (vide : gitlab.com), jeton (vide : celui de `glab`). Connecté, la ligne dit « Connecté · @ada », suivi de l’instance quand elle n’est pas gitlab.com (« @ada · git.acme.dev »). Pour le déconnecter, déconnecte l’instance dans « Hébergeurs de code ». L’endroit des tickets est un « Projet ».

### Azure Boards

Aucun champ : Azure Boards prend les comptes de l’hébergeur Azure DevOps (une organisation de dev.azure.com, ou une collection d’Azure DevOps Server), avec leur jeton personnel ou celui de la CLI `az`. Il faut au jeton le droit Work Items (lecture et écriture) en plus de Code et Build ; sans lui, Azure refuse, et l’erreur le dit avec l’adresse de la page des jetons de l’organisation. « Connecter Azure DevOps… » ouvre le formulaire de « Hébergeurs de code », hébergeur Azure DevOps déjà choisi : adresse (`dev.azure.com/acme`), jeton (vide : celui de `az`). Connecté, la ligne dit « Connecté · ada@acme.dev · acme » (le compte, puis l’organisation). Pour le déconnecter, déconnecte l’organisation dans « Hébergeurs de code ». L’endroit des tickets est un « Projet ».

### Connecter et déconnecter

Clique sur « Connecter » une fois les champs remplis ; le bouton dit « Vérification… » pendant l’appel, et « Annuler » ferme le formulaire. Escouade vérifie les identifiants auprès du service et ne les garde que s’il les accepte. S’il les refuse, ou s’il est injoignable, sa réponse s’affiche en rouge sous les champs, par exemple « Jira refuse ces identifiants (401) ». Un champ manquant est signalé avant tout appel, par exemple « Indique le jeton. ».

« Déconnecter » oublie le compte et ses secrets. Les projets gardent leurs sources liées : elles ne servent plus tant qu’un compte du service n’est pas reconnecté. Pour changer de compte, déconnecte l’ancien puis connecte l’autre.

### Où sont gardés les jetons

Les clés et les jetons vont dans le trousseau du système (Credential Manager sous Windows, Keychain sous macOS) ; le reste du compte est dans `~/.escouade/integrations.json`. Sous Linux, où Escouade ne connaît pas de trousseau, et quand le trousseau refuse une écriture, les secrets restent dans ce fichier, lisible par toi seul. Le compte le dit alors : « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. »

Si le trousseau ne rend pas un jeton au démarrage, le compte affiche « Trousseau du système illisible : relance Escouade ou reconnecte le compte. » et aucun appel ne part tant que c’est le cas. Voir [les données locales](data).

## Sources liées à un projet

Le groupe s’appelle « Sources liées à » suivi du nom du projet. Tant qu’aucun compte n’est connecté, il dit : « Connecte un compte ci-dessus pour lier une source à ce projet. »

Pour chaque service connecté, un menu choisit la source du projet : « Projet Jira », « Tableau Trello », « Dépôt GitHub Issues », « Équipe Linear », « Liste ClickUp », « Projet GitLab Issues » ou « Projet Azure Boards ». « Aucun » (par défaut) délie. Un projet a au plus une source par service.

- **Jira** : les projets du site, présentés sous la forme « CLÉ — nom ».
- **Trello** : tes tableaux ouverts.
- **GitHub Issues** : les dépôts auxquels tu as accès (propriétaire, collaborateur ou membre d’une organisation), les derniers poussés d’abord. Le dépôt du projet, lu dans son remote git, est en tête avec « (dépôt du projet) ».
- **Linear** : tes équipes, sous la forme « Nom (CLÉ) ».
- **GitLab Issues** : les projets dont tu es membre, sur chaque instance connectée (rangés sous l’instance), leur chemin complet avec les groupes et sous-groupes (« acme/back/api »), les derniers actifs d’abord. Le projet du remote du projet est en tête avec « (dépôt du projet) ». Les groupes ne sont pas proposés : les issues vivent dans les projets.
- **Azure Boards** : les projets de l’organisation connectée (rangés sous elle), par nom. Le projet du remote du projet est en tête avec « (dépôt du projet) ».
- **ClickUp** : les listes de tes espaces de travail, rangées sous leur espace et leur dossier (« Produit › Sprints » ; l’espace de travail devant quand tu en as plusieurs), les listes hors dossier d’abord, les archivées en moins.

Choisir une source charge ses statuts et préremplit la correspondance ci-dessous. Si les statuts ne se chargent pas, la source est liée quand même, et l’infobulle des menus de la correspondance dit pourquoi.

## Correspondance des statuts

Ce groupe n’apparaît qu’une fois une source liée à un compte connecté. Il dit : « L’état que prend le ticket externe quand son ticket arrive dans la colonne ; « Commenter » y ajoute un commentaire (critères, issue de la validation…). »

Une ligne par colonne du Kanban (« À faire », « En cours », « À tester », « Terminé »), un menu par service lié, et un interrupteur « Commenter » en bout de ligne.

### Le statut de chaque colonne

Chaque menu propose « — inchangé » (le ticket externe n’est pas touché) et les statuts de la source :

| Service | Statuts proposés | Ce que fait le changement de statut |
|---|---|---|
| Jira | Les statuts des types de ticket du projet | Escouade applique la transition du workflow qui mène à ce statut. Si le workflow n’y mène pas depuis le statut actuel, c’est refusé. |
| Trello | Les listes ouvertes du tableau | La carte passe dans la liste. |
| GitHub Issues | « Ouverte », « Fermée », et les étiquettes du dépôt (précédées de ◆) | « Fermée » ferme l’issue comme terminée ; « Ouverte » la rouvre. Une étiquette est posée sur l’issue (rouverte si besoin), et les autres étiquettes de la correspondance sont retirées. |
| Linear | Les états du workflow de l’équipe | Le ticket prend cet état. |
| GitLab Issues | « Ouverte », « Fermée », les étiquettes des listes des tableaux du projet, puis ses autres étiquettes (précédées de ◆) | Comme GitHub : « Fermée » ferme l’issue, « Ouverte » la rouvre, une étiquette est posée (l’issue rouverte si besoin) et les autres étiquettes de la correspondance retirées, en un seul appel. |
| Azure Boards | Les états des types d’éléments de travail du projet selon son processus (par exemple New, Active, Resolved, Closed, ou To Do, Doing, Done), par catégorie, sans les états supprimés | L’état est posé en un appel (`System.State`, JSON Patch). Un élément dont le type n’a pas cet état prend celui de son type de la même catégorie. Par défaut, « En cours » et « À tester » : un état en cours ; « Terminé » : l’état résolu, quand le processus en a un. |
| ClickUp | Les statuts de la liste liée (chaque liste a les siens) | La tâche prend ce statut. Si la liste ne l’a plus, la carte le dit et rien n’est envoyé. |

Quand tu lies une source, Escouade préremplit les menus d’après le nom des statuts : celui qui ressemble à « en cours » (« In Progress », « En cours », « Doing »…) pour « En cours » et « À tester », et celui qui ressemble à « à tester » (« To test », « QA », « In Review », « À tester »…) pour « Terminé ». « À faire » reste « — inchangé ». Pour ClickUp, quand les noms ne disent rien, le type des statuts décide : le premier statut personnalisé pour « En cours » et « À tester », le suivant (ou celui de type fini) pour « Terminé ». Corrige les menus si ton workflow est autre.

### Commenter

L’interrupteur « Commenter » d’une colonne ajoute un commentaire sur le ticket externe quand un ticket arrive dans la colonne. Il vaut pour toutes les sources du projet. Par défaut, il est activé pour « À tester » et « Terminé », désactivé pour « À faire » et « En cours ».

Le commentaire est écrit dans la langue de « Langue des textes rédigés par les agents » (page [« Général »](settings-application)) :

| Colonne | Commentaire |
|---|---|
| « À faire » | « Escouade : ATL-42 est revenu « À faire ». » |
| « En cours » | « Escouade : ATL-42 est pris par un agent (branche ticket/ATL-42). » |
| « À tester » | « Escouade : ATL-42 est prêt à tester. », ou « … (objectif partiel). » quand le ticket arrive sans tous ses critères faute de boucles. Suivent la liste des critères et « Ce qui a été fait » quand le ticket en a. |
| « Terminé » | « Escouade : ATL-42 est terminé — » suivi de l’issue de la validation, puis le lien de la PR s’il y en a une. |

## Synchronisation

Ces trois réglages valent pour tous les projets.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Mettre à jour le statut externe » | Quand un ticket importé change de colonne dans le Kanban, le ticket externe prend le statut que la correspondance donne à la colonne. Désactivé, les statuts ne sont plus touchés ; les commentaires de la correspondance restent publiés. | Activé |
| « Publier un résumé à chaque boucle » | Quand un ticket entame une nouvelle boucle (à partir de la deuxième), un commentaire donne les critères atteints et les notes de l’agent sur le ticket externe. | Désactivé |
| « Extraire les critères d’acceptation » | À l’import, lit les critères dans la description (Jira, GitHub, Linear, GitLab), la checklist (Trello, ClickUp) ou le champ Acceptance Criteria (Azure Boards, sinon sa description) du ticket importé. Désactivé, le ticket reçoit les critères par défaut. | Activé |

Si un service refuse ou n’est pas joignable, l’opération n’est pas perdue : elle est retentée après 1, 5, 15 puis 60 minutes, pendant 24 h au plus. La carte du ticket affiche alors l’erreur et « Resynchroniser » pour réessayer tout de suite. Un refus que réessayer ne changerait pas (ticket supprimé, workflow qui ne mène pas au statut, liste ClickUp disparue ou statut qu’elle n’a plus) est mis de côté sans attendre. Une limite de requêtes de ClickUp (429) est attendue jusqu’à sa fin annoncée, une minute au plus, puis la requête repart une fois.

## Import automatique

Ces trois réglages valent aussi pour tous les projets. L’import à la main est le bouton « Importer » de l’en-tête du Kanban : voir [les intégrations](integrations).

| Réglage | Effet | Par défaut |
|---|---|---|
| « Importer les tickets étiquetés » | Les tickets ouverts des sources liées qui portent l’étiquette arrivent dans « À faire », avec 5 boucles maximum. | Désactivé |
| « Étiquette » | Le nom de l’étiquette : un label Jira, une étiquette Trello, GitHub, Linear ou GitLab, un tag ClickUp ou Azure Boards. Vide, rien n’est importé. | `claude-ready` |
| « Vérifier toutes les » | L’intervalle entre deux vérifications : 5 min, 15 min ou 60 min. | 15 min |

L’import automatique n’importe un ticket qu’une fois par projet : un ticket déjà importé, même supprimé du Kanban depuis, n’est pas ramené tout seul (l’import à la main peut le reprendre, tant qu’il n’est plus dans le Kanban). Chaque import affiche un message du genre « 3 tickets importés depuis Jira dans demo ». La première vérification a lieu environ 30 secondes après le démarrage de l’app, ou au plus 30 secondes après l’enregistrement si tu actives l’import ensuite ; les suivantes, à l’intervalle choisi.

## Hébergeurs de code

La première partie de « Services connectés » connecte les [hébergeurs de code](code-hosts) : GitHub, GitLab, Bitbucket Cloud, Gitea, Forgejo et Azure DevOps. Ils servent aux pull requests (merge requests sur GitLab) des agents et des tickets, à la CI et aux déploiements, pas aux tickets eux-mêmes. Un compte par instance (github.com, gitlab.com, celle de ton entreprise…) ; chaque projet trouve le sien par son remote. « Connecter un hébergeur… » demande l’hébergeur, l’adresse (vide : son instance publique) et un jeton (vide : celui de sa CLI, `gh`, `glab` ou `az`) ; un remote sans compte propose « Connecter » suivi de son hôte.

Chaque instance dit sur sa ligne si elle sert aussi de source du Kanban :

- « Sert aussi à GitHub Issues » : le compte de github.com ;
- « Sert aussi à GitLab Issues » : chaque compte GitLab, gitlab.com ou auto-hébergé ;
- « Sert aussi à Azure Boards » : chaque compte Azure DevOps ;
- « Pas de tickets : PR et CI seulement » : Bitbucket, Gitea, Forgejo et un GitHub d’entreprise.

### Azure DevOps

Un compte Azure DevOps est celui d’une organisation : son adresse est `dev.azure.com/<organisation>` (un ancien `<organisation>.visualstudio.com` est la même), ou pour Azure DevOps Server l’adresse de sa collection, comme `https://serveur/tfs/DefaultCollection`. Ses remotes sont lus sous toutes leurs formes : `https://dev.azure.com/<organisation>/<projet>/_git/<dépôt>`, `<organisation>.visualstudio.com`, ssh `git@ssh.dev.azure.com:v3/<organisation>/<projet>/<dépôt>`, et ceux d’un Server.

Le jeton est un jeton d’accès personnel (PAT), créé sur la page qu’ouvre « Créer un jeton », avec ces droits : Code (lecture et écriture), Build (lecture et exécution) et Work Items (lecture et écriture). Laissé vide, Escouade prend celui du CLI `az` si tu y es connecté (`az login`), redemandé avant qu’il expire. La ligne dit alors qui est le jeton, par l’e-mail de son compte.

Sur Azure DevOps, la pull request d’un agent s’ouvre en brouillon ou non, depuis « Ouvrir une PR ». Sa carte montre les votes des relecteurs, ses conflits et ses checks : les stratégies de build et de statut de la branche, les statuts posés sur la pull request et les builds d’Azure Pipelines. « L’agent corrige » lui passe le journal des tâches en échec ; « L’agent corrige les retours » les commentaires des fils ouverts. Le merge suit les stratégies que la branche cible permet (merge, squash, rebase, semi-linéaire), et l’auto-merge est l’auto-complete d’Azure.
