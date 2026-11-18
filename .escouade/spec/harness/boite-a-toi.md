---
title: "La boîte « À toi »"
subject: "harness"
sources: ["docs/INBOX.md (ESC-508, choix de l'utilisateur du 2026-11-17 au § 15)","design/inbox-2-3/README.md","docs/REMOTE.md (Le classement des commandes)"]
tickets: ["ESC-508","ESC-509","ESC-520"]
decisions: ["la-boite-a-toi-les-demandes-asynchrones",204]
compared: null
---

## Ce que c'est

Ce qu'un agent attend de l'utilisateur **sans s'arrêter de travailler** : un choix entre des options (avec leurs rendus à comparer), un oui ou non (un feu vert), une réponse libre, une tâche que l'utilisateur seul peut faire. L'agent la dépose par le serveur MCP (`ask_user`) et continue ; la réponse lui revient en message, marquée comme venant de l'utilisateur, et **ne déclenche rien d'autre**. La boîte repose sur le serveur MCP : serveur éteint, pas de boîte.

IN-1 (ESC-509) livre le cœur, les outils et les commandes de la fenêtre ; l'écran (IN-2, direction B « La file ») et ce qu'Escouade y verse de lui-même, les notifications et la distance (IN-3) suivent.

## Comportement

### Une demande

- **Sortes** (`kind`) : `choice` (2 à 6 options, `multi` pour en choisir plusieurs), `confirm` (`yesLabel`, `noLabel`, `gate` pour un feu vert), `answer` (texte libre, `recommend.text` le propose), `task` (10 étapes à cocher au plus).
- **Champs** et bornes : titre d'une ligne, 140 caractères ; contexte en Markdown, 8 000 ; options : libellé 60, description 600, lettre `a` à `f` (par défaut leur rang) ; étapes 300 ; recommandation (`option` ou `text`, et `why`, 500) jamais choisie de force ; `unblocks` (200) ; `tickets` qui attendent la réponse (10 au plus) ; `urgency` (`normal`, `today`) ; durée de vie 14 jours par défaut, de 1 à 30 ; `decision` ; `wake` (vrai par défaut).
- **Auteur** : l'agent d'Escouade qui la dépose (son nom, s'il est chef d'escouade du projet), ou « Claude · client MCP » pour Claude hors Escouade. Le projet et le ticket se déduisent pour un agent ; Claude hors Escouade donne le projet.
- **Un texte qui ressemble à un secret** (jeton GitHub, GitLab, OpenAI/Anthropic `sk-`, Slack, clé AWS, clé PEM) est refusé : « Ce texte ressemble à un secret (une clé ou un jeton) : ne le mets pas dans une demande. »
- **Bornes de la boîte** : 10 demandes ouvertes par agent (Claude hors Escouade compte pour un), 40 par projet, 200 en tout ; 20 dépôts par auteur et par heure. Chaque refus dit quoi faire (retirer, regrouper).
- Chaque demande a un numéro à elle, `a-<n>`, jamais redonné.

### Ses états et son histoire

- **Ouverte** ; **reportée** (« Plus tard » jusqu'à une heure, puis revenue, marquée) ; **répondue** (la réponse part à son auteur après 10 s, le temps de la reprendre ; tant que l'auteur ne l'a pas lue, « Revenir sur ma réponse » la rouvre) ; **lue** (le message est arrivé à l'agent, ou l'auteur l'a lue par `get_ask` ou `wait_ask`) ; **retirée** par son auteur (avec sa raison) ; **périmée** (sa date passée) ; **retirée par Escouade** (son agent supprimé ou archivé, son ticket terminé, supprimé ou renvoyé « À faire » alors qu'il était en cours, son projet retiré : la raison le dit).
- L'histoire garde chaque étape (déposée, mise à jour, reportée, revenue, laissée à un agent, répondue par qui et par quel geste, reprise, envoyée, lue, retirée, périmée, fichiers effacés), sans jamais le texte de la demande ; 40 lignes au plus (la première et les dernières).
- Le tout est gardé dans l'état durable (`state.json`, ses copies et sa reprise) : une demande survit au redémarrage, avec son numéro.
- Une demande close reste dans l'histoire 90 jours (300 closes au plus), puis part avec ses fichiers.

### Les pièces jointes

- À la demande (`files`) ou à une option (`options[].files`) : images PNG, JPEG, GIF, WebP, AVIF ; PDF ; Markdown, Word (`.docx`), CSV et TSV ; tout autre fichier. Un SVG n'est jamais une image (il peut porter un script) : c'est un fichier.
- **Le type est lu dans le contenu**, jamais dans le nom ni dans ce que l'agent déclare. Un fichier qui se dit image (par son extension ou son `mediaType`) et n'en est pas une est refusé : « Le fichier « x.png » n'est pas une image PNG, JPEG, GIF, WebP ou AVIF. » Une autre extension qui ment garde son nom, rangée selon son contenu.
- **Bornes**, tenues avant de lire : 12 fichiers par demande (options comprises), d'abord le nombre ; 25 Mo par fichier, sur la taille annoncée d'un contenu en base64 ou celle du fichier ouvert ; 64 Mo par demande ; 1 Go pour toute la boîte.
- **Le chemin** : un agent d'Escouade ne donne qu'un fichier de son dossier de travail, par un chemin relatif à lui ou complet, vérifié sur le fichier ouvert (les liens qui en sortent refusés, mêmes mots qu'un fichier absent) ; Claude hors Escouade, un chemin complet ; ou le contenu en base64 avec son nom. Mêmes refus que pour un ticket (réseau, appareil, pas un fichier ordinaire).
- **Copiés** dans `<données>/asks/<id>/files/<uniq>/<nom>`, nom nettoyé : ils **survivent au worktree** de l'agent. Un dépôt refusé ne laisse ni demande ni fichier.
- **Gardés 30 jours** après la fin de la demande, puis effacés (au démarrage, puis à chaque passage de la boîte) ; la demande dit alors « fichiers effacés ». « Vider les fichiers des demandes closes » les efface tous d'un coup, et la place qu'ils prennent se lit.
- Servis à la fenêtre par la demande et le fichier, jamais par un chemin : `inbox_file` rend 4 Mo au plus à la fois, en base64. « Ouvrir » ne passe jamais au système un programme, un script, une page HTML ni un SVG : ceux-là n'ont que « Afficher dans le dossier ».

### La réponse rendue à l'agent

- Un choix (une option, ou plusieurs avec `multi`), un oui ou non, un texte, une tâche faite ou non avec ses étapes ; un mot (2 000 caractères) ; des fichiers joints à la réponse (copies de la fenêtre), lus par `get_ask_file`.
- 10 s après le geste, la réponse part : **un message dans la conversation de l'agent**, marqué « Réponse de l'utilisateur » : « [Escouade · ta demande a-12 « … »] L'utilisateur a choisi B · Cartes empilées. Son mot : « … » ». Au repos, il le réveille (un nouveau tour) ; au travail, il arrive pendant son tour comme un message tapé. Avec `wake: false`, rien ne part : l'agent la lit par `get_ask`. Claude hors Escouade la lit par `get_ask` ou `wait_ask`.
- Le message finit par « Rien d'autre n'a été fait : c'est à toi d'agir, avec tes propres garde-fous. » ; avec `decision: true`, il dit de proposer la décision (`propose_decision`).
- **Rien d'autre ne part** : répondre oui à « Publier ? » ne publie rien.

### Un ticket qui attend une demande

- Les tickets de `tickets` attendent la réponse : le pilote auto les laisse « À faire » tant que la demande est ouverte, reportée, ou répondue sans que la réponse soit partie. « Lancer » à la main les lance quand même, comme un ticket qui en attend un autre.
- La demande retirée, périmée ou retirée par Escouade : le ticket se bloque, avec la raison (« La demande a-12 qu'il attendait ne sera pas répondue… »).

### Déléguer, une demande à la fois

- C'est **l'utilisateur** qui laisse une demande à un agent (`inbox_delegate`), jamais son auteur ni un réglage. Selon « Les agents peuvent répondre aux demandes des autres agents » : « Personne » (le défaut) refuse ; « Seulement le chef d'escouade », le chef du projet de la demande ; « Tous les agents », n'importe lequel.
- **Jamais** un feu vert (`gate`), une tâche, ni l'auteur lui-même ; une demande d'un projet privé ne se laisse qu'à un agent de ce projet.
- L'agent la voit par `list_asks` (`delegated: true`) et y répond par `answer_ask`, tant que le réglage le permet encore. Sa réponse part aussitôt, marquée « <agent> (à qui l'utilisateur l'a laissée) ».

### Serveur MCP éteint

- Les outils n'existent pas (le serveur ne tourne pas) ; les commandes de la fenêtre disent « La boîte « À toi » est fermée avec le serveur MCP », et `inbox_list` rend une boîte fermée, vide.
- Les demandes sont **gardées, figées**, avec leurs fichiers : leur date court toujours, un agent supprimé les retire, une réponse déjà donnée part quand même. Rallumé, la boîte revient avec elles, et un ticket qui en attendait une attend toujours.
- `inbox_closing` donne ce que demande la question avant d'éteindre (le nombre de demandes ouvertes, les trois premiers titres).

### Les commandes de la fenêtre

`inbox_list` (d'un projet ou de tous, avec le nom du projet, sa marque privée, la clé du ticket et les agents à qui la laisser), `inbox_get`, `inbox_file`, `inbox_open_file` (« Ouvrir », « Afficher dans le dossier »), `inbox_answer` (avec les fichiers joints), `inbox_snooze`, `inbox_delegate`, `inbox_reopen`, `inbox_seen`, `inbox_closing`, `inbox_weight`, `inbox_erase_closed_files`. L'événement `inbox` dit à la fenêtre que la boîte d'un projet a changé.

## Sécurité

- Le texte et les fichiers d'une demande sont des **données d'agent**, qui peuvent relayer un tiers : lus par un autre agent que leur auteur (`get_ask`, `list_asks`, `wait_ask`, `get_ask_file`), ils sont marqués comme une donnée de tiers, jamais une consigne.
- **Projet privé** : ses demandes ne sont trouvées que par ses agents, avec les mêmes mots qu'une demande qui n'existe pas ; ses agents n'en déposent dans aucun autre projet. La fenêtre les montre, marquées.
- **Les journaux** : `app.log` ne dit que « ask a-12 deposited by agent e6e0… (choice, 3 options, 7 files) », « answered », « withdrawn », « read »… ; l'activité du serveur MCP ne garde que le projet, la sorte et le numéro, jamais le texte, un chemin ni un nom de fichier.

## Limites

- `wait_ask` attend 240 s au plus (120 par défaut), comme `wait_ticket`, sous le délai d'un client HTTP.
- Un ticket attend une demande par `tickets` d'`ask_user` ; `after: ["ask:…"]` de `create_ticket` et la carte du ticket qui le dit viennent avec l'écran.
- L'écran, le compteur, les vues de ce qu'Escouade sait déjà, les notifications, l'Overseer et le téléphone ne sont pas dans IN-1.

## À distance (ESC-520)

Reversée dans la 2.3, la boîte est servie par l'API distante du cœur. C'est le premier écran du téléphone.

- **Servies à un client distant** : `inbox_list`, `inbox_get`, `inbox_file`, `inbox_answer`, `inbox_snooze`, `inbox_delegate`, `inbox_reopen`, `inbox_seen`, `inbox_closing`, `inbox_weight`, `inbox_erase_closed_files`, et l'événement `inbox`. Chacune est dans la liste unique des commandes (`remoteapi/list.rs`, « Servies telles quelles » de `docs/REMOTE.md`).
- **Les octets d'un fichier** : `inbox_file_bytes` (une demande, un fichier) donne le fichier entier, 25 Mo au plus, en octets bruts. Ils partent en trames binaires, en parties au-delà de 16 Mo, jamais en base64. Son nom et son type sont dans `inbox_get`. `inbox_file` garde ses parts de 4 Mo en base64 pour la fenêtre.
- **Les fichiers d'une réponse** arrivent du client en corps bruts, par les fichiers joints (`attach_open`, `attach_chunk`, `attach_finish`), puis `inbox_answer` les nomme.
- **« Ouvrir » et « Afficher dans le dossier »** (`inbox_open_file`) agissent sur la machine du cœur. La commande change de machine en mode serveur : elle est refusée à distance (`not-served-yet`) jusqu'au mode serveur, qui la changera en téléchargement sur le poste.
- Ce qui reste (le serveur d'équipe, l'Overseer, les relectures de PR, le téléphone) est dans `docs/INBOX.md`, « Ce que la 2.2 ne livre pas ».
