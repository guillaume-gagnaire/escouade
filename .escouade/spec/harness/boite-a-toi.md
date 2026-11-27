---
title: "La boîte « À toi »"
subject: "harness"
sources: ["docs/INBOX.md (ESC-508, choix de l'utilisateur du 2026-11-17 au § 15 ; § 5, § 11, § 13 pour IN-3 ; § 16 et § 18 pour le reversement)","design/inbox-2-3/README.md (direction B, captures b-)","docs/REMOTE.md (Le classement des commandes)","docs/INBOX.md (ESC-508, choix de l'utilisateur du 2026-11-17 au § 15 ; § 5, § 11, § 13 pour IN-3 ; § 5.1 pour ESC-557 ; § 10.2 pour le filtre d'ESC-578 ; § 18)","Demande de l'utilisateur du 2026-11-20 (ESC-557) : « quand toi, l'agent qui valide et merge un ticket, le fait, de pouvoir cocher comme terminé la tâche dans \"à toi\", afin d'être toujours à jour »","Demande de l'utilisateur du 2026-11-21 (ESC-578) : « Dans le \"À toi\", ajoute le fait de pouvoir filtrer par type de demande. »"]
tickets: ["ESC-508","ESC-509","ESC-510","ESC-513","ESC-520","ESC-538","ESC-540","ESC-552","ESC-585","ESC-557","ESC-578","ESC-628","ESC-645","ESC-647"]
decisions: [210,204,240,242,222,227,263,290,296,315,359]
compared: null
---

## Ce que c'est

Ce qu'un agent attend de l'utilisateur **sans s'arrêter de travailler** : un choix entre des options (avec leurs rendus à comparer), un oui ou non (un feu vert), une réponse libre, une tâche que l'utilisateur seul peut faire. L'agent la dépose par le serveur MCP (`ask_user`) et continue ; la réponse lui revient en message, marquée comme venant de l'utilisateur, et **ne déclenche rien d'autre**. La boîte repose sur le serveur MCP : serveur éteint, pas de boîte.

IN-1 (ESC-509) livre le cœur, les outils et les commandes de la fenêtre ; IN-2 (ESC-510) l'écran, dans la direction B « La file » ; IN-3 (ESC-513) ce qu'Escouade y verse de lui-même, le résumé des notifications et la consigne aux agents (plus bas). La distance (le téléphone) vient avec la 2.3.

## Comportement

### Une demande

- **Sortes** (`kind`) : `choice` (2 à 6 options, `multi` pour en choisir plusieurs), `confirm` (`yesLabel`, `noLabel`, `gate` pour un feu vert), `answer` (texte libre, `recommend.text` le propose), `task` (10 étapes à cocher au plus).
- **Champs** et bornes : titre d'une ligne, 140 caractères ; contexte en Markdown, 8 000 ; options : libellé 60, description 600, lettre `a` à `f` (par défaut leur rang) ; étapes 300 ; recommandation (`option` ou `text`, et `why`, 500) jamais choisie de force ; `unblocks` (200) ; `tickets` qui attendent la réponse (10 au plus) ; `urgency` (`normal`, `today`) ; durée de vie 14 jours par défaut, de 1 à 30 ; `decision` ; `wake` (vrai par défaut).
- **Auteur** : l'agent d'Escouade qui la dépose (son nom, s'il est chef d'escouade du projet), ou « Claude · client MCP » pour Claude hors Escouade. Le projet et le ticket se déduisent pour un agent ; Claude hors Escouade donne le projet. Une demande qui a changé de main a pour auteur le chef d'escouade qui l'a reprise, ou n'en a plus de vivant (plus bas, *Une demande qui change de main*).
- **Un texte qui ressemble à un secret** (jeton GitHub, GitLab, OpenAI/Anthropic `sk-`, Slack, clé AWS, clé PEM) est refusé : « Ce texte ressemble à un secret (une clé ou un jeton) : ne le mets pas dans une demande. »
- **Bornes de la boîte** : 10 demandes ouvertes par agent (Claude hors Escouade compte pour un), 40 par projet, 200 en tout ; 20 dépôts par auteur et par heure. Chaque refus dit quoi faire (retirer, regrouper).
- Chaque demande a un numéro à elle, `a-<n>`, jamais redonné.

### Ses états et son histoire

- **Ouverte** ; **reportée** (« Plus tard » jusqu'à une heure, puis revenue, marquée) ; **répondue** (la réponse part à son auteur après 10 s, le temps de la reprendre ; tant que l'auteur ne l'a pas lue, « Revenir sur ma réponse » la rouvre) ; **lue** (le message est arrivé à l'agent, ou l'auteur l'a lue par `get_ask` ou `wait_ask`) ; **retirée** par son auteur (avec sa raison) ; **périmée** (sa date passée) ; **retirée par Escouade** (son ticket supprimé, ou renvoyé « À faire » alors qu'il était en cours, son projet retiré : la raison le dit). Une demande encore due dont l'auteur s'en va (son ticket validé, l'agent archivé ou supprimé) n'est jamais retirée : elle **change de main** (plus bas).
- L'histoire garde chaque étape (déposée, mise à jour, reportée, revenue, laissée à un agent, répondue par qui et par quel geste, reprise, envoyée, lue, retirée, périmée, changée de main, fichiers effacés), sans jamais le texte de la demande ; 40 lignes au plus (la première et les dernières).
- Le tout est gardé dans l'état durable (`state.json`, ses copies et sa reprise) : une demande survit au redémarrage, avec son numéro.
- Une demande close reste dans l'histoire 90 jours (300 closes au plus), puis part avec ses fichiers.

### Les pièces jointes

- À la demande (`files`) ou à une option (`options[].files`) : images PNG, JPEG, GIF, WebP, AVIF ; PDF ; Markdown, Word (`.docx`), CSV et TSV ; tout autre fichier. Un SVG n'est jamais une image (il peut porter un script) : c'est un fichier.
- **Le type est lu dans le contenu**, jamais dans le nom ni dans ce que l'agent déclare. Un fichier qui se dit image (par son extension ou son `mediaType`) et n'en est pas une est refusé : « Le fichier « x.png » n'est pas une image PNG, JPEG, GIF, WebP ou AVIF. » Une autre extension qui ment garde son nom, rangée selon son contenu.
- **Bornes**, tenues avant de lire : 12 fichiers par demande (options comprises), d'abord le nombre ; 25 Mo par fichier, sur la taille annoncée d'un contenu en base64 ou celle du fichier ouvert ; 64 Mo par demande ; 1 Go pour toute la boîte.
- **Le chemin** : un agent d'Escouade ne donne qu'un fichier de son dossier de travail, par un chemin relatif à lui ou complet, vérifié sur le fichier ouvert (les liens qui en sortent refusés, mêmes mots qu'un fichier absent) ; Claude hors Escouade, un chemin complet ; ou le contenu en base64 avec son nom. Mêmes refus que pour un ticket (réseau, appareil, pas un fichier ordinaire).
- **Copiés** dans `<données>/asks/<id>/files/<uniq>/<nom>`, nom nettoyé : ils **survivent au worktree** de l'agent. Un dépôt refusé ne laisse ni demande ni fichier.
- **Gardés 30 jours** après la fin de la demande (« Garder les fichiers des demandes closes » : 7, 30 ou 90 jours ; toute autre valeur vaut 30), puis effacés (au démarrage, puis à chaque passage de la boîte) ; la demande dit alors « fichiers effacés », et `get_ask_file` le dit avec le nombre de jours réglé. « Vider les fichiers des demandes closes » les efface tous d'un coup, et la place qu'ils prennent se lit (Réglages › Avancé › Données locales).
- Servis à la fenêtre par la demande et le fichier, jamais par un chemin : `inbox_file` rend 4 Mo au plus à la fois, en base64. « Ouvrir » ne passe jamais au système un programme, un script, une page HTML ni un SVG : ceux-là n'ont que « Afficher dans le dossier ». « Enregistrer sous… » (`inbox_save_file`) copie la copie du cœur là où l'utilisateur le choisit dans la boîte du système, que le cœur ouvre.

### La réponse rendue à l'agent

- Un choix (une option, ou plusieurs avec `multi`), un oui ou non, un texte, une tâche faite ou non avec ses étapes ; un mot (2 000 caractères) ; des fichiers joints à la réponse (copies de la fenêtre), lus par `get_ask_file`.
- 10 s après le geste, la réponse part : **un message dans la conversation de l'agent**, marqué « Réponse de l'utilisateur » : « [Escouade · ta demande a-12 « … »] L'utilisateur a choisi B · Cartes empilées. Son mot : « … » ». Au repos, il le réveille (un nouveau tour) ; au travail, il arrive pendant son tour comme un message tapé. Avec `wake: false`, rien ne part : l'agent la lit par `get_ask`. Claude hors Escouade la lit par `get_ask` ou `wait_ask`.
- Le message finit par « Rien d'autre n'a été fait : c'est à toi d'agir, avec tes propres garde-fous. » ; avec `decision: true`, il dit de proposer la décision (`propose_decision`).
- **Rien d'autre ne part** : répondre oui à « Publier ? » ne publie rien.

### Un ticket qui attend une demande

- Les tickets de `tickets` attendent la réponse : le pilote auto les laisse « À faire » tant que la demande est ouverte, reportée, ou répondue sans que la réponse soit partie. « Lancer » à la main les lance quand même, comme un ticket qui en attend un autre.
- La demande retirée, périmée ou retirée par Escouade : le ticket se bloque, avec la raison (« La demande a-12 qu'il attendait ne sera pas répondue… »). Une demande qui change de main ne le bloque pas : il l'attend toujours.

### Une demande qui change de main (ESC-585)

- **Quand** : l'auteur d'une demande encore due (ouverte, reportée, ou répondue sans que son auteur ait lu la réponse) s'en va : le ticket de son agent est validé (l'agent archivé ou supprimé avec sa carte), ou l'agent est archivé ou supprimé à la main. Elle n'est jamais retirée pour cela : c'est toujours ce qui attend l'utilisateur, et sa réponse doit revenir à quelqu'un.
- **Au chef d'escouade du projet** (le chef choisi pour le projet, vivant : ni archivé ni supprimé, et autre que l'auteur qui s'en va) : il en devient l'auteur (« chef d'escouade »), l'histoire le dit (« Reprise par chef-atlas : le ticket ESC-549 est validé », « … : dev a été archivé », « … : dev a été supprimé ») ; sa date, ses fichiers, ses options et les tickets qui l'attendent restent. Elle ne dépend plus du ticket validé ni de l'agent parti. La réponse revient au chef comme un message, qui dit d'où elle vient (« (Elle était de dev et t'est revenue : le ticket ESC-549 est validé.) ») ; une réponse déjà partie vers l'ancien auteur et pas lue repart vers le chef. Le chef la change ou la retire comme la sienne ; laissée à lui par délégation, elle cesse de l'être.
- **Sans chef** : elle reste, sans auteur vivant (ni l'ancien agent ni Claude hors Escouade ne la changent ni ne la retirent), et sa date court toujours. Une réponse donnée reste dans la demande, sa trace et « Répondu récemment » ; le journal de l'app (`app.log`) dit « ask a-12 kept with no living author ». Dès que le projet a un chef, elle passe à lui, avec la réponse pas lue.
- **L'agent supprimé à la main** suit la même règle, sans rien demander à la suppression : ce qui était dû le reste. L'utilisateur qui n'en veut plus la laisse se périmer ou y répond (« Je ne le ferai pas », « Non ») ; le chef qui la reprend la retire si elle n'a plus lieu d'être.
- **Un chef qui s'en va à son tour** : ses demandes, reprises comprises, passent au nouveau chef, ou restent sans auteur vivant.
- **Ce qui la retire encore** : son ticket supprimé, ou renvoyé « À faire » alors qu'il était en cours (le travail repart, son agent redemandera), son projet retiré, sa date passée.
- **Dans la fenêtre, discrètement** : sa ligne de la file dit « reprise de dev » (ou « sans auteur ») après sa sorte ; son panneau, sous le titre, « Reprise par chef-atlas : le ticket ESC-549 de dev est validé. » ou « Sans auteur vivant : dev a été archivé, et le projet n'a pas de chef d'escouade. Ta réponse reste ici ; elle part au chef dès qu'il y en a un. », et sous la réponse « Ta réponse reste ici : elle part au chef d'escouade dès que le projet en a un. » ; son histoire, « Changée de main ». En français et en anglais.
- Les demandes retirées avant cette règle ne sont pas rouvertes.

### Déléguer, une demande à la fois

- C'est **l'utilisateur** qui laisse une demande à un agent (`inbox_delegate`), jamais son auteur ni un réglage. Selon « Les agents peuvent répondre aux demandes des autres agents » : « Personne » (le défaut) refuse ; « Seulement le chef d'escouade », le chef du projet de la demande ; « Tous les agents », n'importe lequel.
- **Jamais** un feu vert (`gate`), une tâche, ni l'auteur lui-même ; une demande d'un projet privé ne se laisse qu'à un agent de ce projet.
- L'agent la voit par `list_asks` (`delegated: true`) et y répond par `answer_ask`, tant que le réglage le permet encore. Sa réponse part aussitôt, marquée « <agent> (à qui l'utilisateur l'a laissée) ».

### Serveur MCP éteint

- Les outils n'existent pas (le serveur ne tourne pas) ; les commandes de la fenêtre disent « La boîte « À toi » est fermée avec le serveur MCP », et `inbox_list` rend une boîte fermée, vide.
- **Rien de la boîte ne se montre** : ni tuile, ni compteur, ni raccourci, ni vue (une vue ouverte se ferme). Allumé ou éteint, la fenêtre le suit aussitôt, sans redémarrer.
- **Éteindre avec des demandes ouvertes le demande d'abord** : « Éteindre le serveur MCP ? 4 demandes de tes agents t'attendent dans la boîte « À toi ». Éteint, la boîte se ferme : elles sont gardées, sans que tu les voies ni que leurs agents puissent les lire, jusqu'à ce que tu le rallumes. Elles se périment à leur date. », les trois premiers titres (« et N autres »), « Les demandes de permission, les tickets à valider et les décisions proposées ne bougent pas : tu les retrouves où ils sont aujourd'hui. » ; « Ouvrir la boîte », « Annuler » (les réglages reviennent), « Éteindre quand même ». Rien n'attend : il s'éteint sans rien demander. `inbox_closing` donne ce que dit la question.
- Les demandes sont **gardées, figées**, avec leurs fichiers : leur date court toujours, un agent archivé ou supprimé les passe au chef d'escouade (ou les laisse sans auteur vivant), une réponse déjà donnée part quand même. Rallumé, la boîte revient avec elles, et un ticket qui en attendait une attend toujours.

### L'écran (ESC-510, direction B « La file »)

- **Sa place** : une vue de tous les projets, ouverte par la tuile « À toi » de la barre des projets, sous « Vue d'ensemble », ou par `Ctrl+Maj+J` (voir *Barre des projets*, *Raccourcis*) ; elle prend la place du projet, des stats ou de la vue d'ensemble, et `Échap`, la tuile ou le raccourci reviennent à ce qu'elle couvrait.
- **La file**, à gauche (300 px, 236 sous 1100 px de fenêtre) : « À toi » et le compte de la portée ; la portée « Tous » ou « Ce projet » (le dernier projet affiché) ; la jauge, une case par ligne (verte traitée aujourd'hui, ambre un agent arrêté, claire celle qui est ouverte) ; « 3 traitées aujourd'hui · il en reste 9 » ; puis les groupes : « Tes agents se sont arrêtés » (ambre ; les demandes de permission, les questions et les plans de ces agents, la plus ancienne d'abord, réglés sur place : les vues d'IN-3, plus bas ; un agent arrêté sans vue, comme celui qui demande une suppression, mène à sa conversation), « Aujourd'hui » (urgente, revenue de « Plus tard », attendue par un ticket ou déposée aujourd'hui ; l'urgente d'abord, puis la revenue, puis la plus ancienne), « Plus tôt », « Plus tard » (replié, avec l'heure du retour) et « Répondu récemment » (7 jours, 50 au plus : la réponse, qui, l'heure, « lue par <agent> » ; retirée, périmée).
- **Une ligne** : l'icône de sa sorte, le titre, depuis quand ; le projet (sa couleur), l'agent (son nom d'aujourd'hui), ses fichiers (« 1 PDF, 6 images ») ou sa sorte, « aujourd'hui » ou « revenue de Plus tard », « reprise de <agent> » ou « sans auteur » pour une demande qui a changé de main ; un choix montre sous elle la première image de chaque option (repris de la direction D). Un point marque une demande pas encore ouverte.
- **Une demande à la fois**, à droite : la barre « ‹ 3 sur 9 › », les touches `J` `K` `L`, « Aller à l'agent », « Le ticket » (son agent, sinon le Kanban) ; puis la sorte (« Aujourd'hui » en ambre), le projet, l'agent (« chef d'escouade »), le ticket, depuis quand ; le titre ; pour une demande qui a changé de main, la phrase qui le dit ; le texte, rendu en Markdown assaini comme la conversation (rien de distant sans clic) ; un choix, ses options sur leurs **planches** (son rendu sombre, ou le premier ; la ligne de crédits : lettre, nom, description, « 2 images ») et le **sceau** « Recommandé par l'agent » à demi hors de la planche recommandée ; une tâche, ses étapes à cocher ; ses fichiers ; « Recommandé : B · Cartes empilées. » et pourquoi ; « Ta réponse débloque : … » et les tickets qui l'attendent. Une demande reportée dit jusqu'à quand (« La remettre maintenant ») ; laissée à un agent, à qui (« La reprendre »).
- **La réponse, en bas** : un choix, ses options (« Recommandé » sur la sienne ; la réponse part de l'option recommandée, jamais choisie sans l'utilisateur) et « Comparer en grand » ; « Un mot pour l'agent (facultatif) » (« Ce que tu as vu » pour une tâche), le trombone (un fichier joint à la réponse, copié comme un brouillon), le micro (la dictée de la conversation), « Plus tard », puis le geste : « Choisir B » (plusieurs : « Envoyer 2 »), « Non, pas encore » / « Oui, publie » (les mots de l'agent, sinon « Non » / « Oui »), « Je ne le ferai pas » / « C'est fait », « Envoyer » (une réponse libre, partie du texte proposé). Dessous : « Ta réponse part à <agent>, qui la lit à son prochain tour ; tu peux revenir dessus tant qu'il ne l'a pas lue. », pour un feu vert « Un feu vert : toi seul y réponds… », et « Laisser <chef> trancher » quand le réglage le permet (jamais un feu vert ni une tâche).
- **Au clavier** (hors d'un champ, d'un menu, d'une boîte de dialogue) : `J` / `K` (ou `↓` / `↑` hors de la file) suivante, précédente ; `1` à `6` une option ; `Entrée` le geste principal (pas sur un bouton qui a le focus) ; `L` « Plus tard » ; dans la file, `↓` / `↑` d'une ligne à l'autre. Une réponse envoyée ou une demande reportée ouvre la suivante ; la boîte ouverte s'ouvre sur la première qui attend, sans la marquer vue.
- **« Plus tard »** : « Dans 1 heure », « Ce soir, 18:00 » (avant 17:00), « Demain, 9:00 », « Lundi, 9:00 » (sauf si demain est lundi), « Choisir… » (une date et une heure).
- **Envoyée** : « Ta réponse à « … » part dans 10 s · Annuler » en haut du panneau ; « Annuler » la reprend et la rouvre. Une demande répondue montre sa trace : la réponse, qui (« toi », « <agent>, délégué »), quand, par quel geste (clic, clavier, voix), « Envoyée à » / « Pas encore lue par » / « Lue par <agent> », « Revenir sur ma réponse » tant qu'elle n'est pas lue, « Changer d'avis… » ensuite (la conversation de l'agent, un message prêt à compléter) ; avec `decision`, que l'agent la propose dans Docs › Décisions, « Voir les décisions » ; son histoire.
- **Comparer en grand** : par-dessus la fenêtre, les options côte à côte sur leurs planches, le sceau ; « Sombre · Clair · Les deux » quand une option a un rendu clair (son nom finit par `-clair` ou `-light`) et un sombre ; `1` `2` `3`, `←` `→`, `Espace` (l'option seule dans la visionneuse), `Entrée`, `Échap` ; « Un mot pour l'agent » et « Choisir B » en bas.
- **Les fichiers** : une image en vignette (sa taille en pixels et en octets), qui ouvre la visionneuse de la conversation (les images de la demande, zoom, « Copier l'image », « Enregistrer sous… », « Ouvrir », « Afficher dans le dossier ») ; un PDF lu à côté du texte (dessous sous 1100 px), le premier ouvert, « Lire » pour un autre, « 3 pages » dès qu'il est lu ; un Markdown dessiné sous la liste, comme la conversation ; les autres (Word compris sur la 2.2) avec leur nom, leur taille, « Ouvrir » (« Afficher dans le dossier » pour un programme, un script, une page ou un SVG) et « Enregistrer sous… ».
- **Le PDF** est dessiné par pdf.js (`pdfjs-dist`, chargé à la première lecture), sans sa couche de scripts ni XFA, rien chargé du web (son worker et ses décodeurs WebAssembly sont des fichiers de l'app, les polices manquantes celles du système), la politique de la fenêtre inchangée : une page à la fois sur un canevas, blanche dans les deux thèmes, à la largeur du panneau ; « 3 sur 12 » où une page tapée mène, précédente et suivante (`←` `→`, Page précédente, Page suivante, Début, Fin), les vignettes des 30 premières pages, le zoom (50 % à 300 %), « Chercher » (les pages dont le texte tient les mots, casse et accents à part), « Plein écran » (`Échap` en sort). Ses liens et ses formulaires ne vivent pas. Un PDF protégé : « L'aperçu de ce fichier ne s'affiche pas : ce PDF est protégé par un mot de passe. »
- **La boîte vide** : « Rien ne t'attend », ce qui y arrive, les quatre sortes (« Un choix — entre des options, avec leurs rendus à comparer. »…), et que le compteur de la barre de gauche dit quand une demande arrive.
- **Accessibilité** : la file est une liste de boutons (`aria-current` sur la ouverte) ; les options, des boutons radio (ou des cases avec `multi`), `aria-keyshortcuts` ; une demande arrivée depuis la dernière lecture est dite par une région `status` (« Nouvelle demande de design-tableau : … »), sans prendre le focus ; la comparaison et le PDF en plein écran tiennent le focus ; tout se fait au clavier ; clair et sombre, français et anglais.
- **Le compteur** : sur la tuile, tous projets (voir *Barre des projets*) ; le titre de la file, la portée.

### Statistiques d'usage

Si l'utilisateur les laisse : `inbox_opened` (`from` : `tile`, `shortcut`, `settings`, `notification` (le clic sur le résumé) ; `scope`), `ask_answered` (`kind`, `delay` par tranche, `via`, `followed` pour un choix recommandé), `ask_snoozed`, `ask_reopened`, `inbox_compare_opened`, `inbox_attachment_opened` (`type`). Jamais un texte, un nom ni un chemin. Ce que seule la fenêtre voit passe par `inbox_track`, qui ne compte que ces valeurs-là.

### Les commandes de la fenêtre

`inbox_list` (d'un projet ou de tous, avec le nom du projet, sa marque privée, la clé du ticket et les agents à qui la laisser), `inbox_get`, `inbox_file`, `inbox_open_file` (« Ouvrir », « Afficher dans le dossier »), `inbox_save_file` (« Enregistrer sous… »), `inbox_answer` (avec les fichiers joints), `inbox_snooze`, `inbox_delegate`, `inbox_reopen`, `inbox_seen`, `inbox_closing`, `inbox_weight`, `inbox_erase_closed_files`, `inbox_track` ; pour les vues d'IN-3, `inbox_views`, `inbox_view_act` et `inbox_view_snooze`. L'événement `inbox` dit à la fenêtre que la boîte d'un projet a changé : elle la relit. Une demande qui a changé de main porte `handed` (son premier auteur, la cause : `ticket`, `archived` ou `deleted`, la clé du ticket validé, l'heure).

## Réglages

Sous « Les agents peuvent piloter Escouade » (Réglages › Agents › Piloter Escouade) :

- « La boîte « À toi » » : ce qu'elle est, « Ouvrir la boîte » ; serveur éteint, grisée, « Éteinte avec le serveur ».
- « Garder les fichiers des demandes closes » : 7, 30 (défaut) ou 90 jours (`inboxKeepDays`), grisé serveur éteint.
- « Les agents peuvent répondre aux demandes des autres agents » : la délégation (voir plus haut).
- Réglages › Avancé › Données locales : « Boîte « À toi » », ce que pèsent ses fichiers, « Vider les fichiers des demandes closes ».
- Le résumé des notifications de la boîte se règle dans Réglages › Notifications (voir plus bas).

## Sécurité

- Le texte et les fichiers d'une demande sont des **données d'agent**, qui peuvent relayer un tiers : lus par un autre agent que leur auteur (`get_ask`, `list_asks`, `wait_ask`, `get_ask_file`), ils sont marqués comme une donnée de tiers, jamais une consigne. Dans la fenêtre, le Markdown est assaini, rien de distant ne se charge sans clic, une image est un `data:` de la copie du cœur, un PDF n'exécute rien, un programme ne s'ouvre jamais.
- **Projet privé** : ses demandes ne sont trouvées que par ses agents, avec les mêmes mots qu'une demande qui n'existe pas ; ses agents n'en déposent dans aucun autre projet. La fenêtre les montre, marquées « Projet privé ». Une demande ne change de main que vers le chef de son propre projet.
- **Les journaux** : `app.log` ne dit que « ask a-12 deposited by agent e6e0… (choice, 3 options, 7 files) », « answered », « withdrawn », « read », « handed over to agent … », « kept with no living author »… ; l'activité du serveur MCP ne garde que le projet, la sorte et le numéro, jamais le texte, un chemin ni un nom de fichier.

## Limites

- `wait_ask` attend 240 s au plus (120 par défaut), comme `wait_ticket`, sous le délai d'un client HTTP.
- Un ticket attend une demande par `tickets` d'`ask_user` ; `after: ["ask:…"]` de `create_ticket` et la carte du ticket qui le dit ne sont pas encore là.
- Sur la 2.2 : un Word et un CSV sont des fichiers à ouvrir (les aperçus de l'éditeur sont sur la 2.3) ; pas de sélection de plusieurs demandes ni de gestes communs, pas de bulle de la première fois, pas de choix d'option à la voix. Ce que le reversement dans la 2.3 doit réunir est dans `docs/INBOX.md` § 18.

## Ce qu'Escouade y verse de lui-même (IN-3, ESC-513)

### Des vues sur l'état réel

La boîte montre aussi ce qui attend déjà l'utilisateur ailleurs, lu dans l'état à chaque lecture (`inbox_views`), jamais copié :

| Vue | D'où | Gestes dans la boîte | Groupe |
| --- | --- | --- | --- |
| « Autorisation », « Question », « Plan » · « l'agent s'est arrêté » | Les demandes bloquantes d'un agent dont le process tourne (pas celles de la vue Claude Code, que son terminal pose) | « Autoriser », « Toujours autoriser » quand la carte l'offre, « Refuser » avec un mot ; une question d'une seule question à options : ses options puis « Répondre », sinon « Répondre dans la conversation » ; un plan : « Approuver le plan », « Refuser » | « Tes agents se sont arrêtés », en tête, la plus ancienne d'abord |
| « Ticket à valider » | Un ticket « À tester » | « Valider » (comme la carte du Kanban : les lancements de test arrêtés, la question des fichiers non enregistrés de l'éditeur), « Renvoyer à l'agent » avec ce qu'il faut changer | « Aujourd'hui » |
| « Ticket bloqué » | Un ticket qui ne bouge plus seul (conflit, tests, refus) | Un conflit : « L'agent résout », « Annuler » ; « En cours » : « Reprendre » | « Aujourd'hui » |
| « Décision proposée » | Une décision proposée du registre | « Accepter » | « Plus tôt » |
| « Docs à partager » | Les changements des docs non partagés, comme l'onglet Docs les sait | « Partager » | « Plus tôt » |
| « Relecture » | Une PR dont l'hébergeur demande la relecture au compte lu, pas la sienne ni un brouillon, d'après la dernière liste lue (jamais lue pour la boîte) | « Ouvrir la PR » (un lien : rien ne se fait dans la boîte) | « Aujourd'hui » |

- **Synchronisées dans les deux sens** : réglée ailleurs (la conversation, le Kanban, Docs), une vue quitte la boîte aussitôt ; les faits qui changent une source (`agent.status`, `request.*`, `ticket.*`, `tests.result`, `decision.*`, `docs.*`) font relire la boîte, comme les événements des tickets, des agents, des docs à partager et des PR. Réglée dans la boîte (`inbox_view_act`), c'est le geste de sa source, par les mêmes fonctions, **sur la vue que l'utilisateur a lue** : réglée entre-temps, le geste est refusé (« C'est déjà réglé ailleurs : la boîte se met à jour. ») et rien n'est refait. Un geste qui ne va pas avec la vue est refusé.
- **« Plus tard » sur une vue** (`inbox_view_snooze`) : elle quitte les comptes jusqu'à l'heure dite (les heures de « Plus tard » de l'écran, sans « Choisir… »), sa source intacte, gardé dans l'état durable ; revenue à l'heure, ou « Remettre dans la boîte ». **Jamais une demande bloquante** : « Ton agent s'est arrêté pour cette demande : elle ne se reporte pas, elle attend ta réponse. »
- **Dans l'écran de la boîte** : les lignes des vues sont dans la file, dans ses groupes (les bloquantes dans « Tes agents se sont arrêtés », les tickets et les relectures dans « Aujourd'hui », les décisions et les docs dans « Plus tôt », celles qu'on a reportées dans « Plus tard ») ; une vue ouverte se lit dans le panneau, avec les gestes de sa source, et `J`, `K` passent de l'une à l'autre. Un agent arrêté sans vue (une suppression qu'il demande, un agent de la vue Claude Code) garde sa ligne qui mène à sa conversation.
- **Les comptes** : `inbox_views` rend `blocking` (les bloquantes, tous projets) et `total`. Le compteur de la tuile compte les demandes ouvertes, les agents arrêtés (comme « N en attente », jamais une demande bloquante deux fois) et les vues non bloquantes non reportées, tous projets. Une vue d'un projet privé est marquée comme telle.
- Serveur MCP éteint, `inbox_views` ne rend rien : ni ligne, ni compte, ni geste.

### Le résumé des notifications

Jamais une alerte par demande : un **résumé** des demandes asynchrones. La première après un calme attend 2 minutes, pour regrouper ce qui arrive ensemble, puis une notification du système : titre « <projet> · À toi » (« Escouade · À toi » quand plusieurs projets), corps « 2 demandes t'attendent dans À toi : un choix de design-tableau, un feu vert de chef-atlas. » (trois sortes au plus, puis « … »). Ensuite une au plus toutes les 30 minutes, ou toutes les heures, selon « Résumé de la boîte « À toi » » (Réglages › Notifications).

- **Jamais le titre ni le texte d'une demande** : sa sorte et son auteur seulement.
- **Ni carillon, ni barre des tâches qui clignote** ; les demandes bloquantes gardent leurs notifications d'aujourd'hui.
- Une demande n'est dite qu'**une fois** ; dite de nouveau seulement revenue de « Plus tard ». Une demande vue dans la boîte, laissée à un agent ou reportée ne compte pas. Fenêtre devant, rien n'est dit (la boîte et son compteur sont là) ; ce que l'utilisateur n'a pas vu est dit une fois la fenêtre quittée.
- Serveur MCP éteint, ou notifications du système éteintes, ou « Jamais (le compteur seul) » : rien ; ce qui est arrivé entre-temps n'est pas dit au rallumage.
- Un clic sur le résumé ramène la fenêtre sur la boîte, sur le projet quand il n'en dit qu'un (`inboxOpen`).
- `app.log` ne dit que « inbox summary told (2 requests, 1 projects) ».

### La délégation, une seule règle

« Les agents peuvent répondre aux demandes des autres agents » se juge par une seule règle (`delegation.rs`) pour les demandes de la boîte que l'utilisateur laisse à un agent et pour les demandes bloquantes (`answer_permission`, `answer_question`) : « Personne » refuse tout agent, chef compris ; « Seulement le chef d'escouade » laisse le chef du projet de la demande seul ; « Tous les agents », tous. Les mêmes agents sont refusés pour la même raison, chaque refus dit où le régler ; jamais un agent ne répond à sa propre demande.

### La consigne aux agents

Les instructions du serveur MCP (données à chaque harness, serveur allumé seulement) et la mémoire partagée du projet (`deposer-une-demande-a-toi`) disent quand déposer une demande plutôt que de l'écrire dans son bilan : un choix entre des options rendues (maquettes, directions), les choix d'une étude, un feu vert (publier, signer, supprimer, dépenser), un essai que seul l'utilisateur peut faire, un fichier à supprimer à la main ; les rendus joints à leurs options (clair et sombre), la recommandation et ce que la réponse débloque ; retirer ce qui n'a plus lieu d'être, jamais deux pour la même chose ; une question qui bloque reste une question ; jamais de secret.

### Ce qui se mesure

Si l'utilisateur a accepté les statistiques : `ask_created` (sorte, auteur, fichiers par tranche, recommandation, décision), `ask_withdrawn`, `ask_expired` (sorte), `inbox_view_settled` (source et geste), `inbox_summary_shown` (demandes par tranche). Jamais un texte, un nom ni un chemin.

### Limites de la 2.2

- L'Overseer (ses nouvelles, la voix, sa délégation), l'API distante et le téléphone, les réglages imposés, l'invitation d'équipe et la station de l'onboarding viennent avec la 2.3 (`docs/INBOX.md` § 18).
- Une relecture écrite par un agent qui attend d'être publiée n'est pas une vue ; une « Relecture » n'est connue qu'après une lecture de la liste des PR du projet.
- Les docs à partager sont ce que l'onglet Docs a lu en dernier : un projet dont les docs n'ont pas été lues depuis le démarrage n'en montre pas.

## À distance (ESC-520)

Reversée dans la 2.3, la boîte est servie par l'API distante du cœur. C'est le premier écran du téléphone.

- **Servies à un client distant** : `inbox_list`, `inbox_get`, `inbox_file`, `inbox_answer`, `inbox_snooze`, `inbox_delegate`, `inbox_reopen`, `inbox_seen`, `inbox_closing`, `inbox_weight`, `inbox_erase_closed_files`, et l'événement `inbox`. Chacune est dans la liste unique des commandes (`remoteapi/list.rs`, « Servies telles quelles » de `docs/REMOTE.md`).
- **Les octets d'un fichier** : `inbox_file_bytes` (une demande, un fichier) donne le fichier entier, 25 Mo au plus, en octets bruts. Ils partent en trames binaires, en parties au-delà de 16 Mo, jamais en base64. Son nom et son type sont dans `inbox_get`. `inbox_file` garde ses parts de 4 Mo en base64 pour la fenêtre.
- **Les fichiers d'une réponse** arrivent du client en corps bruts, par les fichiers joints (`attach_open`, `attach_chunk`, `attach_finish`), puis `inbox_answer` les nomme.
- **« Ouvrir » et « Afficher dans le dossier »** (`inbox_open_file`) agissent sur la machine du cœur. La commande change de machine en mode serveur : elle est refusée à distance (`not-served-yet`) jusqu'au mode serveur, qui la changera en téléchargement sur le poste.
- Ce qui reste (le serveur d'équipe, l'Overseer, les relectures de PR, le téléphone) est dans `docs/INBOX.md`, « Ce que la 2.2 ne livre pas ».

### L'écran et ce qu'Escouade y verse, à distance (ESC-540)

Reversés dans la 2.3 avec la 2.2.4, l'écran (IN-2) et les vues (IN-3) sont servis de même : une fenêtre connectée à un serveur (*Se connecter à un serveur*) lit et règle la boîte du serveur, comme un client distant.

- **Servies à un client distant** : `inbox_views` (les vues, lues dans l'état du cœur à chaque lecture, avec `blocking` et `total`), `inbox_view_act` (le geste de la source, sur la vue lue : « Accepter » une décision la fait accepter au registre du cœur), `inbox_view_snooze` (« Plus tard » sur une vue, gardé dans l'état durable du cœur) et `inbox_track` (ce que la fenêtre voit de la boîte, compté sur le cœur dont c'est la boîte). L'événement `inbox` dit au client que la boîte a bougé, une décision proposée comme une demande déposée.
- **« Enregistrer sous… »** (`inbox_save_file`) ouvre le dialogue du système de la machine du cœur et y écrit la copie : comme « Ouvrir », elle change de machine en mode serveur et est refusée à distance (`not-served-yet`), jusqu'à ce qu'elle devienne un téléchargement enregistré par le client (`inbox_file_bytes` sert déjà les octets). Ni l'une ni l'autre n'est « du poste seulement » : une telle commande part au cœur du poste, qui n'a pas la demande.
- **Le résumé des notifications** passe par l'hôte du cœur (`Host::notify`) : le bureau montre sa notification et ramène sa fenêtre sur la boîte ; un cœur sans fenêtre (`escouade-server`) la publie pour ses clients.

- **« Ouvrir » et « Afficher dans le dossier » d'une fenêtre sur un serveur** (ESC-540, sur le chemin d'ESC-538) : la fenêtre ne demande pas au serveur d'ouvrir le fichier (`inbox_open_file` y reste refusée) ; elle télécharge ses octets (`inbox_file_bytes`), en garde la copie sur le poste (`Téléchargements/Escouade/<espace>`, `received_save`), puis l'ouvre ou la montre là ; ouvert de nouveau, c'est la copie, sans nouveau téléchargement. « Enregistrer sous… » reste refusé à distance.

### La cible d'un ticket dans la boîte (ESC-552)

- Une vue « Ticket à valider » ou « Ticket bloqué » d'un ticket qui a sa propre cible, autre que celle du Kanban, le dit sur sa ligne, après sa sorte : « vers main » (`InboxItem::target`).
- « Valider » depuis la boîte suit la même garde que la carte (voir *Validation*) : une branche qui ne part pas de sa cible arrête la validation avant toute fusion, et le ticket devient une vue « Ticket bloqué ».
- Cette vue (`otherLine`, la branche d'où part celle du ticket) a un geste : « Viser main » (`inbox_view_act` `retarget`, avec la branche), qui donne cette cible au ticket et lève l'arrêt ; il redevient « Ticket à valider », « vers main » sur sa ligne. Ni « L'agent résout » ni « Annuler » : il n'y a pas de conflit.

## Le chef d'escouade tient la boîte à jour (ESC-557)

Une vue reste lue à sa source ; le chef d'escouade y pose une **marque**, gardée dans l'état durable par la clé de la vue, avec l'état de sa source quand il l'a posée. Demandé le 2026-11-20, après que l'utilisateur a validé depuis la boîte trois tickets que le chef vérifiait encore.

### « Je m'en occupe » (`claim_inbox_view`)

- Le chef prend en charge une vue qu'il est en train de traiter : un ticket « À tester » qu'il vérifie, un ticket bloqué, les demandes d'un agent arrêté (celles de maintenant ; une nouvelle arrive dans la boîte), une décision proposée. Avec une **note** courte (« vérification en cours, chaîne de tests ») : une ligne, 200 caractères au plus, jamais un secret.
- La vue **ne compte plus comme t'attendant** : elle quitte « Tes agents se sont arrêtés », « Aujourd'hui » et « Plus tôt », et le nombre de la tuile, de l'en-tête et de « il en reste » ; un agent arrêté dont le chef a pris toutes les demandes n'y compte plus non plus (« N en attente » de la barre d'état, qui dit les agents arrêtés, ne change pas).
- Elle **reste lisible** dans un groupe « Le chef s'en occupe » de la file, après « Plus tôt » : son titre, le nom du chef, sa note, depuis quand. Ouverte, le panneau dit « overseer-fable s'en occupe · 15:02 », la note entre guillemets, « Ça ne compte plus comme t'attendant. Tu peux toujours la régler toi-même. », et « Remettre dans la boîte ».
- **L'utilisateur peut toujours agir dessus** (valider, refuser, renvoyer…) : son geste demande d'abord « overseer-fable est en train de la vérifier · Sa note : « … ». La régler toi-même quand même ? », avec le geste pour bouton.
- Elle **tombe d'elle-même** quand sa source change (le ticket quitte « À tester » puis y revient, son blocage change, la décision est révisée), ou quand le chef la rend (`release: true`).

### « Réglé » (`settle_inbox_view`)

- Le chef coche comme réglée une vue qu'il a réglée **par un autre chemin**, avec une note qui dit comment (« fusionné à la main sur main en 79fe7aec ») : un ticket à valider ou bloqué (par exemple un ticket de la ligne 2.2 fusionné à la main, qui reste « À tester » jusqu'à ce que sa carte soit fermée), ou la décision proposée d'un ticket.
- La vue **sort de la boîte et passe dans sa trace**, dans « Répondu récemment » : « Réglé par overseer-fable : fusionné à la main sur main en 79fe7aec ». Ouverte, elle n'a plus ses gestes : « Réglé par overseer-fable · 15:10 », sa note, « Elle est sortie de ta boîte ; elle y revient d'elle-même si sa source change encore. », et **« Remettre dans la boîte »** (`inbox_view_reopen`), qui la rend à l'utilisateur. Sa source n'est pas touchée ; quand elle part (la carte fermée), la ligne part avec elle.
- Une vue réglée **revient d'elle-même** si sa source change encore (le ticket revient « À tester »).
- **Jamais ce qui reste le geste de l'utilisateur**, chaque refus dit pourquoi : partager les docs ; une relecture que l'hébergeur lui demande ; un feu vert (`gate`) ; une tâche ; une demande où un agent s'est arrêté (elle attend une réponse, pas une coche) ; une décision qui ne vient d'aucun ticket (l'utilisateur l'accepte). Une demande de la boîte se règle par son auteur (`withdraw_ask`, avec une note) ou par `answer_ask` quand l'utilisateur l'a laissée.

### Qui

- Le même réglage que les réponses, « Les agents peuvent répondre aux demandes des autres agents » (une seule règle, `delegation.rs`) : « Personne » refuse tout agent ; « Seulement le chef d'escouade », le chef du projet de la vue seul ; « Tous les agents », chacun. Claude hors Escouade, comme pour les permissions et les questions. Jamais un agent sur son propre ticket ni sur ses propres demandes.
- Serveur MCP éteint, rien ne se marque ; les marques gardées reviennent avec lui.
- `app.log` dit « inbox view ticketReview claimed », « settled », « released », « put back », jamais la note ; l'activité du serveur MCP garde le ticket nommé, jamais la note.

### Le retrait d'une demande

`withdraw_ask` prend sa raison comme **une note visible** de l'utilisateur dans la trace : « Retirée par chef-atlas : tranché dans la conversation du chef : B », dans « Répondu récemment » et dans le panneau de la demande ; sans raison, « Retirée par chef-atlas ». Une raison qui ressemble à un secret est refusée.

### La consigne aux agents

Les instructions du serveur MCP ajoutent : retirer une demande avec une note qui dit comment elle a été tranchée ; chef d'escouade d'un projet, tenir la boîte à jour : prendre en charge un ticket « À tester » qu'on vérifie (`claim_inbox_view`, avec ce qu'on fait), cocher comme réglé ce qu'on a réglé par un autre chemin (`settle_inbox_view`, avec comment), jamais ce qui reste le geste de l'utilisateur.

### Les commandes de la fenêtre

`inbox_views` rend la marque de chaque vue (`mark` : `kind` `claimed` ou `settled`, `note`, `byName`, `at`) et ne compte pas les vues marquées dans `total` ; `inbox_view_reopen` remet une vue marquée dans la boîte. Sur la 2.2, l'API distante n'existe pas encore : au reversement dans la 2.3, `inbox_view_reopen` y est servie avec les autres commandes de la boîte (`docs/INBOX.md` § 18).

## Le filtre par type (ESC-578)

Demandé le 2026-11-21 : « Dans le "À toi", ajoute le fait de pouvoir filtrer par type de demande. »

### Ce qu'il offre

- En haut de la file, sous « 3 traitées aujourd'hui · il en reste 9 » : un groupe « Filtrer par type » (“Filter by type”), « Tout » (“All”), puis un bouton par type qui attend dans la portée, avec son nombre (« Autorisations 3 », « Feux verts 1 »), dans un ordre fixe : Autorisations, Questions, Plans, Choix, Feux verts, Tâches, Tickets, Décisions, Relectures, Docs (“Permissions”, “Questions”, “Plans”, “Choices”, “Go-aheads”, “Tasks”, “Tickets”, “Decisions”, “Reviews”, “Docs”). Les boutons passent à la ligne dans la largeur de la file (300 px, 236 sous 1100 px).
- Un type sans rien qui attende n'apparaît pas, sauf s'il est choisi : il reste à 0 pour qu'on puisse l'ôter. Le filtre ne s'offre pas quand un seul type attend et que rien n'est choisi.
- **Les regroupements** : « Autorisations » réunit l'autorisation d'un agent arrêté et la suppression qu'un agent demande (un oui ou non sur ce que l'agent ferait) ; « Questions », la question d'un agent arrêté, une demande `answer` et un `confirm` sans `gate` (ce que l'agent demande, quelle que soit la forme de la réponse) ; « Tickets », un ticket à valider et un ticket bloqué (les deux se règlent sur le ticket). « Feux verts » (`confirm` avec `gate`) reste à part : toi seul y réponds.
- **Plusieurs à la fois** : chaque bouton s'allume ou s'éteint ; « Tout », « Effacer » (au bout des boutons, filtre actif) ou « Tout montrer » ôtent le filtre d'un geste.
- **Gardé sur ce poste**, d'une ouverture à l'autre et d'un lancement à l'autre (le stockage local de la fenêtre) ; le téléphone garde le sien.

### Ce qu'il change

- Les groupes et leur ordre restent ; seules les lignes des types choisis y restent, « Plus tard », « Le chef s'en occupe » et « Répondu récemment » compris ; un groupe vidé par le filtre se tait. `J`, `K` et « 2 sur 5 » parcourent les lignes montrées. La demande ouverte, cachée par le filtre, laisse la place à la première montrée.
- **Les comptes ne changent pas** : la tuile « À toi », le titre de la file, la jauge et « il en reste 9 » disent toujours tout ce qui attend.
- **Ce qu'il cache se dit discrètement**, sous lui : « 2 autres demandes masquées par le filtre · Tout montrer » (“2 more requests hidden by the filter · Show all”) ; une demande d'un type caché qui arrive y ajoute un, et un lecteur d'écran l'entend (région `status`). Quand le filtre cache tout ce qui attend, le panneau dit « Rien de ce type ne t'attend » ; « 3 autres demandes attendent, masquées par le filtre. » ; « Effacer le filtre » — jamais « Rien ne t'attend ».

### Clavier et lecteur d'écran

- Chaque bouton dit son état (`aria-pressed`), le groupe son nom ; `Tab` y entre, `←`, `→`, `Début`, `Fin` vont d'un bouton à l'autre, `Espace` ou `Entrée` choisit. Aux deux tailles de fenêtre (1440 × 900, 1024 × 728), rien ne déborde.

### Partagé avec l'app mobile

- Les types, leurs regroupements, les nombres, ce qui est gardé et caché vivent dans un module de la fenêtre sans son état (`src/lib/inbox/filter.ts`), que l'app mobile reprend (MO-5, ESC-571). Le filtre ne passe pas par le cœur : rien de neuf à servir par l'API distante.

## Ouvrir une pièce jointe sans rien lancer (ESC-628)

- « Ouvrir » d'une pièce jointe suit la règle de la fenêtre (voir *Fenêtre › Sécurité de la fenêtre*) : jamais un fichier que le système lancerait, installerait, monterait ou suivrait en l'ouvrant (un programme, un script, un installeur, une image de disque, un raccourci), ni une page ou un SVG. Ceux que la fenêtre reconnaît à leur nom n'ont que « Afficher dans le dossier » ; ceux que le cœur refuse (marqué exécutable, par exemple) le disent dans un toast, avec la raison et le bouton « Afficher dans le dossier ». La liste de la fenêtre (`src/lib/runnable.ts`) est celle du cœur : un test les tient égales.
- **Sous macOS**, chaque copie d'une pièce jointe (`<données>/asks/<id>/files/…`, celles d'un agent comme celles de la réponse) porte l'attribut `com.apple.quarantine` : Gatekeeper la juge à son ouverture. Sous Windows, une copie faite d'un fichier qui porte la marque du web (`Zone.Identifier`) la garde.

## Un ticket à accepter (ESC-645)

- Une vue « Ticket à accepter » : un ticket « À faire » d'un Kanban d'équipe, assigné à l'utilisateur, dont le texte vient d'un autre membre et qu'il n'a pas accepté (voir *Le Kanban d'équipe*). Sa ligne : « DEM-4 · Nettoyer le cache · texte de Bob Durand », dans « Aujourd'hui », l'icône de l'équipe ; son type pour le filtre : « Tickets ».
- Ses gestes : « Accepter » (comme la carte) et « Voir dans le Kanban ». Sous eux : « Son texte vient de Bob Durand : accepté, le pilote auto peut le lancer avec tes droits et tes comptes. Lis-le d'abord dans le Kanban ; changé ensuite par un collègue, il revient ici. »
- **Sur la vue que l'utilisateur a lue** : sa clé porte l'empreinte des écritures du collègue ; un texte changé entre-temps fait une autre vue, et « Accepter » sur l'ancienne est refusé.
- Le chef d'escouade ne la prend pas en charge et ne la coche pas : « Accepter le texte qu'un collègue a écrit reste le geste de l'utilisateur ».

## Une tâche programmée à accepter (ESC-647)

- Une vue « Tâche programmée à accepter » : une tâche programmée qu'un agent a créée ou modifiée, qui ne lance rien avant l'accord de l'utilisateur (voir *Tâches programmées › Une tâche programmée par un agent*). Sa ligne : « Claude programme « Maintenance » », dans « Aujourd'hui », l'icône d'horloge, l'agent et son ticket ; son type pour le filtre : « Feux verts » (un accord que l'utilisateur seul donne).
- Son panneau montre ce qu'elle fera : le projet, le rythme (en mots et son expression cron), la consigne, les critères. Son geste : « Accepter ». Sous lui : « L'agent Claude l'a programmée : elle ne lance aucun ticket avant ton accord. Acceptée, elle lance un agent à chaque échéance, avec tes comptes ; changée ensuite par un agent, elle revient ici. »
- **Sur la vue que l'utilisateur a lue** : sa clé porte l'empreinte de la tâche (projet, titre, consigne, critères, boucles, harness, compte, modèle, effort, rythme, rattrapage) ; une tâche changée entre-temps fait une autre vue, et « Accepter » sur l'ancienne est refusé. L'empreinte est vérifiée encore sous le verrou des tâches.
- Le chef d'escouade ne la prend pas en charge et ne la coche pas : « Accepter une tâche programmée par un agent reste le geste de l'utilisateur ».
