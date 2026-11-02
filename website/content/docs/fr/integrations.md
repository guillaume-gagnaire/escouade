# Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards

Escouade importe les tickets de Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues et Azure Boards dans le Kanban, avec leurs critères d’acceptation, puis tient leur statut à jour et y laisse des commentaires au fil du travail. Cette page couvre la connexion, l’import et la synchronisation.

![La fenêtre « Importer des tickets » : des tickets Jira filtrés par « Assignés à moi », avec leurs critères détectés](/images/integrations.jpg)

## Connecter un service

Les comptes se connectent dans Réglages › Intégrations, groupe « Comptes connectés » (voir [Réglages des intégrations](settings-integrations)). Pour chaque service, « Connecter… » ouvre un formulaire. « Connecter » vérifie les identifiants auprès du service (« Vérification… »), puis la ligne dit « Connecté · » suivi de qui tu es chez lui (par exemple `ada@atlas.dev · atlas.atlassian.net`). Si le service refuse, la raison s’affiche sous le formulaire. La connexion est enregistrée tout de suite : elle ne passe pas par « Enregistrer ».

| Service | Ce que tu donnes | Où l’obtenir |
|---|---|---|
| Jira | « Site » (`atlas.atlassian.net`, en https), « E-mail » et « Jeton d’API » | « Créer un jeton d’API Atlassian » |
| Trello | « Clé d’API » et « Jeton » | « Obtenir une clé d’API », puis « Obtenir un jeton pour cette clé » (disponible une fois la clé saisie : un jeton en lecture et écriture, sans expiration) |
| GitHub | « Jeton », facultatif | « Créer un jeton GitHub ». Laissé vide, Escouade utilise celui de la CLI `gh` (`gh auth login`) ; le compte s’affiche alors avec « · via gh » |
| Linear | « Clé d’API » | « Créer une clé d’API Linear » |
| GitLab | Rien de plus : le compte de l’hébergeur GitLab (gitlab.com ou ton instance, jeton `api` ou celui de `glab`) | « Connecter GitLab… » ouvre le formulaire de Réglages › Intégrations › « Hébergeurs de code ». Un seul compte sert aux merge requests et aux tickets |
| ClickUp | « Clé d’API » (`pk_…`) | « Créer une clé d’API ClickUp » (Réglages › Apps de ClickUp). La ligne dit alors « Connecté · » suivi de ton nom chez ClickUp |
| Azure Boards | Rien de plus : le compte de l’hébergeur Azure DevOps (une organisation de dev.azure.com ou une collection d’Azure DevOps Server ; jeton personnel avec le droit Work Items en lecture et écriture, ou celui de `az`) | « Connecter Azure DevOps… » ouvre le formulaire de Réglages › Intégrations › « Hébergeurs de code ». Un seul compte sert aux pull requests et aux éléments de travail. Sans le droit Work Items, Azure refuse et l’erreur dit où créer un autre jeton (la page des jetons de l’organisation) |

### Hébergeur de code ou source de tickets

Un hébergeur de code (GitHub, GitLab, Bitbucket, Gitea, Azure DevOps) sert aux pull requests, à la CI et aux déploiements des agents ; une source du Kanban sert aux tickets. Réglages › Intégrations les montre en deux parties nommées. Connecter son GitLab comme hébergeur ne met aucun ticket dans le Kanban : il faut encore lier un projet GitLab comme source (voir plus bas), ce que la fenêtre d’import propose d’un clic. Bitbucket et Gitea n’ont pas encore de source : leur ligne dit « Pas de tickets : PR et CI seulement ».

« Déconnecter » oublie le compte et ses secrets. Les projets gardent leurs liens, inutilisés jusqu’à une nouvelle connexion.

### Où sont gardés les jetons

Les jetons restent sur ta machine, à part des réglages, et ne servent qu’aux appels de ces services. Escouade les range dans le trousseau du système : le Gestionnaire d’identification sous Windows, le Trousseau d’accès sous macOS. Sous Linux, qui n’a pas de trousseau pris en charge, ils restent dans `~/.escouade/integrations.json`, lisible par toi seul.

Quand le trousseau refuse un jeton sous Windows ou macOS, il reste dans ce fichier et l’onglet le dit : « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. » Si le trousseau existe mais ne peut pas être lu, l’onglet dit « Trousseau du système illisible : relance Escouade ou reconnecte le compte. » et aucun appel ne part sans les jetons. Pour GitHub par `gh`, Escouade ne garde aucun jeton.

### Les services servis par un plugin

GitHub (l’hébergeur et GitHub Issues), GitLab (l’hébergeur ; GitLab Issues fait encore partie de l’app), Bitbucket, Gitea et Forgejo, et Trello sont servis par des plugins livrés avec Escouade : rien ne change à l’usage, les comptes et les formulaires sont les mêmes. Un compte servi par le plugin d’un autre auteur le dit : « Servi par le plugin … ». Le jeton reste au trousseau ; Escouade le prête au seul plugin de ce service, quand il en a besoin.

Ces plugins se coupent dans la vue Plugins, pour toute l’app ou pour un projet (voir [les plugins livrés avec Escouade](plugins/installer#les-plugins-livres-avec-escouade)). Coupé ou arrêté, un plugin le dit au lieu de se taire :

- **un hébergeur** : la carte de l’agent porte « Hébergeur arrêté : PR plus suivies » et garde les PR lues en dernier ; Réglages › Intégrations dit sous le compte « Le plugin GitHub est coupé (Réglages › Plugins) : ce compte ne sert plus rien, ses PR ne sont plus suivies », ou qu’il s’est arrêté et pourquoi ;
- **une source** : la synchro s’arrête, et son échec dit « Le plugin dev.escouade.trello ne répond pas : … » ; les tickets importés restent.

Rallumé, le plugin reprend là où il s’était arrêté : la première lecture qui marche efface le message.

## Lier un projet à une source

Un projet se lie à un endroit par service : un projet Jira, un tableau Trello, un dépôt GitHub, une équipe Linear, une liste ClickUp, un projet GitLab ou un projet Azure DevOps. Dans Réglages › Intégrations, le groupe « Sources liées à `<projet>` » propose, pour chaque service connecté, une liste : « Projet Jira », « Tableau Trello », « Dépôt GitHub Issues », « Équipe Linear », « Liste ClickUp », « Projet GitLab Issues », « Projet Azure Boards ». « Aucun » délie le service. Pour GitHub, le dépôt du projet, quand son remote est sur GitHub, vient en premier (« (dépôt du projet) ») ; de même pour GitLab, quand le remote est sur une instance connectée, et pour Azure Boards, quand il est sur l’organisation connectée. Les projets Azure DevOps sont rangés par organisation. Les projets GitLab sont rangés par instance, avec leurs groupes et sous-groupes dans le chemin (« acme/back/api »). Pour ClickUp, les listes de tous tes espaces de travail sont rangées par espace et par dossier (« Produit › Sprints »), comme dans la barre latérale de ClickUp : les listes hors dossier d’abord, les archivées en moins ; taper les premières lettres d’un nom, menu ouvert, y va.

Choisir une source charge ses statuts et pré-remplit la correspondance des statuts (voir plus bas). Enregistre ensuite les réglages.

Depuis le Kanban, « Importer » sans source liée affiche « Aucune source liée à ce projet », nomme les sept sources et propose « Lier une source », qui ouvre ce même onglet sur « Sources du Kanban ». Quand le remote du projet est sur un hébergeur connecté dont la source existe (GitHub, GitLab, Azure DevOps), elle propose directement « Lier acme/api (GitLab Issues) » : un clic lie le projet avec les statuts par défaut, et ses tickets s’affichent aussitôt.

## Importer des tickets

Dans le [Kanban](board), « Importer » (⤓) ouvre la fenêtre « Importer des tickets ». Elle verse les tickets choisis à la fin de « À faire ».

- Un onglet par source liée et connectée (le service, le nom de la source, et le nombre de tickets que tu y as choisis). « ⚙ Gérer les sources » ouvre les réglages.
- Le champ de recherche cherche dans la source choisie. La recherche part 300 ms après la dernière frappe.
- Les puces de filtre se cliquent pour s’activer ou se retirer. Le champ et les puces dépendent du service :

| Service | Champ de recherche | Puces |
|---|---|---|
| Jira | « Rechercher par clé ou par texte… » | « Assignés à moi », « Sprint actif », « À faire » |
| Trello | « Rechercher une carte… » | « Mes cartes », puis une puce par liste du tableau |
| GitHub | « Rechercher une issue… » (un numéro, `42` ou `#42`, trouve l’issue) | « Assignées à moi », puis les étiquettes du dépôt (douze au plus) |
| Linear | « Rechercher par identifiant (ENG-42) ou par titre… » | « Mes issues », puis une puce par état du workflow qui n’est ni terminé ni annulé |
| GitLab | « Rechercher une issue (#12) ou par texte… » (un numéro, `12` ou `#12`, trouve l’issue ; un texte cherche dans le titre et la description) | « Assignées à moi », puis les étiquettes du projet (douze au plus) |
| Azure Boards | « Rechercher un élément de travail (#12) ou par titre… » (un numéro, `12` ou `#12`, trouve l’élément ; un texte cherche dans le titre) | « Assignés à moi », puis une puce par type d’élément de travail du projet (User Story, Bug, Task, Product Backlog Item, Issue… selon son processus), puis ses tags (douze au plus) |
| ClickUp | « Rechercher par id (#86abx3, ABC-12) ou par titre… » (l’id d’une tâche, avec ou sans `#`, ou son id personnalisé quand l’espace en a, trouve la tâche si elle est dans la liste) | « Mes tâches », puis une puce par statut de la liste qui n’est ni fini ni fermé |

Seuls les tickets ouverts sont proposés : pas ceux de la catégorie « Done » de Jira, ni les issues fermées de GitHub (les pull requests sont laissées de côté), ni les issues terminées ou annulées de Linear, ni les issues fermées de GitLab, ni les tâches de ClickUp dont le statut est de type fini ou fermé, ni les éléments de travail d’Azure Boards dans un état terminé ou supprimé (catégories Completed et Removed de leur processus) ni ceux des types qui ne sont pas d’un backlog (tests, revues, retours). Pour Azure Boards, une requête WIQL donne les éléments qui correspondent (1 000 au plus, les derniers modifiés d’abord), lus ensuite 50 à la fois. ClickUp n’ayant pas de recherche par titre, Escouade lit la liste 100 tâches à la fois et garde celles dont le nom contient le texte, jusqu’à 5 pages avant de montrer ce qu’elle a trouvé ; « Afficher plus » continue.

Chaque ligne montre la clé (pour ClickUp, l’id personnalisé, sinon `#` et l’id de la tâche), le titre, le type (Bug, Story, Tâche, Carte, Issue, Incident, User Story…), la priorité, la personne assignée et l’état, et « ✓ 2 critères détectés » (au survol : lesquels). Un ticket déjà dans le Kanban est grisé (« Déjà dans le Kanban ») et ne se choisit pas. Les tickets arrivent par pages ; « Afficher plus » charge la suivante, et la ligne au-dessus de la liste dit « 50 affichés sur 312 » quand le service compte plus de tickets qu’il n’en a envoyé.

Pour importer :

1. Coche les tickets (un clic, ou `Espace` ou `Entrée` sur une ligne), ou « Tout sélectionner » pour ceux de la liste qui ne sont pas déjà importés. La sélection se garde d’une recherche, d’une page et d’une source à l’autre.
2. Choisis « Boucles max » (3, 5 ou 8) : le même nombre pour tous les tickets importés.
3. Clique sur « Importer 3 tickets ».

Chaque ticket importé a sa propre clé Escouade (`DEM-7`) et garde, sur sa carte, la pastille de son service avec sa clé chez lui (`ATL-1294`) : un clic l’ouvre dans le service. Sa description reprend celle du ticket externe, et se termine par une ligne « Ticket Jira ATL-1294 : `<adresse>` ». Un message confirme (« 3 tickets importés depuis Jira et Trello »). Avec le pilote auto allumé, les tickets importés partent d’eux-mêmes (voir [Le pilote auto](autopilot)).

### Les critères d’acceptation

Avec « Extraire les critères d’acceptation » activé (par défaut), Escouade cherche les critères dans le ticket :

- Jira et GitHub : les éléments de liste sous un titre ou une ligne qui les nomme (« Critères d’acceptation », « Acceptance criteria », « Definition of done », « AC », « DoD »), sinon les cases à cocher (`- [ ]`) de la description ;
- Trello : les éléments de la checklist qui porte ce nom, sinon ceux de toutes les checklists, sinon ceux que la description liste ;
- Linear et GitLab : comme pour Jira et GitHub, dans la description ;
- ClickUp : comme pour Trello, les éléments de la checklist qui porte ce nom, sinon ceux de toutes les checklists de la tâche, sinon ceux que sa description (en Markdown) liste ;
- Azure Boards : le champ Acceptance Criteria quand il est rempli (ses éléments de liste, sinon ses lignes), sinon la description comme pour Jira (pour un bug, ses étapes de reproduction quand la description est vide). Les champs d’Azure Boards sont en HTML : Escouade les convertit en Markdown.

Il garde vingt critères au plus, de 300 caractères au plus. Sans critère trouvé, ou avec le réglage éteint, le ticket reçoit les deux critères par défaut. La description Jira est convertie en Markdown avant d’être cherchée.

## Importer par étiquette

Réglages › Intégrations › « Import automatique » laisse Escouade importer toute seule :

| Réglage | Rôle | Par défaut |
|---|---|---|
| « Importer les tickets étiquetés » | Les tickets ouverts des sources liées qui portent l’étiquette arrivent dans « À faire » | Éteint |
| « Étiquette » | Le label Jira, l’étiquette Trello, GitHub, Linear, ClickUp ou GitLab (un tag de tâche) | `claude-ready` |
| « Vérifier toutes les » | L’intervalle entre deux passes : 5, 15 ou 60 min | 15 min |

Une passe regarde, dans chaque projet, les sources liées dont le compte est connecté. Les tickets arrivent avec 5 boucles maximum, et un message l’annonce (« 3 tickets importés depuis Jira dans demo »). Un ticket que le projet a déjà importé n’est jamais ramené, même si tu l’as supprimé du Kanban depuis. La première passe a lieu peu après l’activation.

## Le statut et les commentaires suivent le ticket

Quand un ticket importé change de colonne, Escouade met à jour le ticket externe. Les liens, la correspondance des statuts et les commentaires se règlent par projet ; les trois réglages du groupe « Synchronisation » valent pour tous les projets.

### Correspondance des statuts

Le groupe « Correspondance des statuts » (une fois une source liée) a une ligne par colonne du Kanban et, pour chaque service lié, une liste de statuts. « — inchangé » laisse le ticket externe tel qu’il est. Choisir une source pré-remplit les colonnes ainsi :

- « En cours » et « À tester » : le statut qui ressemble à « en cours » (`In Progress`, `En cours`, `Doing`, `WIP`, `In development`, `Started`…) ;
- « Terminé » : le statut qui ressemble à « à tester » (`To test`, `Testing`, `QA`, `In Review`, `Review`, `Recette`, `À valider`, `Validation`, `Test`…) ;
- « À faire » : inchangé.

Pour ClickUp, chaque liste a ses propres statuts : la correspondance se règle donc par projet, sur ceux de la liste liée. Quand leurs noms ne disent rien de ce qui précède, leur type décide : le premier statut personnalisé (entre l’ouvert et les fermés) pour « En cours » et « À tester », le suivant pour « Terminé », sinon le statut de type fini. Une liste qui n’a qu’un statut ouvert et un fermé ne déplace rien par défaut. Si la liste n’a plus le statut choisi (renommé, retiré), la carte le dit (« La liste « Sprint 12 » de ClickUp n’a pas le statut « review » : ABC-12 reste où il est ; choisis un autre statut dans Réglages › Intégrations ») au lieu d’échouer sans un mot.

Ce que « statut » veut dire dépend du service : une transition de workflow pour Jira, la liste où la carte se range pour Trello, un état du workflow pour Linear, un statut de la liste pour ClickUp. Pour GitHub, ce sont « Ouverte », « Fermée » ou une étiquette du dépôt (marquée ◆) : « Fermée » ferme l’issue, une étiquette la rouvre si besoin et retire les autres étiquettes de la correspondance. GitLab fait de même, avec en tête les étiquettes des listes des tableaux du projet (« Doing », « To review »…), puis ses autres étiquettes ; une issue change d’étiquettes et d’état en un seul appel, et le commentaire est une note. Pour Azure Boards, ce sont les états des types d’éléments de travail du projet, selon son processus (Basic : To Do, Doing, Done ; Agile : New, Active, Resolved, Closed ; Scrum, CMMI ou un processus hérité : les siens), rangés par catégorie ; « En cours » et « À tester » prennent par défaut un état en cours (Active, Doing, In Progress…), « Terminé » l’état résolu quand le processus en a un. L’état part en un seul appel (`System.State`, en JSON Patch) ; quand le type de l’élément n’a pas cet état (« Committed » d’un Product Backlog Item pour une Task), il prend l’état de son type de la même catégorie (« In Progress »). Le commentaire s’écrit en Markdown dans la discussion de l’élément.

La dernière colonne, « Commenter », choisit les colonnes dont l’arrivée écrit un commentaire (par défaut « À tester » et « Terminé »). Le commentaire dit où en est le ticket :

| Colonne | Commentaire |
|---|---|
| « À faire » | « Escouade : DEM-5 est revenu « À faire ». » |
| « En cours » | « Escouade : DEM-5 est pris par un agent (branche ticket/dem-5). » |
| « À tester » | « Escouade : DEM-5 est prêt à tester. » (avec « (objectif partiel) » le cas échéant), les critères (✓ ou ○, avec la note de l’agent) et « Ce qui a été fait ». |
| « Terminé » | « Escouade : DEM-5 est terminé — » suivi de l’issue (« ⤵ Mergé dans main · squash »), et du lien de la pull request s’il y en a une. |

### Les réglages de synchronisation

| Réglage | Rôle | Par défaut |
|---|---|---|
| « Mettre à jour le statut externe » | Quand un ticket importé change de colonne, son ticket externe prend le statut de la correspondance | Activé |
| « Publier un résumé à chaque boucle » | À chaque nouvelle boucle, un commentaire donne les critères atteints et les notes de l’agent (« Escouade : DEM-5, boucle 2/5 — 1/3 critères atteints. ») | Éteint |
| « Extraire les critères d’acceptation » | Voir plus haut | Activé |

### La langue des commentaires

Les commentaires sont publiés pour ton équipe : ils sont écrits dans la « Langue des textes rédigés par les agents » (Réglages › Application, voir [Réglages de l’application](settings-application)), pas dans celle de l’interface. L’issue d’un ticket terminé y est citée dans cette même langue.

## Synchronisations qui échouent

Chaque changement à répercuter est gardé dans une file, `~/.escouade/sync-queue.json`, jusqu’à ce que le service l’ait pris. Les changements d’un ticket partent dans l’ordre, l’un après l’autre : un changement qui échoue retient les suivants du même ticket, pas ceux des autres tickets. Une transition de statut remplace celle qui n’est pas encore partie pour le même ticket, et un ticket qui revient dans une colonne précédente annule la transition qui n’était pas partie.

Une carte dont la synchronisation a échoué affiche un ⚠. Un clic dessus montre la raison (« Jira refuse ces identifiants (401) », « Jira : aucune transition de « En cours » vers « Done » pour ATL-1294 », « La liste ClickUp 901 n’existe plus ou n’est plus accessible : choisis-en une autre dans Réglages › Intégrations »…) et le bouton « Resynchroniser ».

- Quand le service est injoignable, en panne ou trop sollicité, ou qu’il refuse les identifiants (ce qui se répare), l’échec peut passer plus tard : Escouade réessaie seule, après 1, 5, 15 puis 60 minutes, la dernière attente étant répétée, pendant 24 heures à partir du premier échec.
- Un refus qu’un nouvel essai ne changerait pas (requête refusée telle quelle, ticket disparu, aucune transition vers le statut) est mis de côté tout de suite, comme tout ce qui échoue encore au bout d’un jour. Il ne retient plus les changements suivants du ticket.
- Quand ClickUp limite les requêtes (429), Escouade attend la fin de la limite qu’il annonce (`X-RateLimit-Reset`, une minute au plus) et redemande une fois avant de compter l’échec.
- Au démarrage d’Escouade, tout ce qui attendait repart (sauf ce qui a été mis de côté).
- « Resynchroniser » renvoie tout de suite, mis de côté compris. Quand il ne reste rien à envoyer, il redonne au ticket externe le statut de sa colonne (sans réécrire de commentaire). Le ⚠ disparaît dès que tout est passé.

Supprimer le ticket du Kanban abandonne ce qui attendait pour lui.

## Voir aussi

- [Le Kanban](board) : la carte d’un ticket importé.
- [Réglages des intégrations](settings-integrations) : la liste complète des réglages.
- [Le serveur MCP](mcp) : le ticket externe d’un ticket se lit avec `get_ticket`.
