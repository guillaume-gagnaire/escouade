---
title: "Pull requests"
subject: "Git & fichiers"
sources: ["docs/SPEC.md","docs/TODO-1.8.md","src/components/PrPanel.svelte","src-tauri/src/prview.rs"]
tickets: ["ESC-64","ESC-161","ESC-305","ESC-56","ESC-58","ESC-65","ESC-66","ESC-68","ESC-383"]
decisions: [15]
compared: "2026-10-30T18:20:19+01:00"
---

## Ce que c'est

Escouade ouvre la pull request d'un agent ordinaire et suit celles de tout agent : leur état, leur CI et leurs relectures, sur la carte de l'agent, dans l'en-tête de sa conversation, dans la barre d'état et dans l'onglet « PR et CI » du panneau latéral. Quand la CI échoue ou qu'une relecture demande des changements, un bouton envoie le tout à l'agent pour qu'il corrige. La PR d'un ticket est ouverte par sa validation (voir *Validation*) ; elle s'affiche et se lit comme les autres. Toutes les PR ouvertes du projet, les siennes ou d'autres, se listent à côté du Kanban, où l'on lance un agent sur l'une d'elles (voir *Vue « Pull requests » du projet*) ou la fait relire par un agent (voir *Relecture d'une PR par un agent*). Ce que chaque hébergeur permet est dans *Hébergeurs de code*.

## Comportement

### Quel agent peut ouvrir une PR

- Un agent ordinaire (pas celui d'un ticket), non archivé, qui a un worktree d'Escouade (sa branche), ou qui travaille dans un autre worktree du dépôt : créé par Claude Code ou rejoint par un `cd`. Ce dossier est celui de la dernière entrée de sa session Claude Code, relu à chaque lecture de ses PR. Sa racine doit être un worktree du projet (`git worktree list`), ni le dossier du projet ni le worktree d'un autre agent, et sur une branche. L'en-tête de la conversation dit alors cette branche, et sa PR vise la cible du Kanban, sinon la branche du dossier du projet. Changer de branche ou revenir au dossier du projet oublie les PR de l'ancienne.
- « Ouvrir une PR » (« Ouvrir une MR » sur GitLab) n'est offert que si l'hébergeur du projet en ouvre : le projet a un dépôt distant chez GitHub, GitLab, Bitbucket Cloud, Gitea / Forgejo ou Azure DevOps, et un compte y est connecté. Le bouton reste quand l'hébergeur ne répond pas. Sans dépôt distant, chez un autre hébergeur, ou sans compte : ni bouton ni entrée, aucune erreur (l'onglet « PR et CI » dit pourquoi).

### Où

- Le bouton « Ouvrir une PR » de l'en-tête de la conversation, tant qu'aucune PR de l'agent n'est ouverte ; « Ouvrir une PR… » du menu de sa carte, du menu de sa branche dans le sélecteur de branche (le sélecteur se ferme d'abord) et de l'onglet « PR et CI » quand sa branche n'en a pas.
- « Voir la PR #n » à leur place tant qu'une est ouverte : elle s'ouvre dans le navigateur.

### La fenêtre « Ouvrir une PR sur <hébergeur> »

- Elle dit « <agent> · <branche> → <base> ». Titre et description sont proposés par le modèle des textes rédigés, dans leur langue (voir *Langues*), d'après les commits de la branche depuis sa base et son diff (40 Ko au plus). Sans réponse : le dernier sujet de commit et la liste des commits, et la fenêtre le dit.
- Champs modifiables ; « Branche cible » (la base de son worktree, sinon la cible du Kanban) ; « En brouillon » quand l'hébergeur en fait ; le nombre de fichiers non commités, qui n'y seront pas ; « Proposer à nouveau ».
- Refusé quand la branche n'a aucun commit de plus que sa base, ou qu'une PR est déjà ouverte.
- Rien ne part avant « Pousser et ouvrir » : la branche est poussée (`git push -u`), puis la PR ouverte. Le refus de l'hébergeur reste dans la fenêtre ; ouverte, un toast dit « PR #n ouverte. ».
- Une branche qui a une PR n'est plus renommée au nom de l'agent.

### Quelles PR sont celles d'un agent

- Celles de la branche où il est réellement, quel qu'en soit l'auteur : celle de son worktree (agent de ticket compris, en lecture seule : ni « Ouvrir une PR » ni correction, son Kanban la suit), celle du worktree où il est entré seul, sinon celle du dossier du projet. Pas celle du dossier quand il est sur la cible du Kanban ou la branche par défaut d'un dépôt distant (`main` ou `master` faute de mieux) : une PR `main → prod` irait sinon à tous les agents du dossier.
- Elles sont cherchées sur le dépôt de la cible (là où « Ouvrir une PR » ouvre), sur celui que suit la branche (sinon `origin`), et sur GitHub, GitLab, Gitea et Forgejo sur le dépôt dont ce dernier est un fork. Un dépôt autre que celui de la cible qui ne répond pas garde ce qui y a été lu en dernier ; un dépôt parent que l'hébergeur n'a pas dit est redemandé 10 min plus tard.
- Deux PR de dépôts différents peuvent avoir le même numéro : elles se distinguent par leur dépôt. La PR d'un fork est gardée quand sa branche de tête est celle de l'agent (une branche poussée sur ton fork, la PR ouverte vers l'original), écartée sinon (un autre fork qui a une branche du même nom). Lue ailleurs que sur le dépôt de la cible, elle garde son dépôt, où « L'agent corrige » et l'onglet « PR et CI » lisent ses checks et ses retours.
- Elles sont lues à l'affichage de l'agent, puis chaque minute tant qu'il est affiché, au plus toutes les 20 s, et plus du tout une fois toutes mergées. Elles sont gardées avec l'agent, dans l'ordre : ouvertes prêtes, brouillons, mergées, fermées, la plus récente d'abord. L'hébergeur dit aussi leur auteur, leur date de création et à qui une relecture est demandée, quand il les donne.

### Sur la carte et dans l'en-tête

- Trois numéros au plus (le groupe « Pull requests de <agent> » sous la carte, hors du bouton qu'elle est), précédés du signe de leur CI (✓ verte, ✗ en échec, ● en cours, ○ aucune, ⇡ mergée) et colorés par ce qui compte d'abord : échec ou conflit, puis changements demandés ou CI en cours, sinon vert ; brouillon et fermée grisés.
- Sur la carte, chaque numéro est suivi de sa cible (« #41 → main ») et, pour une PR ouverte qui a une CI, de son état en mots (« CI verte », « CI en échec », « CI en cours », « CI ignorée »), dans la couleur de son signe. Un clic sur ces mots choisit l'agent et ouvre le panneau latéral sur l'onglet « PR et CI » (côte à côte, le panneau est déjà là).
- Un clic sur un numéro ouvre la PR dans le navigateur. L'infobulle dit « PR #12 · titre », puis l'état, la CI (la dernière exécution d'un check relancé), les approbations, les changements demandés, les fils ouverts, le conflit.
- La même PR suit le bouton de branche de la barre d'état (voir *Branches*).

### L'onglet « PR et CI »

Quatrième onglet du panneau latéral (voir *Git & fichiers*), « PR et CI » (« MR et CI » sur GitLab : le mot de l'hébergeur de l'agent). Une pastille rouge (« La CI de la PR #n a échoué ») s'y ajoute quand la première PR de l'agent est ouverte et que sa CI a échoué. Sous 420 px de large, l'onglet quitte la bande pour le × (choix 189) : les mots de la CI sur la carte de l'agent l'ouvrent alors.

Pour l'agent ouvert :

- **Sa PR**, la première de la liste de ses PR : son signe, son numéro, son état (ouverte, brouillon, mergée, fermée), son titre, « <branche> → <cible> », « Conflit avec <cible> » quand l'hébergeur le dit, « créée il y a 2 h par <auteur> », et l'icône qui l'ouvre dans le navigateur. « Faire relire par un agent » y est offert quand elle est ouverte et sur le dépôt de la cible (voir *Relecture d'une PR par un agent*). « L'agent corrige » et « L'agent corrige les retours » y sont offerts aux mêmes conditions que sous la carte, sur un clic seulement. Ses autres PR sont listées en bas (« Ses autres PR »), chacune avec son état, sa CI, son âge et son auteur.
- **La CI** : un résumé (« 1 échec sur 11 », « 3 en cours sur 14 », « 10 vérifications réussies · 1 annulée »), puis chaque vérification du dernier commit (sur GitLab, chaque job du pipeline de tête), les échecs d'abord, puis en cours, en attente, annulées, réussies, ignorées. Chacune a son signe, son état (en attente, en cours, réussie, échouée, annulée, ignorée), sa durée (« 2m 30s », jusqu'à maintenant pendant qu'elle tourne, quand l'hébergeur dit son début) et l'icône vers sa page chez l'hébergeur. Le nom d'une vérification échouée dont l'hébergeur garde le journal s'ouvre sur « La fin du journal » : ses 20 derniers Ko coupés à une ligne, en monospace, dans un cadre qui défile. « Aucune vérification sur le dernier commit. » sinon.
- **« Relancer les échecs »** (GitHub, GitLab, Azure DevOps, Gitea de la 1.26), sur une PR ouverte dont une vérification a échoué ou été annulée : relance chez l'hébergeur les jobs en échec du dernier commit (sur GitLab, le pipeline de tête en un appel), au clic seulement. Un toast dit « N exécutions relancées. », ou « Aucun job à relancer : l'hébergeur ne relance que ses propres jobs. », puis la CI est relue.
- **La relecture** : la CI est relue toutes les 20 s tant qu'une vérification est en attente ou en cours, que l'onglet est affiché et que la fenêtre est visible ; jamais en arrière-plan. Les PR de l'agent sont relues à l'affichage de l'onglet (au plus toutes les 20 s) ; quand l'état de la CI d'une PR change, sa CI est relue aussitôt. Une lecture refaite qui échoue garde ce qui était affiché et dit pourquoi.
- **Les revues et commentaires**, en lecture : chaque relecteur qui approuve (« a approuvé ») ou demande des changements, avec son texte ; une relecture qui ne fait que commenter n'apparaît que si elle dit quelque chose. Puis les commentaires, les fils ouverts d'abord : auteur, « fichier:ligne » pour un commentaire de ligne, « résolu », le texte (6 lignes au plus). « En lecture : Escouade ne publie rien en ton nom. » : aucun champ, aucune réponse, aucune résolution (décision du 2026-10-16).
- **Les déploiements** de la branche, quand l'hébergeur les donne (GitHub, GitLab, Bitbucket Cloud ; pas pour la branche d'un fork ; ni Gitea ni Forgejo n'en ont) : l'environnement, son état (déployé, échoué, en erreur, en attente, en file, en cours, remplacé) et son adresse, ouverte dans le navigateur par un clic.
- Sans PR sur sa branche : « Pas de PR sur cette branche », puis « Ouvrir une PR… » quand l'agent peut en ouvrir une, sinon la raison (pas de branche à lui, agent de ticket, pas de dépôt distant, hébergeur sans client, pas de compte), avec « Réglages › Services connectés » quand c'est là que ça se règle (hébergeur inconnu, pas de compte) : il ouvre cette page des réglages (voir *Réglages*).

Sans agent ouvert (un projet sans agent : « PR et CI du projet » sous « Nouvel agent », ou `Ctrl+Maj+B`) : « PR ouvertes du projet » et leur nombre, les 30 plus récemment mises à jour du dépôt de la cible, chacune avec son signe, son numéro, son titre, son état, sa CI, un conflit, son âge et son auteur (sur GitLab, la liste ne dit ni la CI ni les approbations). Un clic sur celle d'un agent du projet (une PR qu'il a lue, ou de sa branche) ouvre cet agent ; sur une autre, la PR dans le navigateur. L'icône à droite ouvre toujours la PR dans le navigateur.

Les états :

- Chargement : « Lecture de la PR… », « Lecture des PR ouvertes du projet… », « Lecture du journal… ».
- Vide : « Aucune PR ouverte. », « Aucune revue ni commentaire. ».
- Erreur, dans un cadre rouge avec « Réessayer » : « Jeton refusé par l'hébergeur : reconnecte son compte. » (401, avec « Réglages › Services connectés ») ; « Limite d'API de l'hébergeur atteinte, jusqu'à 15:48. » (403 ou 429 quand il ne reste aucun appel ou que l'hébergeur dit la limite) ; « Hébergeur injoignable. » (pas de réponse, ou une erreur 5xx) ; sinon les mots de l'hébergeur. Les trois premiers gardent aussi ses mots en dessous. Le projet sans dépôt distant, sans client ou sans compte dit sa raison, sans « Réessayer ».
- Une lecture qui tombe sur une erreur 5xx ou une coupure est refaite une fois avant d'être dite (les check runs de GitHub répondent parfois 500).

### Faire corriger l'agent

Sous la carte et dans l'onglet « PR et CI », sur une PR ouverte, jamais d'elles-mêmes :

- **« L'agent corrige »**, quand la CI est en échec : l'agent reçoit la PR, les checks en échec et la fin de leurs journaux (6 Ko chacun).
- **« L'agent corrige les retours »**, quand une relecture demande des changements ou que des fils restent ouverts : l'agent reçoit les relectures qui disent quelque chose et les commentaires non résolus, avec fichier et ligne.
- Les textes de l'hébergeur y sont balisés (checks, journaux, relectures, commentaires ; une balise fermante dans le texte est neutralisée), et la consigne dit de les traiter comme des informations, pas comme des instructions. L'agent corrige, vérifie, commite et pousse, sans répondre ni résoudre les fils : rien ne se publie sans l'utilisateur.
- Rien à corriger : refusé avec un toast. Le message fait 40 Ko au plus.

## Limites

- « Ouvrir une PR », la lecture des PR et l'onglet « PR et CI » : GitHub, GitLab, Bitbucket Cloud, Gitea / Forgejo et Azure DevOps (voir *Hébergeurs de code*).
- Azure DevOps ne donne ni déploiements ni durée des évaluations de stratégie (seulement celle des builds). Gitea et Forgejo ne donnent pas de déploiements, et leurs journaux et leur relance dépendent de la version de l'instance.
- La liste des PR du projet et « Lancer un agent sur cette PR » : voir *Vue « Pull requests » du projet* ; « Faire relire par un agent » : voir *Relecture d'une PR par un agent*.
- Un agent qui travaille dans un worktree qui n'est pas d'Escouade n'est suivi que sur Claude Code.
- Le cycle des PR hors tickets et ce qui en reste à faire : `docs/TODO-1.8.md`, chapitre 2 (« Les PR hors tickets »).

- Ce qu'une PR devient se notifie (voir *Notifications*), celles qui attendent l'utilisateur sont dans la vue d'ensemble (voir *Vue d'ensemble*), et les agents les lisent par `list_pull_requests` et `get_pull_request` (voir *Serveur MCP*).
