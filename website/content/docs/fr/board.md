# Le Kanban

Le Kanban d’un projet range ses tickets en quatre colonnes. Tu écris un ticket avec ses critères d’acceptation, un agent le prend et travaille jusqu’à les atteindre, puis il te le remet à tester. Cette page décrit le tableau, les tickets et leurs cartes.

![Le Kanban d’un projet : les colonnes « À faire », « En cours », « À tester » et « Terminé », avec leurs tickets](/images/board.jpg)

## Ouvrir le Kanban

Chaque projet a son Kanban. En haut de la barre latérale, le sélecteur du projet propose « Agents » et « Kanban ». Clique sur « Kanban » : le tableau prend la place de la conversation. Clique sur « Agents » pour revenir à la liste des agents. Un badge sur « Kanban » compte les tickets « À tester » du projet.

Cliquer une notification de ticket (« prêt à tester » ou « bloqué ») ouvre aussi le Kanban du projet.

### L’en-tête du tableau

| Élément | Ce qu’il dit ou fait |
|---|---|
| « Kanban » et la ligne dessous | Le nom du projet, le nombre de tickets et combien sont « En cours » (« demo-api · 5 tickets · 2 en boucle »). |
| Les places | « 1 place libre » ou « Toutes les places sont prises », ou bien pourquoi rien ne démarre : « Quota atteint — reprise à 14:30 », « Claude Code introuvable — aucun ticket ne démarre », une branche cible sans commit ou introuvable. |
| « Importer » | Ouvre la fenêtre d’import des tickets de Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues ou Azure Boards (voir [Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards](integrations)). |
| « Après validation : … » | Résume ce que fait « Valider » (« merge squash → main », « PR → main », « push ticket/* », « laisser en l’état »). Un clic ouvre Réglages › Projets › Kanban (voir [Réglages du Kanban](settings-board)). |
| « Pilote auto » | L’interrupteur du [pilote auto](autopilot) de ce projet. |

Quand le pilote auto est en pause, une ligne sous l’en-tête dit pourquoi et propose « Reprendre maintenant ».

## Les colonnes

| Colonne | Ce qu’elle contient | Ordre |
|---|---|---|
| « À faire » | Les tickets qui attendent un agent. | Par priorité, de haut en bas, puis par ancienneté. |
| « En cours » | Les tickets dont un agent s’occupe (« Boucle 2/5 »), et ceux qui sont bloqués. | Par heure de démarrage. |
| « À tester » | Les tickets que l’agent te remet : critères atteints, ou nombre maximal de boucles atteint. À toi de les tester, puis de les valider ou de les renvoyer. | Par heure d’arrivée. |
| « En revue » | Les tickets dont la pull request est ouverte chez l’hébergeur, jusqu’à son merge. Elle n’apparaît que si le Kanban ouvre des PR ou qu’un ticket y est. | Par heure d’arrivée « À tester ». |
| « Terminé » | Les tickets validés. | Le plus récent en tête. |

Chaque colonne affiche son nombre de tickets. Une colonne vide dit ce qu’elle attend (« Ajoute un ticket : un agent le prendra dès qu’une place se libère », « Rien à tester »). Seule la colonne « À faire » a un bouton `+` (« Nouveau ticket »).

Les cartes ne se déplacent pas à la main : un ticket change de colonne selon ce que fait son agent et ce que tu décides (voir [le pilote auto](autopilot) et [Tester et valider un ticket](testing)).

## Créer un ticket

1. Dans la colonne « À faire », clique sur `+` (« Nouveau ticket »). Une grande fenêtre « Nouveau ticket » s’ouvre, pour écrire de longs textes.
2. Remplis les champs. Seul le titre est obligatoire.
3. Appuie sur `Ctrl+Entrée` (le bouton « Ajouter » fait la même chose). Le ticket arrive à la fin de « À faire ».

Sous macOS, `Ctrl` devient `⌘`.

| Champ | À quoi il sert |
|---|---|
| « Titre du ticket » | Ce que fait le ticket, en une ligne. « Ajouter » reste grisé tant que le titre est vide. |
| « Description » | Le contexte, que l’agent lira avec le titre. Facultative. |
| Images | Une capture du bug ou une maquette : colle-la (`Ctrl+V`) ou glisse-la n’importe où dans la fenêtre, ou clique sur « Joindre un fichier ». Elle apparaît en vignette sous la description, et sa croix la retire. PNG, JPEG, GIF et WebP, 5 Mo par image et 18 Mo en tout, comme les pièces jointes du [chat](chat), et 20 images au plus. L’agent du ticket les reçoit avec son premier message, et elles restent dans sa conversation pour les boucles suivantes. Le [serveur MCP](mcp) les joint, les retire, les liste et les lit aussi (`create_ticket`, `update_ticket`, `get_ticket`, `get_ticket_image`). |
| Fichiers (« Joindre un fichier ») | N’importe quel autre fichier : un tableur, une archive, un journal, un export… Mêmes gestes que pour une image, et le bouton « Joindre un fichier » ouvre le choix de n’importe quel fichier. Tout ce qui n’est pas une image que le ticket peut montrer (un autre format, une image de plus de 5 Mo, un PDF, un texte…) apparaît en puce sous les images (icône, nom, taille), dont la croix le retire ; 20 fichiers au plus, en plus des 20 images. Il est copié aussitôt, jamais lu à son emplacement d’origine : l’agent du ticket le lit plus tard, quand l’original peut avoir bougé ou disparu. Tant que le ticket n’est pas enregistré, la copie attend dans un brouillon (vidé après 24 h) ; « Abandonner » efface les copies faites par la fenêtre, et « Ajouter » ou « Enregistrer » attendent la fin des copies. La taille maximale de [Réglages › Avancé › Pièces jointes](settings-claude-code#pieces-jointes) s’applique, et au-delà de 200 Mo la puce porte un avertissement, comme dans le [chat](chat#pieces-jointes). L’agent du ticket lit le dossier de ses fichiers sans demander, et reçoit à la fin de son premier message une ligne « Fichier joint : nom, chemin » par fichier. Le [serveur MCP](mcp) les joint, les retire, les liste et les lit aussi (`create_ticket`, `update_ticket`, `get_ticket`, `get_ticket_file`). |
| « Critères d’acceptation » | Un critère par ligne. L’agent vérifie chacun, et le ticket n’est fini que quand ils sont atteints. Sans critère, Escouade en met deux : « Implémentation conforme au ticket » et « Tests verts » (dans la langue de l’interface au moment de la création). |
| « Boucles max » | Le nombre maximal de boucles de l’agent : 3, 5 (par défaut) ou 8. Voir [le pilote auto](autopilot#la-boucle). |
| « Branche » | Par défaut « Nouvelle branche `ticket/<clé>` » : le ticket a sa propre branche, créée à son démarrage. Pour un ticket importé d’une source, la clé et le titre du ticket d’origine la nomment (`ticket/proj-123-corriger-la-connexion`, `ticket/45-corriger-la-connexion` pour `#45`), de même que son agent et le titre de sa PR. « Reprendre une branche existante… » ouvre le sélecteur de branches (locales ou distantes) : l’agent travaille alors sur cette branche, qu’Escouade ne supprime jamais, même une fois le ticket validé. La branche cible du Kanban et celle du dossier du projet ne peuvent pas être choisies. |
| « Compte », « Modèle », « Effort » | Sur quoi part l’agent du ticket, à chaque démarrage. Chacun vaut « Auto » par défaut et dit ce qu’il donne : « Auto · Principal » (le compte courant du harness du ticket pour ce projet), « Auto · Opus 5.5 (Kanban) » (le modèle du Kanban, sinon celui du compte), « Auto · Moyen (Kanban) ». « Compte » liste les comptes du harness du ticket que le projet peut utiliser (un [projet privé](accounts#un-compte-pour-un-projet-prive), seulement les siens s’il en a) ; « Modèle », les modèles qu’annonce ce compte (le compte courant sur « Auto »), comme le menu de la zone de saisie ; « Effort », les niveaux de ce modèle, grisé avec sa raison pour un modèle qui n’en prend pas, comme Haiku. Changer de harness remet le compte sur « Auto », changer de compte remet un modèle qu’il n’a pas, changer de modèle un niveau qu’il n’a pas : la fenêtre le dit sous les champs. Un ticket fixé sur un compte y reste : à sa limite, ou au-delà de « Changer de compte à », son agent attend la remise à zéro au lieu de passer sur un autre compte ou un autre harness, et sa carte le dit (« En attente du quota de Pro, reprise à 15:30 »). Un compte supprimé, désactivé ou déconnecté fait repasser le ticket en « Auto » : sa carte et une notification le disent. |
| « Après » | Les tickets qui doivent être terminés avant celui-ci. La liste montre les autres tickets du projet qui ne sont pas « Terminé » ; « Rechercher une clé » la filtre, et les tickets cochés restent visibles. Elle n’apparaît que s’il y a d’autres tickets. |

Un ticket ne peut pas venir après un ticket qui l’attend déjà, directement ou non : ni l’un ni l’autre ne démarrerait jamais. La case se décoche et la fenêtre dit lequel (« DEM-3 attend déjà DEM-5 (directement ou non). »).

Dans la fenêtre :

- `Ctrl+Entrée` ajoute (ou enregistre) depuis n’importe quel champ. `Entrée` seule le fait depuis le titre, et passe à la ligne dans les autres champs.
- `Échap`, la croix, un clic à côté ou « Annuler » ferment la fenêtre. Si tu as saisi quelque chose, Escouade demande « Abandonner les modifications ? » ; « Annuler » te ramène au formulaire avec ce que tu avais écrit.

> [!NOTE]
> Chaque ticket reçoit une clé, comme `DEM-5` : les trois premières lettres du nom du projet en majuscules (`TIC` s’il n’en a pas), puis un numéro qui n’est jamais réutilisé. Le premier ticket fixe aussi la branche cible du Kanban : la branche sur laquelle le projet se trouve alors. Si le projet est sur une HEAD détachée, le ticket est refusé tant que tu n’as pas choisi la branche cible dans [Réglages › Projets › Kanban](settings-board).

## Modifier, prioriser et supprimer un ticket

### Modifier

Clique sur une carte « À faire » (ou clic droit › « Modifier ») : la fenêtre « Modifier DEM-5 » s’ouvre avec les mêmes champs. `Ctrl+Entrée` ou « Enregistrer » enregistre. Seul un ticket « À faire » se modifie : une fois parti, un clic sur sa carte ouvre son agent.

### Passer un ticket en tête

Clic droit sur une carte « À faire » › « Passer en tête ». L’ordre de la colonne est celui dans lequel le pilote auto prend les tickets.

### Supprimer

Clic droit sur une carte › « Supprimer ».

| Colonne | Ce qui se passe |
|---|---|
| « À faire » | Une confirmation, puis le ticket, sa description, ses images et ses fichiers disparaissent. Les tickets qui l’attendaient ne l’attendent plus. Pour un ticket importé, la confirmation ajoute qu’il ne sera plus importé depuis son service : l’import automatique ne le ramène pas (tu peux toujours le réimporter à la main). |
| « En cours » et « À tester » | Une confirmation, puis le ticket disparaît et son agent est archivé. |
| « Terminé » | Le ticket disparaît tout de suite, sans confirmation. |

Avec « Supprimer le worktree d’un ticket terminé ou supprimé » (page [« Kanban »](settings-board) du projet, activé par défaut), les worktrees des agents du ticket sont supprimés aussi, sauf ceux qui gardent des changements non commités ou des commits ni mergés ni poussés : la carte de leur agent archivé le dit. Un ticket « Terminé » laissé en l’état voit alors son agent archivé.

Supprimer un ticket du Kanban ne le supprime pas chez Jira, Trello, GitHub, Linear, ClickUp, GitLab ou Azure Boards.

Archiver ou supprimer l’agent d’un ticket « En cours » ou « À tester » renvoie ce ticket dans « À faire », à zéro (Escouade le demande avant d’archiver : « Son ticket DEM-5 repartira « À faire ». »).

## Ce que montre une carte

Toutes les cartes ont leur clé, leur titre et, pour un ticket importé, la pastille de son service avec sa clé chez lui (un clic l’ouvre dans le service). Un ⚠ signale une synchronisation qui a échoué (voir [Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards](integrations#synchronisations-qui-echouent)). Un ticket fixé sur un compte montre son nom à côté de sa clé (« Pro »). Un ticket qui a son propre modèle ou son propre effort, choisis dans sa fenêtre ou donnés par le [serveur MCP](mcp), les montre dans une étiquette à côté de sa clé (« Opus · Max ») : ils passent avant ceux du Kanban pour son agent. Avec Haiku (celui du ticket, sinon du Kanban, sinon de l’app), qui ne gère pas l’effort, l’étiquette n’en affiche pas. Un trombone suivi d’un nombre dit combien d’images et de fichiers le ticket a (« 2 images jointes » au survol, ou « 1 image jointe, 2 fichiers joints » s’il a aussi des fichiers).

| Colonne | La carte montre |
|---|---|
| « À faire » | Le nombre de critères et de boucles (« 3 critères · max 5 boucles »), et quand le ticket partira : « Pris dès qu’une place se libère », « En attente d’une place (2/2) », « ⏸ après DEM-3 », « Pilote auto désactivé », « En attente : pilote auto en pause »… Avec le pilote auto éteint, un bouton « Lancer » apparaît quand une place est libre. |
| « En cours » | « Boucle 2/5 » ; les critères, cochés (✓) ou non (○), avec la note de l’agent au survol ; une barre des critères atteints ; les trois derniers éléments de l’avancement (« +2 » pour les autres) ; la ligne d’état que l’agent a publiée par le [serveur MCP](mcp) ; ce qu’il fait (« Lit src/billing/invoices.ts »), « Prépare le worktree · … » pendant la préparation, « Question en attente de ta réponse », ou « Reprise à 14:30 » s’il attend son quota. En bas : le nom de l’agent avec son état, et le coût du ticket (précédé de « ≈ » tant qu’il est estimé). |
| « À tester » | « Ce qui a été fait » en entier, les critères, « Objectif partiel » si le nombre maximal de boucles a été atteint sans tous les critères, le coût et « 3/3 critères », puis les boutons de [validation et de test](testing). |
| « En revue » | Le numéro de la PR, son état, sa CI et ses jobs, ses approbations, ses fils ouverts et un conflit avec la cible (voir [plus bas](#la-colonne-en-revue)). |
| « Terminé » | Ce que le ticket est devenu (« ⤵ Mergé dans main · squash », « ⇡ PR #41 → main », « ⇡ Poussé sur ticket/dem-3 », « ◇ Laissé dans le worktree », « ∅ Aucune modification »), puis « 3 boucles · 1,12 $ ». Une PR se rouvre d’un clic. |

Un ticket bloqué (« En cours » ou « À tester ») affiche son motif dans un encadré : « Interrompu », « Erreur : … », « Bilan des critères manquant », un conflit… Un ticket « En cours » bloqué a un bouton « Reprendre » et ne tient plus de place. La carte de l’agent qui attend ta réponse a un liseré ambre.

> [!TIP]
> « Me prévenir pour › Tickets (prêt à tester, bloqué) » dans les réglages décide si un ticket qui arrive dans « À tester » ou qui se bloque te prévient (voir [Notifications](notifications)).

## La colonne « En revue »

Avec « Ouvrir une pull request » comme action de validation (voir [Réglages du Kanban](settings-board)), un ticket validé attend son merge dans cette colonne. La PR s’ouvre par l’API de l’[hébergeur de code](code-hosts) du dépôt ; sa carte dit où elle en est, sans ouvrir le navigateur.

![La colonne En revue : la carte d’un ticket avec sa PR, sa CI et ses approbations](/images/board-review.jpg)

Un clic sur sa CI ouvre ses jobs dans Escouade. Selon ce qui arrive, la carte propose :

- **« L’agent corrige »**, quand la CI échoue : son agent est restauré et reçoit les jobs en échec et la fin de leurs journaux, entre balises, donnés comme des informations et non des consignes. Le pilote auto le fait seul, dans la limite des boucles du ticket.
- **« L’agent corrige les retours »**, quand un relecteur demande des changements ou commente : jamais seul, même avec le pilote auto.
- **« L’agent met à jour la branche »**, quand la PR est en conflit avec sa cible : jamais seul non plus.
- **« Merger (squash) »**, pour une PR approuvée, verte et sans conflit, et **« Merger quand c’est vert »** tant que sa CI tourne. Rien ne se merge sans ce geste.

Mergée, ici ou chez l’hébergeur, le ticket passe « Terminé » et les tickets qui l’attendaient partent. Fermée sans merge, il se bloque, avec « Remettre à faire » et « Terminer quand même ». Une erreur de l’hébergeur ne déplace jamais un ticket : la carte la dit, avec ce qui a été lu en dernier.

Les tickets sont relus toutes les minutes fenêtre devant et toutes les 5 minutes sinon ; « Rafraîchir », dans l’en-tête de la colonne, les relit tout de suite. Un ticket terminé et mergé dit ensuite dans quels [environnements](code-hosts#les-environnements-et-les-deploiements) il tourne.

## L’agent du ticket et sa conversation

Quand un ticket démarre, Escouade crée un agent pour lui. Il porte la clé et le titre du ticket (`dem-5-export-csv-des-factures`) et apparaît dans la liste des agents avec une étiquette « ▸ DEM-5 · boucle 1/5 », puis « DEM-5 · à tester ». Il travaille dans un worktree à lui.

- Clique sur la carte d’un ticket « En cours », « À tester » ou « Terminé » (ou donne-lui le focus et appuie sur `Entrée` ou `Espace`) pour ouvrir la conversation de son agent. Clic droit › « Ouvrir l’agent » fait la même chose.
- C’est la conversation des autres agents (voir [Les agents](managing-agents) et [Le chat](chat)) : tu y lis ce que fait l’agent, tu réponds à ses questions et à ses demandes d’autorisation, tu lui écris. L’agent d’un ticket reste dans la vue d’Escouade ; il n’a pas de bouton pour la [vue Claude Code](claude-code-view).
- L’en-tête de la conversation et le bilan des critères ont un bouton « ▶ Tester » quand l’agent a de quoi lancer son travail, comme la carte « À tester » (voir [Tester et valider un ticket](testing)).

## Le bilan des critères

L’agent d’un ticket termine chacune de ses réponses par un bilan : quels critères sont atteints, ce qu’il a fait, et, quand il l’a préparée, la façon de lancer son travail pour le tester. Escouade lit ce bilan pour faire avancer le ticket, et la conversation l’affiche comme une carte « Bilan des critères » au lieu du texte brut :

- le compte des critères atteints (« 2/3 ») et, pour chacun, ✓ ou ○ avec la note de l’agent ;
- « Ce qui a été fait » : jusqu’à huit éléments courts, que la carte du ticket reprend ;
- « Lancement de test » : les étapes de « Préparation » et les processus (nom, commande, adresse), avec un bouton « ▶ Tester » quand le test est possible.

Si une réponse arrive sans bilan, l’agent est relancé une fois. Sans bilan une seconde fois, le ticket se bloque (« Bilan des critères manquant ») et tu peux le « Reprendre ».

## Aller plus loin

- [Le pilote auto](autopilot) : comment les tickets sont pris, bouclés et mis en pause.
- [Tester et valider un ticket](testing) : « ▶ Tester », « Valider », « Renvoyer ».
- [Jira, Trello, GitHub, Linear, ClickUp, GitLab et Azure Boards](integrations) : importer des tickets et tenir leur statut à jour.
- [Le serveur MCP](mcp) : créer et lancer des tickets depuis Claude.
- [Réglages du Kanban](settings-board) : tous les réglages du tableau.
