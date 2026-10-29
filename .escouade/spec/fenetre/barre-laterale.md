---
title: "Barre latérale"
subject: "Fenêtre"
sources: ["docs/SPEC.md","src-tauri/src/core.rs","src-tauri/src/engine/claude/mod.rs","src-tauri/src/worktrees.rs"]
tickets: ["ESC-204","ESC-73","ESC-75","ESC-160","ESC-19"]
decisions: []
compared: null
---

## Ce que c'est

La colonne d'un projet, à droite de la barre des projets. Elle liste les agents du projet en cartes : ce qu'ils font, ce qu'ils attendent, ce qu'ils coûtent. On peut leur répondre depuis la liste sans ouvrir leur conversation. Elle porte aussi les commandes de lancement du projet, ses terminaux et, en pied, son dossier et sa branche. Le sélecteur « Agents | Kanban » en haut de la barre passe au Kanban (voir *Kanban*).

## Comportement

### Largeur

- Le bord droit se tire comme celui de la barre des projets, de 220 à 480 px. Au clavier : ←/→ par 16 px, Début, Fin. Un double clic rend la largeur par défaut, 276 px.
- La largeur est gardée, la même pour toute l'app, sur Agents comme sur Kanban. La barre ne prend jamais plus que la largeur de la fenêtre, moins la barre des projets, moins 672 px.
- Sous 250 px, « Ctrl N » quitte le bouton « Nouvel agent », et les petites barres du plan et des boucles quittent les cartes ; leurs mots restent.

### Agents

- En tête : le nombre d'agents et « Nouvel agent ». À côté, un menu :
  - « Nouvel agent sur une branche… » (voir *Git & fichiers*) ;
  - « Nouvel agent sans worktree » dans un projet à « Un worktree par agent », « Nouvel agent dans un worktree » sinon : l'inverse du réglage, pour cet agent seul ;
  - « Nouvel agent depuis une autre branche… » (voir *Git & fichiers*).
- Un agent naît sous le nom `agent-N`. Après son premier message, le modèle des textes rédigés lui donne un nom court en slug (voir *Langues*). Il se renomme par double-clic.
- Une fois une pull request ouverte sur sa branche, la branche garde son nom quand l'agent est renommé (voir *Pull requests*).

### La carte d'un agent

- Statut : Prêt, En cours, Question ou Terminé. « Attend », ou « Réveil à 14:20 », quand son tour est fini mais que ce qu'il a laissé continue (voir *Avancée d'un plan*).
- Nom, puis modèle · durée active.
- Pendant un tour, ce que fait l'agent : un verbe puis ce sur quoi il agit (« Modifie src/auth/jwt.ts », « Lance npm test », « Lit », « Cherche », « Réfléchit »). Les mots sont les mêmes d'un harness à l'autre ; Codex dit « Lit » ou « Cherche » pour une commande qui ne fait que lire un fichier ou chercher.
- Tour fini : « Tour fini · 1 sous-agent, 1 tâche de fond » si ce qu'il a laissé continue, sinon ce qu'il a changé (« 2 fichiers modifiés »).
- À droite : tokens · coût. « ≈ » pendant un tour, et pour un harness qui ne dit pas son coût exact (Codex). Pour Copilot CLI : crédits · coût (« 50 crédits · 0,50 $ »).
- Agent d'un ticket : « ▸ ATL-42 · boucle 2/5 », puis « · à tester » une fois le ticket à tester (voir *Agent d'un ticket*).
- Sous le nom : la ligne d'état que l'agent a donnée par `report_progress`, s'il en a donné une (voir *Serveur MCP*).
- Avec une liste de tâches : une mini-barre et « Plan 2/7 », ou « Plan 2/7 · 3 sous-agents » (voir *Avancée d'un plan*).
- La puce « N en fond » tant que des tâches de fond tournent.
- Sous la carte, dans cet ordre :
  - « Sous-agents de <agent> », « Tâches de fond de <agent> » et « Agents pilotés par <agent> », tant qu'il y en a (voir *Avancée d'un plan*) ;
  - pour un agent ordinaire à worktree, ses pull requests (trois numéros au plus) et leurs deux corrections (voir *Pull requests*).

### Répondre depuis la liste

- La carte de l'agent qui attend montre sa première demande, quand elle se lit en entier sur la carte, avec ses choix numérotés :
  - une ou plusieurs questions à choix, une à la fois (« Question 1 sur 2 »). Les choix sont partagés avec la carte de la conversation et partent avec la dernière question. Le message d'un formulaire de serveur MCP est au-dessus ;
  - une autorisation, pour un outil dont le résumé dit tout, en 3 lignes et 160 caractères au plus, avec ce que dit le harness (sa raison, le « bac à sable » de Codex, le motif que garderait « Toujours autoriser » d'OpenCode ; 240 caractères au plus). Boutons : « Autoriser », « Toujours autoriser » (Claude Code, OpenCode) ou « Pour cette session » (Codex, Copilot CLI), « Refuser ».
- Le reste montre son début et « Répondre dans la conversation », qui ouvre l'agent : une question à écrire, une question à choix multiple, un plan, une demande coupée ou trop longue, une demande que Claude Code refuserait par défaut.
- Un clic, ou les touches `1`, `2`…, répondent à l'agent dont la carte a le focus, sinon au premier de la liste qui peut l'être.
- Les touches ne marchent que hors d'un champ : ni dans un terminal, l'éditeur, un menu ou une boîte de dialogue, ni sur le Kanban, ni avec Ctrl, Alt ou Cmd, ni maintenues. Elles ne font rien pendant 500 ms après qu'une demande s'affiche.
- La conversation ouverte ne change pas. La carte de l'agent qui repart brille en vert un instant, un toast dit la réponse (« tests-e2e : « Oui » »), et l'agent reprend sa place dans la liste.
- `Ctrl+J`, ou le bouton « N en attente », donne le focus à la carte du prochain agent en attente ou à voir, dans tous les projets, sans l'ouvrir. `Entrée` l'ouvre.

### Menu d'un agent

Au clic droit sur sa carte :

- Renommer.
- « Dupliquer la conversation » (ci-dessous).
- Archiver : l'agent passe dans l'historique du projet, d'où il se rouvre.
- « Lancer… » : pour un agent à worktree non archivé, dans un projet qui a des commandes de lancement (voir *Lancement* ci-dessous).
- « Préparer le lancement » : pour un agent à worktree, pas tant que son ticket est « En cours » (voir *Lancement de test*).
- « Intégrer <base> » : pour un agent à worktree non archivé (voir *Git & fichiers*).
- « Ouvrir dans l'éditeur ».
- « Ouvrir un terminal » : dans son worktree, sinon dans le dossier du projet, avec le premier shell détecté ; nommé « <shell> · <agent> ». Pas pour un agent archivé.
- « Ouvrir une PR… », ou « Voir la PR #n » tant qu'une est ouverte (voir *Pull requests*).
- « Activer le remote control » ou « Désactiver le remote control » (ci-dessous).
- Supprimer : arrête son process et supprime son worktree, après confirmation s'il n'est pas mergé.

### Dupliquer la conversation

- Pas pour un agent archivé. Désactivé pendant un tour (« Attends la fin de son tour. ») ; une demande faite quand même est refusée (« Attends la fin du tour de <nom> pour le dupliquer. »).
- Crée « <nom> (copie) », puis « (copie 2) »…, dans le même projet, avec le même modèle, effort et mode, et la sélectionne.
- La copie montre la conversation de l'original. Claude Code la démarre avec `--resume=<session de l'original> --fork-session --resume-session-at=<dernière entrée>` : la dernière réponse de l'assistant ou le dernier résultat d'outil du fil principal, noté dès qu'il change (donc même après un arrêt brutal en plein tour). Dès son premier tour, la copie a sa propre session : l'original n'est jamais touché, et ses tours suivants ne sont pas dans la copie. Une copie de copie qui n'a encore rien fait reprend le même point.
- Sans point noté (l'agent vient de compacter sa conversation, ou n'a fait aucun tour depuis la 1.6), la copie reprend toute la session telle qu'elle est au moment de la copie. Point introuvable, parce que l'original a compacté depuis : « Point de copie introuvable dans la session de l'original : Claude reprend toute sa conversation, jusqu'à maintenant. » Session de l'original disparue : traitée comme une reprise perdue, avec une nouvelle session.
- Avec un worktree, la copie a le sien, sur la branche `escouade/<nom de l'original>-copie` (`-copie-2`…, 40 caractères au plus), partie du commit où en est l'original. Elle a la même branche de base, les fichiers copiés et la préparation d'un nouvel agent. Sa consigne (`--append-system-prompt`) lui dit : « Cette conversation a été copiée depuis un agent qui travaillait dans <dossier>. Tu travailles maintenant dans <dossier de la copie> : ne lis et n'écris que dedans. » Si l'original a des modifications non commitées, une notice le dit : « Les modifications non commitées de <original> ne sont pas dans cette copie. » Si le worktree ne peut pas être créé, aucune copie n'est faite.
- Sans worktree, la copie travaille dans le même dossier que l'original.
- Ne passent pas à la copie : le ticket, son protocole et ses ports (un agent de ticket se copie en agent ordinaire), les coûts, le remote control allumé à la main, les recettes de lancement et leurs approbations. Un réglage de remote control allumé couvre la copie comme tout nouvel agent, sauf si l'original est mis à part : la copie l'est aussi.

### Remote control

Le remote control rend la session d'un agent Claude Code accessible depuis claude.ai et l'app Claude mobile.

- Éteint par défaut. Il s'allume pour un agent, au clic droit, ou pour tous les agents d'un projet ou de l'app, par un réglage.
- La session apparaît sur claude.ai sous le nom « projet · agent » ; pour un agent de ticket, son nom vient du ticket.
- Escouade lance tout de suite le process de chaque agent concerné, au plus 3 à la fois. Ce process reste lancé : il démarre avec l'app et n'est jamais arrêté pour inactivité. Après un redémarrage, il retrouve la même session distante : le lien reste valable.
- Renommer un agent renomme sa session distante. Renommer un projet renomme celles de ses agents en vue Escouade ; l'interface d'un agent en vue Claude Code garde l'ancien nom jusqu'à son prochain lancement.
- Les messages envoyés depuis claude.ai s'affichent dans la conversation (« depuis claude.ai »). L'écho des messages envoyés depuis l'app est écarté.
- Sur la carte, une icône d'ondes : connecté ou en attente. Au clic droit : « Ouvrir sur claude.ai », « Copier le lien ».
- Couper le remote control (clic droit, réglage éteint, archivage, suppression) ferme la session distante, qui est archivée sur claude.ai. Si aucun process de l'agent ne tourne à ce moment, elle est fermée au prochain démarrage d'un process de l'agent, ou reprise avec le même lien si le remote control est revenu entre-temps. Une fermeture qui échoue est écrite dans le journal et retentée de même. Une coupure suivie d'un rallumage rapide ne mêle pas leurs effets.
- Restaurer un agent archivé lui rend le remote control si un réglage le couvre, pas s'il était allumé à la main.
- « Remote control indisponible : … » (le lien n'a pas pu s'établir) n'est dit qu'une fois par agent et par lancement d'Escouade, jusqu'à un lien réussi ou un rallumage à la main.

#### Réglages et sources

- Pour toute l'app : Réglages › Harness, groupe « Remote control », juste avant « Serveur MCP ». Interrupteur « Remote control pour tous les agents » (éteint par défaut), avec la description : « Chaque agent de chaque projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l'app Claude. Escouade lance tout de suite le process claude de chacun, qui reste lancé. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu'à son prochain lancement. Pour mettre un agent à part : clic droit, « Désactiver le remote control ». »
- Pour un projet : Réglages › Projets, groupe « Remote control », après « Identité ». Interrupteur « Remote control pour les agents de ce projet » (éteint par défaut), avec la même description pour les agents du projet (« Chaque agent de ce projet, existant ou nouveau… »).
- Les deux passent par le brouillon des réglages : ils marquent leur onglet comme modifié et s'enregistrent avec « Enregistrer ».
- Jamais deux interrupteurs qui se contredisent : quand le réglage de l'app est allumé, celui du projet se montre allumé et grisé, avec « Activé pour tous les projets dans Réglages › Harness » (dit aussi aux lecteurs d'écran). La valeur propre du projet est gardée ; un clic sur l'interrupteur grisé ne change rien. Le réglage de l'app éteint, même dans le brouillon, le projet retrouve son état.
- Allumé, un réglage vaut pour les agents existants non archivés de son périmètre et pour chaque nouvel agent, agents de tickets et copies compris. Éteint, il retire le remote control aux agents qui le tenaient de lui, sans toucher à ceux allumés à la main ni à ceux qu'un autre réglage couvre encore.
- D'où il vient : l'infobulle de l'icône d'ondes finit par « · activé à la main », « · activé par le réglage du projet » ou « · activé par le réglage de tous les agents ». Le clic droit reste possible quelle que soit la source.
- Mettre un agent à part : « Désactiver le remote control » au clic droit retire toujours le remote control à l'agent et le met à part, même si un réglage le couvre. Aucun réglage ne le lui rend, ni en s'allumant ensuite ni au redémarrage d'Escouade, jusqu'à « Activer le remote control ». Une copie d'un agent mis à part l'est aussi. Le ticket d'un agent mis à part le reste : renvoyé en « À faire » puis relancé, il crée un agent lui aussi mis à part, sauf rallumage à la main.
- Projet fermé (voir *Fermer un projet*) : son historique garde son réglage et, pour chaque agent, d'où lui venait le remote control et sa mise à part. Un projet fermé ne lance aucun agent, même si le réglage de l'app s'allume. Rouvert, ses agents reçoivent les réglages comme au démarrage d'Escouade : un agent mis à part le reste, les autres reprennent leur session distante, avec le même lien.
- Statistiques d'usage : l'événement `remote_control_enabled` porte `from` : `agent` (clic droit), `project` ou `global`. Rien d'identifiant.

#### En vue Claude Code

- Mêmes règles. L'interface d'un agent en remote control est lancée avec `--remote-control=<projet · agent>`, en un seul argument : un nom qui commence par « - » n'est pas pris pour une option. Claude Code affiche alors le lien de la session. Quand l'interface reprend sa session (redémarrage d'Escouade, relance), il rattache la même session distante, avec le même lien. Une interface fermée archive sa session distante.
- Au démarrage d'Escouade, l'interface d'un tel agent est lancée en fond, sans être ouverte, pour qu'il soit joignable. Ouvrir l'agent montre cette interface déjà lancée.
- Ces lancements, comme ceux d'un réglage, se font au plus 3 à la fois. Chacun garde sa place jusqu'à ce que l'interface soit prête, 15 s au plus.
- Passer un agent en vue Claude Code ferme la session distante de sa vue Escouade ; son interface ouvre la sienne. Revenir en vue Escouade en ouvre une nouvelle, avec un nouveau lien. Le passage en vue Claude Code n'est pas refusé quand le remote control est allumé.
- Escouade ne peut ni allumer ni couper le remote control d'une interface déjà ouverte : un changement vaut au prochain lancement de l'interface. Après un changement au clic droit, un toast le dit : « Pris en compte au prochain lancement de son interface Claude Code ; d'ici là, c'est sa commande /remote-control qui le fait. » Pour un réglage, c'est sa description qui le dit.
- Une interface lancée en fond et jamais ouverte s'arrête quand son remote control s'éteint, sauf en plein tour.
- Une interface qu'Escouade arrête pour de bon (retour en vue Escouade, archivage, suppression, interface jamais affichée dont le remote control s'éteint) est fermée en douceur, pour que Claude Code archive sa session : deux Ctrl+C, le second 700 ms après le premier, puis un arrêt forcé au bout de 5 s. Un démarrage de l'agent demandé pendant ce temps attend que l'ancienne interface soit partie : jamais deux process sur une session. À la fermeture d'Escouade, l'interface n'est pas fermée ainsi, pour que le lien revienne au prochain démarrage.
- Menu de l'agent : « Activer / Désactiver le remote control » ; « Ouvrir sur claude.ai » et « Copier le lien claude.ai » une fois l'interface connectée. Le lien est lu dans le transcript de l'interface.
- Infobulle de l'icône d'ondes :
  - « Remote control : aucune interface lancée, pas joignable pour l'instant (ouvre l'agent pour la lancer) » ;
  - « Remote control : connexion… (accessible depuis claude.ai et l'app Claude) » ;
  - « Remote control : connecté (accessible depuis claude.ai et l'app Claude) » ;
  - « Remote control : au prochain lancement de son interface Claude Code », quand l'interface qui tourne a été lancée sans le remote control ;
  - « Remote control : pas ouvert par Claude Code (compte sans abonnement claude.ai, réseau, proxy ou règle de l'organisation) », quand l'interface tourne sans que Claude Code ait ouvert la session distante (ci-dessous).
- Remote control éteint pour Escouade mais encore actif dans l'interface ouverte : l'icône reste, barrée et dans la couleur d'avertissement, avec « Remote control encore actif dans l'interface ouverte, jusqu'à son prochain lancement (sa commande /remote-control le coupe) ». Seulement si l'interface a vraiment eu son lien.
- Pas ouvert par Claude Code : Claude Code n'écrit pas dans le terminal qu'il refuse le remote control ; Escouade le déduit. Deux cas donnent un avis dans la conversation, dit une fois par agent et par lancement d'Escouade :
  - le transcript de l'interface dit l'état du lien sans adresse, alors qu'elle est lancée avec l'option et qu'aucun lien n'a été vu : « Remote control indisponible : Claude Code n'a pas pu ouvrir la session distante (« <ce qu'il dit, 200 caractères au plus> »). », sans la parenthèse s'il ne dit rien ;
  - rien ne prouve un lien 20 s après que Claude Code a inscrit l'interface dans son registre des sessions, l'agent tenant encore le remote control : « Remote control indisponible : l'interface de Claude Code ne l'a pas ouvert (compte sans abonnement claude.ai, réseau, proxy ou règle de l'organisation). »
  Le remote control reste en vigueur pour l'agent : son prochain lancement réessaie. Un lien qui arrive plus tard remet « connecté » et permet de redire l'avis. Une interface lancée sans l'option ne dit rien d'un lien qui manque.
- Repli : une interface lancée avec le remote control, jamais montrée, qui se ferme dans les 10 s sans que l'utilisateur ait fermé son terminal, donne la notice « Remote control indisponible : l'interface de Claude Code s'est fermée juste après son lancement avec le remote control ; elle est relancée sans lui. » Elle est relancée tout de suite en fond, sans l'option, et les lancements suivants de cet agent se font sans l'option, jusqu'au prochain démarrage d'Escouade ou un rallumage à la main.
  - Une interface qu'une fenêtre a montrée n'est jamais un échec de l'option, quel que soit son délai : l'utilisateur a pu la finir lui-même (« No, exit » sur la boîte de confiance, Échap, `/exit`, deux Ctrl+C). Ni repli ni notice ; « Relancer » reste à lui.
  - Une session introuvable n'est pas non plus un échec de l'option (voir *Pilotage de Claude Code*).
  - Le repli et les lancements en fond suivent la garde du lancement automatique : après deux arrêts de suite dès le lancement, plus aucun lancement automatique, seulement « Relancer ». Une interface qui quitte aussitôt n'est donc jamais relancée deux fois d'elle-même.
- Interface prête : elle prend ce qu'on tape, donc elle n'est ni sur l'écran du premier lancement d'un compte, ni sur la boîte de confiance du dossier (« Quick safety check », dont le choix par défaut est « No, exit » : un texte tapé y choisirait une option). Elle est prête dès que l'un de ces signes est là :
  - une fenêtre l'a montrée ;
  - son transcript, depuis son lancement, dit l'état de son lien distant ;
  - Claude Code l'a inscrite dans son registre des sessions (`<dossier de config du compte>/sessions/<pid>.json`, comme interface `cli`). Il l'y écrit en moins de 2 s une fois prête, jamais sur l'écran du premier lancement ni sur la boîte de confiance.
- Message refusé : tant que l'interface n'est pas prête, un message de la fenêtre ou du serveur MCP est refusé : « L'interface de Claude Code de cet agent n'est pas encore prête (premier lancement ou dossier à approuver) : ouvre l'agent pour la voir. » Cela vaut pour toute interface, celle relancée sans l'option comprise.
- Fenêtre rechargée : une fenêtre rechargée, ou qui ouvre l'agent après un lancement en fond, se rattache à l'interface qui tourne au lieu de la relancer : sa sortie est rejouée, sans les requêtes du terminal, et ses modes reposés. « Relancer » attend l'arrêt de l'ancienne interface avant d'ouvrir la nouvelle.

### Lancement

La section entre Agents et Terminaux : les commandes qui lancent le projet pour le développer.

- Elles se configurent dans l'onglet « Projets » des réglages, groupe « Lancement », qu'ouvrent le ⚙ de la section et « Réglages du projet… ». Chacune a un nom, une ligne de commande, un shell parmi ceux détectés et un sous-dossier facultatif ; elles sont enregistrées avec le projet.
- Sans commande, la section propose « Configurer » et « ✦ Proposer des commandes ». Le second ouvre ces réglages sur le groupe « Lancement » et lance la lecture du projet.
- Chaque commande tourne dans son propre terminal, sans saisie : `pwsh -Command …`, `bash --login -c …` ou `wsl -- bash -lc …`. Un clic sur la commande affiche son log dans la zone principale, en lecture seule, gardé d'un lancement à l'autre avec un séparateur « relancé à HH:MM ».
- Statut en direct : prêt, en cours, arrêté (depuis l'app), terminé (code 0), planté (code N, avec une notification).
- Boutons : lancer ▶, relancer ⟳, stopper ■ (arrête aussi les programmes lancés par la commande), « Tout lancer », « Tout arrêter ».
- Fermer le projet arrête ses commandes. Elles ne survivent pas à un redémarrage d'Escouade.

#### Proposer des commandes

- « ✦ Remplir automatiquement », première rangée du groupe « Lancement », fait lire le projet par le modèle des textes rédigés, qui ne modifie ni n'exécute rien (voir *Langues*). Il lit les manifestes, les scripts, docker-compose et le README, et propose les processus qu'on lance pour développer : serveurs de dev, watchers, workers, base locale par docker compose… Chacun a un nom court, son sous-dossier et sa commande pour le shell par défaut. Jamais d'installation ni de tests.
- 8 commandes au plus. Une commande est écartée quand son sous-dossier sort du projet ou n'existe pas, ou quand la commande ou le dossier ne tient pas sur une ligne simple : retour à la ligne, caractère de contrôle ou invisible, 24 blancs de suite ou plus, plus de 300 caractères. Cette règle est aussi donnée au modèle. Un nom est nettoyé de ses caractères invisibles ; un nom absent ou vide est tiré de la commande.
- La proposition s'affiche d'abord en entier sous la rangée, dans la région « Commandes de lancement proposées » : « Proposition de Claude : relis chaque commande en entier. Elles remplacent celles de la liste. » Chaque commande est en police à chasse fixe, jamais coupée, ses caractères invisibles écrits en clair et dans l'ordre où le shell la lit, avec son shell et son dossier.
- « Remplacer les commandes » les met à la place de celles du brouillon, à relire avant « Enregistrer » (« Annuler » les jette). « Ignorer » laisse les commandes telles quelles. La proposition reste en attente d'un onglet à l'autre ; une nouvelle ouverture des réglages l'oublie.
- Pendant la lecture, le bouton affiche « Claude lit le projet… ». À la fin, le focus passe à la proposition s'il était resté sur le bouton. Chacun des deux boutons est décrit par le texte de sa rangée pour les lecteurs d'écran.
- Toasts : « 2 commandes proposées, 1 écartée (caractères invisibles ou trop longue) : relis-les avant d'enregistrer. » ; toutes écartées : « Claude a proposé des commandes illisibles : aucune n'a été gardée. » ; réponse illisible : « Claude n'a pas proposé de commandes lisibles. » ; rien à lancer : « Claude n'a trouvé aucune commande à lancer pour ce projet. »

#### Dans le worktree d'un agent

- Dès qu'un agent non archivé du projet a un worktree, ▶ ouvre un menu : « Dans le projet » ou « Dans le worktree de <agent> ». Au clic droit d'une carte, « Lancer… » liste les commandes.
- La commande tourne dans le même sous-dossier, pris dans le worktree et confiné à lui. Elle est refusée, avec sa raison, quand ce dossier n'y est pas (« Le dossier « web » de « Front » n'existe pas dans le worktree de « refacto-auth » (sa branche ne l'a pas, ou pas encore). »), et pour un agent archivé, sans worktree, au worktree disparu, ou dont le ticket est en validation.
- Son terminal s'appelle « Front · refacto-auth », dans un groupe au nom de l'agent, avec son propre état et son propre log.
- Elle reçoit `ESCOUADE_PORT_BASE` et `ESCOUADE_PORT_END`, le bloc de ports de l'agent, réservé alors s'il n'en a pas.
- Si la même commande tourne ailleurs (dans le projet ou un autre worktree), Escouade demande d'abord : « Arrêter l'autre et lancer » ou « Lancer quand même ». « Tout lancer », dans le projet, nomme d'abord celles qui tournent dans un worktree : « Les arrêter et tout lancer » ou « Lancer quand même ».
- Elle est suivie avec les lancements de test de l'agent (voir *Lancement de test*) et s'arrête avec eux : archivage, suppression, validation, fermeture du projet. Une nouvelle recette de lancement ne la touche pas.

### Terminaux

- Menu `+` : PowerShell 7 (cherché dans le PATH, Program Files et l'alias du Microsoft Store ; à défaut, Windows PowerShell 5.1), Git Bash, WSL (Ubuntu).
- Un vrai terminal : autocomplétion du shell (Tab, PSReadLine, bash-completion), historique, Ctrl+R, couleurs ANSI, programmes plein écran (vim, less, htop), redimensionnement, copier-coller, recherche, liens cliquables (voir *Fenêtre*).
- Il s'ouvre dans le dossier du projet. « Dans le worktree de… », au bas du menu `+`, liste les agents du projet qui ont un worktree (pas les archivés, dans l'ordre de la barre latérale ; ni l'entrée ni son séparateur sans un tel agent) et ouvre le terminal dans le worktree choisi.
- « Ouvrir un terminal » du menu d'un agent (clic droit sur sa carte, ou « … » de l'en-tête de sa conversation, voir *Conversation*) l'ouvre dans le worktree de l'agent. « Ouvrir un terminal ici », dans l'arborescence de l'éditeur, l'ouvre dans un dossier de la source affichée, confiné à elle ; un worktree disparu est refusé.
- Les terminaux ne survivent pas à un redémarrage d'Escouade.
- Saisie : chaque terminal écrit ses frappes dans l'ordre, sans attendre les autres. Un programme qui ne lit pas son entrée ne bloque que son terminal : ni la fenêtre, ni la saisie, le redimensionnement ou la fermeture des autres terminaux. Au-delà de 32 Mo en attente, la saisie de ce terminal est refusée (« le terminal ne lit plus son entrée ») ; l'écriture se fait par 256 Ko au plus. Si l'écriture échoue parce que le programme est parti, le journal le dit (« its input can no longer be written »). Fermer un terminal, et arrêter ses programmes, ne fige jamais la fenêtre.
- Latence, dans le journal de l'app (`app.log` du dossier de données) :
  - à l'ouverture d'un terminal interactif, son moteur de rendu : « renderer webgl (<nom du GPU>) » (un GPU logiciel comme SwiftShader se reconnaît à son nom) ou « renderer dom (<raison>) » sans WebGL ; de nouveau s'il perd son contexte WebGL (Chromium en garde 16 au plus et retire le plus ancien) ;
  - une frappe ou un collage dont l'écho met plus de 300 ms à s'afficher (et moins de 10 s : au-delà, ce n'était pas un écho), au plus une fois toutes les 10 s par terminal, avec le temps de chaque étape : « echo of a 1-byte write shown after 837 ms: written 0 ms after the backend got it, first output 835 ms after, the echo 835 ms after, to and from the window 1 ms (answer to the write 2 ms), painted 1 ms later ». « written » est l'attente avant l'écriture ; « first output » et « the echo », quand le programme a répondu quelque chose puis quand l'écho lui-même est sorti (le temps du programme et du pseudo-terminal) ; « to and from the window », les échanges avec la fenêtre, aller et retour (« answer to the write » : l'aller-retour de la frappe seule) ; « painted », le rendu ;
  - seules les frappes de l'utilisateur (un caractère, dont l'écho est reconnu à son contenu) et ses collages sont mesurés, fenêtre visible : ni les réponses automatiques du terminal (focus, position du curseur, souris), ni les touches qui n'affichent rien. Le détail des mesures : `docs/PROTOCOL.md`, *Terminaux intégrés*.

### Pied

- Le chemin du projet, sa branche (et « Parcourir » : l'éditeur sur la branche du projet), `~ modifiés / + ajoutés / − supprimés`, le sélecteur de couleur.

## Réglages

- Largeur de la barre : à la souris ou au clavier, sur son bord.
- Remote control : Réglages › Harness › « Remote control » et Réglages › Projets › « Remote control » (ci-dessus).
- Commandes de lancement : Réglages › Projets › « Lancement ».
- Shells des terminaux : Réglages › Terminaux (voir *Réglages*).

## Limites

- Les commandes de lancement et les terminaux ne survivent pas à un redémarrage.
- Escouade ne peut pas changer le remote control d'une interface Claude Code déjà ouverte : le changement attend son prochain lancement.
- Claude Code ne dit pas pourquoi il refuse le remote control : Escouade le déduit de signes indirects.
