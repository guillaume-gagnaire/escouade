---
title: "Intégrations"
subject: "Kanban"
sources: ["docs/SPEC.md"]
tickets: ["ESC-165","ESC-201","ESC-242"]
decisions: []
compared: null
---

## Ce que c'est

Les intégrations relient le Kanban d'un projet à un outil de tickets : Jira Cloud, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards. On en importe des tickets, à la main ou par une étiquette. Escouade tient ensuite le ticket d'origine à jour : son état suit la colonne, et des commentaires disent où en est le travail.

## Comportement

### Réglages › Intégrations

- L'onglet a deux parties nommées, chacune avec sa ligne :
  - « Hébergeurs de code » : « PR et MR des agents, CI et déploiements : un compte par instance, que chaque projet trouve par son remote. » (voir *Hébergeurs de code*) ;
  - « Sources du Kanban » : « Import et synchro des tickets : un ticket importé suit sa colonne là-bas (état, commentaires). ». Sous elle : les comptes des sources, les sources liées, la correspondance, la synchro et l'import automatique.
- Une instance d'hébergeur dit sur sa ligne si elle sert aussi de source (« Sert aussi à GitHub Issues », « Sert aussi à GitLab Issues », « Sert aussi à Azure Boards ») ou non (« Pas de tickets : PR et CI seulement » : Bitbucket, Gitea / Forgejo, GitHub Enterprise).
- La fenêtre d'import mène à « Sources du Kanban », pas en haut de l'onglet.

### Les comptes

Les comptes se connectent tout de suite, hors du brouillon « Enregistrer ».

| Source | Compte |
|---|---|
| Jira Cloud | site en https (http seulement vers cette machine), e-mail, jeton d'API |
| Trello | clé d'API et jeton ; « Obtenir un jeton pour cette clé » ouvre la page d'autorisation |
| GitHub Issues | un jeton, ou vide : celui de `gh auth token` (« · via gh ») ; c'est le compte de l'hébergeur github.com (voir *Hébergeurs de code*) |
| Linear | une clé d'API |
| ClickUp | une clé d'API personnelle (`pk_…`) ; le compte se dit par le nom de l'utilisateur ClickUp |
| GitLab Issues | pas de compte à lui : ceux de l'hébergeur GitLab (ci-dessous) |
| Azure Boards | pas de compte à lui : ceux de l'hébergeur Azure DevOps (ci-dessous) |

- « Connecter » vérifie le compte auprès du service. Un refus s'affiche sous le formulaire : « Jira refuse ces identifiants (401) — … ».
- Les secrets (jetons, clés) vivent dans le trousseau du système : Gestionnaire d'identification sous Windows, Trousseau sous macOS ; service `dev.gagnaire.escouade`, une entrée par intégration. Le reste des comptes est dans `~/.escouade/integrations.json`, à part des réglages. Les secrets ne repassent jamais par la fenêtre, qui ne reçoit que « Connecté · <compte> », ni par le journal.
- « Connecter… » écrit dans le trousseau et relit avant d'enregistrer le fichier sans eux. « Déconnecter » les y efface. GitHub via `gh` n'y laisse rien ; l'ancien jeton d'un compte passé à `gh` est effacé.
- Un effacement refusé est noté dans le fichier (les noms des services, jamais un secret) et retenté au démarrage suivant, sauf si le service a de nouveau un compte avec son propre jeton. « Déconnecter » le dit dans un toast quand le jeton était dans le trousseau : « Le jeton <service> n'a pas pu être retiré du trousseau du système : Escouade réessaiera au prochain démarrage, ou retire-le à la main (son nom contient « escouade »). ». Un service que cette version ne connaît pas, noté par une plus récente, est ignoré au lieu de rendre le fichier illisible.
- Aucune autre entrée du trousseau n'est jamais effacée, même d'un service qu'aucun compte ne cite : elle peut être la seule copie d'un jeton.
- Au chargement, un secret trouvé dans le fichier (version d'avant 1.6, ou refusé la fois précédente) est écrit dans le trousseau, relu pour vérifier, et alors seulement retiré du fichier, réécrit d'un coup.
- Trousseau indisponible, ou absent hors Windows et macOS : le secret reste dans le fichier (droits 0600 sous Unix), l'onglet le dit sous le compte (« Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. »), et le démarrage suivant retente.
- Trousseau qui ne rend pas les secrets d'un compte au démarrage (hors d'atteinte, refus, entrée effacée ou illisible) : rien n'est réécrit ni effacé. Le compte reste connecté, mais aucun appel ne part sans ses secrets ; GitHub ne prend pas le jeton de `gh` à la place. L'appel échoue avec « Trousseau du système illisible : relance Escouade ou reconnecte le compte <service>. », et l'onglet dit sous le compte « Trousseau du système illisible : relance Escouade ou reconnecte le compte. ».
- Une exécution isolée (`ESCOUADE_DATA_DIR` : tests e2e, démo) range ses secrets sous `dev.gagnaire.escouade.sandbox`, jamais à la place de ceux de l'utilisateur. Une build de développement sur le dossier de l'utilisateur les écrit dans le trousseau mais les laisse aussi dans le fichier : l'app installée, peut-être plus ancienne, les y lit.
- Les appels passent par le proxy des réglages. `ESCOUADE_TRELLO_API`, `ESCOUADE_GITHUB_API` et `ESCOUADE_CLICKUP_API` remplacent les adresses des API (tests, démo).

### Les sources liées

- Par projet, pour chaque compte connecté : un projet Jira, un tableau Trello, un dépôt GitHub (celui du remote `origin` en tête, « (dépôt du projet) »), une liste ClickUp, un projet GitLab ou un projet Azure Boards ; ou « Aucun ».
- Choisir une source pré-remplit la correspondance : « En cours » et « À tester » vers l'état qui ressemble à « en cours », « Terminé » vers celui qui ressemble à « à tester », « À faire » inchangé.

### La correspondance des statuts

- Par colonne du Kanban et par source : l'état externe donné au ticket qui arrive dans la colonne, ou « — inchangé ». Jira : un statut, par sa transition. Trello : une liste. GitHub : « Ouverte », « Fermée », ou une étiquette « ◆ … » qui remplace les autres étiquettes de la correspondance.
- Par colonne, « Commenter » (par défaut : « À tester » et « Terminé ») publie un commentaire : pris par un agent (sa branche), prêt à tester (critères ✓ / ○, ce qui a été fait), terminé (issue de la validation et son lien), revenu « À faire ».
- Ces commentaires, comme le résumé de chaque boucle, sont écrits dans la langue des textes rédigés (voir *Langues*).

### La synchronisation

- Une synchro à la fois, dans l'ordre des changements. Un échec est journalisé et noté sur la carte (⚠ et sa raison), sans bloquer le ticket.
- L'opération ratée (transition, commentaire) est gardée dans `sync-queue.json`, dans le dossier de données, sans aucun jeton. Une opération que cette version ne connaît pas, laissée par une plus récente, est écartée seule, pas toute la file.
- Un échec qui peut passer plus tard (service injoignable ou trop lent, erreur 5xx, 429, identifiants refusés 401 / 403) est retenté 1, 5, 15 puis 60 min après chaque échec, puis toutes les heures, 24 h au plus après le premier échec ; aussi au démarrage de l'app et au changement suivant du ticket. Les opérations d'un ticket gardent leur ordre : celles qui suivent un échec l'attendent. Les autres tickets continuent.
- Un refus que redemander ne changerait pas (400, 404, 410, 422, Jira sans transition vers l'état) est mis de côté tout de suite, comme un échec au bout de 24 h. Il n'est plus retenté seul ni au démarrage, ne retient plus les opérations suivantes du ticket, et reste gardé pour « Resynchroniser ».
- Une transition remplace celle du même ticket pas encore passée, mise de côté ou non.
- Un retour en arrière l'emporte aussi quand la colonne où revient le ticket laisse l'état « — inchangé » (« À faire » compte toujours comme un retour, sinon une colonne avant celle de la transition) : envoyée plus tard, la transition mettrait le ticket externe là où le ticket n'est plus. Ce qui l'attendait part aussitôt.
- Passé à une colonne suivante laissée « — inchangé » (« À tester », « Terminé »), le ticket garde la transition : le ticket externe y serait si le service avait répondu.
- Les opérations d'un ticket supprimé sont oubliées.
- Le ⚠ reste tant qu'une opération du ticket attend ou est de côté. Il se déplie : la raison, et « Resynchroniser », qui retente tout de suite celles qui attendent et celles de côté. S'il n'en reste aucune (un ⚠ laissé par une version précédente), il redonne l'état de sa colonne. Un échec le dit dans un toast. Tout passé, le ⚠ disparaît.
- Un commentaire peut être publié deux fois : quand son envoi expire, le service l'a peut-être reçu, et il est renvoyé.

### L'import automatique

- Premier passage 30 s après le démarrage, puis à l'intervalle réglé. Les tickets ouverts des sources liées qui portent l'étiquette (la première page de chaque source) arrivent dans « À faire » avec 5 boucles max, et un toast : « 2 tickets importés depuis Jira dans <projet> ».
- Un ticket déjà importé dans le projet, même supprimé depuis, n'est jamais ramené.

### « ⤓ Importer »

- Dans l'en-tête du Kanban ; l'icône seule dans une fenêtre étroite.
- Sans source liée : « Aucune source liée à ce projet » et les sept sources nommées. Quand l'hébergeur du remote du projet est connecté et que sa source n'est pas encore liée : « Lier <dépôt> (GitHub Issues) », « Lier <projet> (GitLab Issues) » ou « Lier <projet> (Azure Boards) », liée d'un clic avec les états par défaut, et la liste s'ouvre aussitôt. « Lier une source » (« Lier une autre source » à côté d'une proposition) ouvre Réglages › Intégrations sur « Sources du Kanban ».
- Avec des sources : une pastille par source (avec le nombre de tickets cochés), « ⚙ Gérer les sources », une recherche (lancée 300 ms après la frappe), les filtres du service, « Tout sélectionner ».
  - Recherche : Jira par texte ou par clé, GitHub par sa recherche, Trello en filtrant sur place.
  - Filtres : Jira « Assignés à moi », « Sprint actif », « À faire » ; Trello « Mes cartes », un par liste ; GitHub « Assignées à moi », un par étiquette.
- Une ligne par ticket : clé externe, titre, type, priorité · assigné · statut, « ✓ n critères détectés ». Un ticket « Déjà dans le Kanban » est grisé.
- Par pages : 50 pour Jira et la recherche GitHub, 100 pour la liste GitHub, tout le tableau d'un coup pour Trello. « Afficher plus » en bas quand le service en a d'autres. « 50 affichés sur 312 » dès que le service en compte plus que la liste n'en montre, dernière page comprise ; la recherche GitHub s'arrête à 1 000. Le total est le compte approximatif de Jira (jamais moins que la liste) ou celui de la recherche GitHub ; aucun quand une pastille d'étiquette GitHub trie sur place. « 50 affichés » quand il en reste sans total, « n résultats » quand tout est là. Une nouvelle recherche repart de la première page.
- Pied : boucles max 3 / 5 / 8, et « Importer n tickets », qui les met à la fin de « À faire », avec un toast « 3 tickets importés depuis Jira et Trello ».

### Le ticket importé

- Il a sa clé interne `<PRÉ>-<n>` et garde sa référence externe. Son agent et sa branche prennent la clé d'origine (voir *Agent d'un ticket*).
- Titre externe ; description (celle de Jira convertie en Markdown) suivie de « Ticket Jira ATL-1287 : <adresse> » ; critères extraits, sinon ceux par défaut.
- Il est importé une fois par projet : même service, même id. Pour Jira, l'id du ticket, qui reste quand il change de projet.
- La recherche GitHub d'un numéro (« #42 ») donne l'issue elle-même.
- Sa carte porte la pastille du service et la clé externe ; un clic ouvre le ticket externe dans le navigateur.

### GitLab Issues

- gitlab.com et les instances auto-hébergées.
- Il prend tous les comptes de l'hébergeur GitLab, un par instance, sans rien redemander (voir *Hébergeurs de code*). Sa ligne dans « Comptes connectés » dit « Connecté · @ada » (« @ada · git.acme.dev » pour une instance auto-hébergée ; plusieurs, séparés par des virgules) et « Compte de l'hébergeur GitLab : il se connecte et se déconnecte dans Hébergeurs de code. ». « Connecter GitLab… » ouvre le formulaire de l'hébergeur, type GitLab choisi. Pas de « Déconnecter » à part : connecter ou déconnecter GitLab Issues seul est refusé, en le disant.
- La source d'un projet est un projet GitLab, `<hôte>/<groupes>/<projet>`. Le menu « Projet GitLab Issues » range par instance les projets dont le compte est membre (100, les plus actifs, archivés exclus), le projet du remote en tête « (dépôt du projet) » quand son instance a un compte. Les groupes ne sont pas des sources : les issues vivent dans les projets, et un groupe mêlerait plusieurs dépôts.
- États : « Ouverte », « Fermée », puis les étiquettes des listes des tableaux du projet, par position (aucune si l'instance ne les donne pas), puis ses autres étiquettes. Une étiquette « ◆ … » remplace les autres étiquettes de la correspondance, en une seule écriture qui rouvre l'issue au besoin.
- Import : les issues ouvertes, 50 par page, avec le total ; « Assignées à moi » ; l'étiquette de l'import automatique ; une puce par étiquette (12 au plus). « #12 » donne l'issue elle-même, ouverte seulement.
- Ligne : `#iid`, titre, type (Issue, Incident en rouge, Tâche), étiquettes · assignée · « par @auteur ». Description en Markdown.
- Commentaire : une note sur l'issue.
- Erreurs : le message de GitLab est cité ; 401 et 429 sont retentés, 404 est durable. Le compte d'une instance illisible au trousseau, ou dont `glab` ne répond pas, est laissé de côté tant qu'une autre répond.

### Azure Boards

- dev.azure.com et Azure DevOps Server.
- Il prend les comptes de l'hébergeur Azure DevOps, un par organisation ou collection, sans rien redemander (voir *Hébergeurs de code*). Sa ligne dans « Comptes connectés » dit « Connecté · ada@acme.dev · acme » (le compte, puis l'organisation ; l'hôte pour un Server), « Compte de l'hébergeur Azure DevOps : il se connecte et se déconnecte dans Hébergeurs de code. » et « Son jeton doit avoir le droit Work Items (lecture et écriture). ». « Connecter Azure DevOps… » ouvre le formulaire de l'hébergeur, type Azure DevOps choisi ; connecter ou déconnecter Azure Boards seul est refusé, en le disant.
- Un 401 ou un 403 sur les éléments de travail dit qu'il faut au jeton le droit Work Items, avec l'adresse de la page des jetons de l'organisation (`<organisation>/_usersSettings/tokens`). L'erreur est retentée.
- La source d'un projet est un projet Azure DevOps, `<org>/<projet>`. Le menu « Projet Azure Boards » range sous chaque organisation ses projets, le projet du remote en tête « (dépôt du projet) » quand son organisation a un compte.
- États : ceux des types d'éléments de travail du projet, sans les types désactivés ni ceux qui ne sont pas d'un backlog (Test Case, Test Plan, Test Suite, Shared Steps, Shared Parameter, Code Review, Feedback). Ils sont rangés par catégorie (Proposed, InProgress, Resolved, Completed ; Removed laissé de côté), chacun une fois : Basic (To Do, Doing, Done), Agile (New, Active, Resolved, Closed), Scrum (New, Approved, Committed, Done ; To Do, In Progress pour Task), CMMI, ou un processus hérité.
- Par défaut : « En cours » et « À tester » vers l'état en cours nommé comme tel, sinon le premier de la catégorie InProgress ; « Terminé » vers le premier état Resolved, quand il y en a un.
- État poussé : rien si l'élément y est déjà. Un état que le type de l'élément n'a pas devient l'état de son type de la même catégorie (« Committed » devient « In Progress » pour une Task Scrum). Un refus d'Azure (une règle du processus) est durable.
- Import : les éléments du projet des types du backlog (ou ceux des puces de type), hors des états Completed et Removed du processus, du plus récemment changé au plus ancien, 1 000 au plus, lus 50 à la fois. Puces : « Assignés à moi », une par type, puis les tags du projet (12 au plus, aucun si Azure ne les donne pas), en OU entre eux ; l'import automatique prend le tag de son étiquette. « #12 » ou « 12 » donne l'élément ; un texte cherche dans les titres.
- Ligne : `#id`, titre, type tel qu'Azure le nomme, état · tags · assigné (« Non assigné ») · « par <créateur> » ; un lien vers l'élément.
- Description : celle de l'élément (sinon les étapes de reproduction d'un bug), convertie du HTML en Markdown. Critères : le champ Acceptance Criteria quand il est rempli (ses éléments de liste, sinon ses lignes), sinon ceux de la description.
- Commentaire : en Markdown, sur l'élément.
- PR liée : valider en « PR » un ticket venu d'Azure Boards, sur un dépôt Azure Repos, ouvre la PR par le compte de l'hébergeur (en brouillon selon le réglage), liée à l'élément de travail. Sur GitHub, « Fixes AB#<id> » termine la description ; l'app Azure Boards de GitHub le lit, quand le projet l'a. De même pour la PR qu'ouvre un agent ordinaire travaillant sur un tel ticket.

### ClickUp

- Le compte est une clé d'API personnelle (`pk_…`), au trousseau. « Connecter » la vérifie ; le compte se dit par le nom de l'utilisateur ClickUp, dont l'id sert à « Mes tâches ».
- La source d'un projet est une liste. Le menu « Liste ClickUp » range les listes de tous les espaces de travail (archivées exclues) en groupes « Espace › Dossier », l'espace de travail devant quand il y en a plusieurs, les listes hors dossier d'abord. Le menu se parcourt au clavier par le nom.
- Statuts : ceux de la liste, dans son ordre. Les défauts se choisissent par le nom, comme ailleurs, sinon par le type : le premier statut `custom` pour « En cours » et « À tester », le suivant (sinon celui de type `done`) pour « Terminé ». Une liste sans statut `custom` ne déplace rien.
- Avant d'écrire un statut, la liste est relue. Un statut qu'elle n'a plus est un refus durable, dit sur la carte (« La liste « Sprint 12 » de ClickUp n'a pas le statut « review » : ABC-12 reste où il est… ») ; rien n'est envoyé.
- Import : les tâches ouvertes et leurs sous-tâches, de la plus récemment changée à la plus ancienne, 100 par page ; « Mes tâches » ; une puce par statut ni `done` ni `closed` ; l'étiquette de l'import automatique cherchée en tag, en minuscules. Les tâches de type `done` ou `closed` sont écartées.
- Recherche : `#86abx3`, ou un mot de 6 caractères au moins mêlant lettres et chiffres, est un id de tâche ; `ABC-12` un id personnalisé, cherché dans chaque espace de travail. La tâche n'est gardée que si elle est dans la liste. Un id inconnu, ou un autre texte, est cherché dans les noms par Escouade, ClickUp n'ayant pas de recherche par titre : jusqu'à 5 pages lues avant de rendre ce qui est trouvé, et « Afficher plus » continue.
- Clé affichée : l'id personnalisé, sinon `#<id>`. Description en Markdown, images comprises. Critères : la checklist nommée « Critères d'acceptation » (ou un équivalent), sinon toutes les checklists, dans leur ordre, sinon la description.
- Commentaire : sur la tâche, sans notifier tout le monde.
- Erreurs : le message de ClickUp est cité. 401 : clé refusée. 429 : attente jusqu'à la remise à zéro annoncée (2 s sans annonce ; au-delà d'une minute, pas d'attente), puis une relance ; un second 429 se dit comme les autres. Liste disparue (404), ou refusée alors que la clé passe : « La liste ClickUp … n'existe plus ou n'est plus accessible : choisis-en une autre dans Réglages › Intégrations », durable.

## Réglages

Dans Réglages › Intégrations › « Sources du Kanban » :

- les comptes ; par projet, les sources liées et la correspondance des statuts ;
- pour toute l'app, la synchronisation : « Mettre à jour le statut externe » (oui par défaut), « Publier un résumé à chaque boucle » (non), « Extraire les critères d'acceptation » (oui : les éléments sous « Critères d'acceptation », « Acceptance criteria » ou « Definition of done », sinon les cases à cocher ; pour Trello, la checklist de ce nom, sinon toutes) ;
- l'import automatique : « Importer les tickets étiquetés » (non par défaut), « Étiquette » (`claude-ready`), « Vérifier toutes les » 5, 15 ou 60 min.

## Limites

- Bitbucket, Gitea / Forgejo et GitHub Enterprise ne servent pas de source de tickets.
- GitLab Issues : un groupe n'est pas une source, seulement un projet.
- Azure Boards : ni zone ni itération ; une itération courante demanderait une équipe.
- ClickUp : une clé personnelle seulement. L'OAuth de ClickUp (une app enregistrée, un jeton par espace de travail autorisé) n'est pas fait ; il servirait à une équipe qui ne veut pas de clés personnelles.
