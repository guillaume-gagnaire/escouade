# Les notifications

Escouade te prévient quand un agent pose une question, demande une autorisation, termine ou tombe en erreur. Cette page décrit chaque signal, ce que dit une notification et comment choisir ce qui te prévient.

![Escouade avec l’agent « tests-e2e » à l’état « Question » et sa carte de question, un onglet de projet mis en évidence, et l’icône de la zone de notification de Windows avec sa pastille](/images/notifications.jpg)

## Ce qui déclenche une notification

| Événement | Couleur du clignotement |
|---|---|
| Un agent pose une question ou demande une autorisation (statut « Question ») | ambre |
| Un agent termine son tour (« Terminé ») | verte |
| Un agent tombe en erreur (« Erreur »), par exemple parce que son processus s’est arrêté | rouge |

Un ticket qui arrive dans « À tester » ou qui se bloque envoie aussi une notification (voir [le Kanban](board)), sans faire clignoter d’agent.

Ces signaux valent aussi pour un agent qui tourne dans [la vue Claude Code](claude-code-view). Quand tu interromps un tour toi-même (`Échap`), ou qu’un message envoyé pendant le tour attend dans la file d’attente de l’agent, la fin du tour n’envoie aucune notification. Pour un agent qui travaille sur un ticket « En cours », seules ses questions te préviennent : c’est le Kanban qui annonce quand le ticket est prêt à tester ou bloqué.

## Dans la fenêtre

### La tuile et la carte qui clignotent

Quand un agent pose une question, termine ou tombe en erreur alors que tu ne le regardais pas, sa carte dans la barre latérale clignote, et la tuile de son projet, dans la barre des projets, prend un anneau qui respire (s’il n’est pas celui que tu regardes). La couleur dit de quoi il s’agit : ambre pour une question, vert pour une fin, rouge pour une erreur. Quand plusieurs agents d’un projet sont à voir, l’onglet prend la couleur la plus pressante : l’erreur d’abord, puis la question, puis la fin. Survole l’onglet pour lire les noms (« À voir : refacto-auth, tests-e2e »).

Le clignotement dure jusqu’à ce que tu affiches l’agent, avec la fenêtre au premier plan. « Regarder » l’agent veut dire que sa conversation est à l’écran : si l’éditeur, le Kanban, un terminal, le journal d’un lancement, les statistiques ou la vue d’ensemble la cachent, l’agent reste à voir. Si ton système demande de réduire les animations, la carte garde une teinte fixe au lieu de clignoter.

### Les compteurs

Les tuiles des projets affichent une pastille avec le nombre d’agents qui attendent une réponse ; un projet hors de vue qui attend se signale par une pastille sous la liste. La barre d’état, en bas, compte les agents « actifs », « en attente » et « terminés », et son bouton « N en attente » met le focus sur la carte du prochain agent à voir. `Ctrl+J` fait la même chose (voir [suivre ses agents](overview)).

## Hors de la fenêtre

### Le carillon

Un carillon de deux notes sonne quand un agent pose une question, termine ou tombe en erreur. Il sonne même quand la fenêtre est au premier plan. Sous Linux, il est joué par `paplay` : sans ce programme, il reste muet. Le bouton « ♪ On » / « ♪ Off » de la barre d’état l’active ou le coupe ; le même réglage est dans Réglages › Notifications, avec un bouton « ▶ Tester ».

### La notification système

Quand la fenêtre n’est pas au premier plan (cachée, réduite, ou sans le focus), Escouade envoie aussi une notification du système, sous Windows et sous macOS. Il n’y en a pas sous Linux. Son titre est `projet · agent`, et son texte dit de quoi il s’agit (voir plus bas). Sous Windows, un clic dessus ramène la fenêtre et ouvre l’agent concerné, ou le Kanban pour une notification de ticket. Sous macOS, un clic ramène l’app au premier plan sans ouvrir l’agent.

### La barre des tâches et le Dock

Quand la fenêtre n’est pas au premier plan, le bouton d’Escouade clignote dans la barre des tâches de Windows, et l’icône du Dock rebondit sous macOS. Sous Linux, Escouade demande l’attention de la fenêtre au système ; ce que ça donne dépend du bureau.

### La zone de notification

Escouade garde une icône dans la zone de notification (la barre des menus sous macOS). Une pastille ambre s’y pose dans le coin tant qu’au moins un agent attend une réponse, et l’infobulle donne le compte (« Escouade — 2 agents en attente »). Un clic sur l’icône affiche la fenêtre ; son menu propose « Afficher », « Recharger l’interface » et « Quitter ». « Recharger l’interface » ramène la fenêtre sur l’interface d’Escouade si elle reste un jour bloquée ou blanche, et te demande d’abord confirmation s’il reste des fichiers non enregistrés dans l’éditeur. Fermer la fenêtre la cache seulement : les agents continuent, et ils te préviennent comme d’habitude (voir [les agents](managing-agents#fermer-la-fenetre-quitter-et-redemarrer)).

## Ce que dit une notification

| Cas | Texte |
|---|---|
| Question | Le texte de la question (la première, s’il y en a plusieurs), sur une ligne ; « Claude attend ta réponse » s’il n’y en a pas |
| Autorisation | « Autoriser Bash : npm test ? » : l’outil, et ce sur quoi il agit (la commande, le fichier relatif au dossier de l’agent, l’adresse, la recherche) |
| Plan | « Approuver le plan : Ajouter le filtre PDF ? » : la première ligne du plan |
| Fin de tour | La première ligne de la réponse de Claude, sans la mise en forme Markdown ; « Tâche terminée » si elle n’a pas de texte |
| Erreur | « Erreur : » suivi de la raison, par exemple « Erreur : Claude Code s’est arrêté (code 1) » |
| Ticket | Le titre est le nom du projet, le texte « ATL-42 prêt à tester » ou « ATL-42 bloqué : » suivi de la raison |
| Pull request | « PR #12 : CI en échec », « DEM-1 · PR #12 : approuvée » pour celle d’un ticket (« MR » sur GitLab). Jamais son titre : il est écrit par un tiers et part hors de l’app. |
| Déploiement | « Déployé : production (<commit>) », « Déploiement en échec : production (<commit>) » |

Le texte d’un agent est coupé à 120 caractères.

## Me prévenir pour

Dans Réglages › Notifications, le groupe « Me prévenir pour » choisit les types qui sonnent et qui envoient une notification système : « Questions et autorisations », « Tâches terminées », « Erreurs » et « Tickets (prêt à tester, bloqué) ». Un type désactivé n’a ni carillon ni notification système, mais l’onglet, la carte et la barre des tâches (ou le Dock) continuent de signaler l’agent. Le groupe « Canaux » coupe d’un coup les notifications système (« Notifications Windows quand l’app n’est pas au premier plan », ou « macOS » sous macOS ; sous Linux, le libellé dit aussi « Windows », mais Escouade n’y affiche pas de notification du système), et le groupe « Son » le carillon. Toutes les options sont décrites dans [les réglages des notifications](settings-notifications).

## Une commande de lancement qui plante

Quand une commande de lancement s’arrête en erreur (voir [le lancement du projet](launch)), un message rouge s’affiche dans la fenêtre : il nomme la commande et donne son code de sortie. La commande passe aussi à « planté (code 2) » dans la section « Lancement » de la barre latérale. Ce n’est pas une notification système et il n’y a pas de carillon : le message ne se voit que dans la fenêtre.
