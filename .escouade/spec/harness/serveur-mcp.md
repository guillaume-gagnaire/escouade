---
title: "Serveur MCP"
subject: "Harness"
sources: ["docs/SPEC.md"]
tickets: ["ESC-13","ESC-14","ESC-50","ESC-90","ESC-91","ESC-92","ESC-98","ESC-102","ESC-103","ESC-107","ESC-108","ESC-119","ESC-128","ESC-141","ESC-157","ESC-198","ESC-256"]
decisions: []
compared: null
---

## Ce que c'est

Escouade sert ses outils aux harness : un agent d'Escouade, ou un harness lancé hors d'Escouade (un terminal, son app), lit l'état d'Escouade et agit dessus (projets, agents, tickets, Kanban, demandes en attente, workflows, docs). C'est un serveur MCP « Streamable HTTP », sans état, à réponses JSON, sur `http://127.0.0.1:<port>/mcp`, la boucle locale seulement. Il tourne quand « Les agents peuvent piloter Escouade » est activé, et s'arrête avec l'app. Un projet privé y est cloisonné (voir *Projets privés*).

## Comportement

### Adresse et poignée de main

- Port libre entre 47000 et 47999, choisi au premier démarrage et gardé dans les réglages, pour que la config des harness reste valable d'un démarrage à l'autre. S'il est pris au démarrage suivant, un autre est choisi, enregistré, et la déclaration refaite.
- Les deux poignées de main passent : `initialize` en `2025-11-25`, et le chemin `2026-07-28` que prend Claude Code 2.1.296.

### Jetons et garde-fous

- Chaque requête est vérifiée avant d'atteindre un outil :
  - un `Host` autre que `127.0.0.1:<port>` ou `localhost:<port>`, ou absent → 403 (contre le DNS rebinding) ;
  - un en-tête `Origin` (une page web) → 403 ;
  - `Authorization: Bearer` absent ou inconnu → 401 (comparaison à temps constant) ;
  - un autre chemin que `/mcp` → 404.
- Une connexion a 10 s pour envoyer sa requête ; 64 connexions au plus à la fois.
- Jetons de 32 octets aléatoires :
  - celui de « Claude (hors Escouade) », pour les harness lancés hors d'Escouade : au trousseau du système (entrée `mcp`), ou dans `mcp-token.json`, lisible par son seul propriétaire, quand le trousseau refuse ;
  - celui de chaque process d'un agent d'Escouade, en mémoire et dans son fichier de config.
- Le jeton dit qui appelle ; l'identité d'un agent ne s'imite pas.

### Déclaré dans chaque harness

L'interrupteur démarre le serveur, le donne aux agents d'Escouade de tous les harness, et le déclare, avec le jeton de « Claude (hors Escouade) », dans chaque harness installé, pour **chaque compte actif** ajouté (un harness sans compte n'a la déclaration nulle part). Un Claude Code, un Codex, un OpenCode ou un Copilot CLI lancé hors d'Escouade pilote alors Escouade.

- Un harness installé plus tard est déclaré au retour du focus sur la fenêtre, un compte ajouté ou réactivé à la sauvegarde ; un compte supprimé perd son entrée. Un changement de port refait la déclaration partout, et l'app la relit à chaque démarrage.
- **Claude Code** : `claude mcp add --scope user --transport http escouade http://127.0.0.1:<port>/mcp --header "Authorization: Bearer <jeton>"`, lancé avec le `claude` et le `CLAUDE_CONFIG_DIR` du compte (jamais pour Principal, dont l'entrée va dans `~/.claude.json`). Escouade compare en lisant l'entrée `mcpServers.escouade` du `.claude.json` du compte (la CLI ne sert qu'à écrire : `claude mcp get` se connecterait au serveur) : identique, rien n'est lancé ; absente, `add` ; différente (autre port, autre jeton), `remove` puis `add`.
- **Codex, OpenCode et Copilot CLI** : Escouade écrit elle-même l'entrée `escouade` (adresse et `Authorization: Bearer <jeton>`) dans leur config d'utilisateur, même avant leur premier agent : `[mcp_servers.escouade]` du `config.toml` de Codex (`~/.codex/config.toml` pour Principal), `mcp.escouade` de l'`opencode.json` d'OpenCode (sous `$XDG_CONFIG_HOME/opencode`, ou l'`opencode.jsonc` qui est là), `mcpServers.escouade` du `mcp-config.json` de Copilot CLI (`~/.copilot/mcp-config.json` pour Principal). Le reste du fichier est gardé (commentaires, ordre, indentation, fins de ligne) ; un fichier lié à celui du Principal reste un lien ; un fichier qui n'est pas un TOML ou un JSON valide n'est pas touché, et l'onglet donne l'entrée à coller.
- Le jeton est écrit **en clair** dans ces fichiers, ce que dit l'onglet.
- **Désactiver** change d'abord le jeton externe, dès que l'interrupteur est coupé : l'ancien n'ouvre plus rien tout de suite, quoi que fassent les commandes qui suivent. Puis les jetons des agents sont oubliés avec leurs fichiers (les agents déjà lancés perdent les outils tout de suite), et l'entrée est retirée de chaque compte : chaque fichier de Codex, OpenCode et Copilot CLI redevient celui d'avant, à l'octet près. Supprimer un compte retire son entrée.

### Agents d'Escouade

- Le même interrupteur sert les agents de tous les projets ; il n'y a pas de réglage par projet.
- Activé et serveur en marche : le process d'un agent reçoit le serveur avec un jeton à lui, nouveau à chaque lancement (pour Claude Code, `--mcp-config <dossier de données>/mcp/<id>.json`, dans un fichier privé, 0600 sous Unix ; pour les autres harness, la ligne « Serveur MCP d'Escouade (agents) » de la table de *Harness*). Le jeton est oublié quand ce process se termine, à la suppression de l'agent, à la fermeture du projet et à l'arrêt de l'app. Un serveur passé par `--mcp-config` remplace celui du même nom de la portée `user`.
- Désactivé, ou activé alors que le serveur est arrêté : l'agent est tenu à l'écart du serveur, car une entrée `escouade` de la config de l'utilisateur peut rester déclarée et l'agent l'hériterait (il agirait comme « Claude (hors Escouade) »). `--disallowedTools mcp__escouade` lui refuse les outils, et `--settings <dossier de données>/mcp-refused.json` (`{"deniedMcpServers": [{"serverName": "escouade"}]}`) empêche Claude Code de charger le serveur, faute de quoi l'agent recevrait ses instructions sans aucun de ses outils. En vue Claude Code, ce refus entre dans le fichier `--settings` des hooks (Claude Code ne lit que le dernier `--settings`).
- **Outils d'Escouade au prochain démarrage.** Un agent dont le process (ou l'interface, en vue Claude Code) a démarré sans les outils, alors qu'un démarrage les lui donnerait maintenant (l'interrupteur activé depuis), le dit : sa conversation montre « Outils d'Escouade au prochain démarrage » avec « Relancer ». Jamais sans process vivant ; recalculé à chaque changement de l'interrupteur ou du serveur.
  - « Relancer » ne vaut que pour un agent au repos : ni tour en cours, ni question ou autorisation en attente, ni préparation de worktree, ni sous-agent, workflow ou tâche de fond (commande, surveillance…) en arrière-plan ; sinon il est refusé et le bouton est grisé. Pour un tour, une question ou une préparation, qui finissent seuls, la carte dit « Il pourra être relancé quand il aura fini ce qu'il fait. » et le bouton se rallume. Pour une tâche de fond : « Il a encore des tâches de fond (commande, surveillance, sous-agent, workflow) : le relancer les arrêterait. Une commande ou un serveur de dev ne finit pas seul : arrête-la d'abord, depuis sa vue (« Arrêter »), ou attends qu'elle ait fini. » Refusé aussi pour un agent archivé.
  - Le process s'arrête (entrée fermée, tué au bout de 10 s), puis le suivant reprend la même session (`--resume`) : la conversation reste. En vue Claude Code, « Relancer » redémarre son interface sur sa session. Le process n'est jamais relancé tout seul.
  - Sous l'interrupteur, Réglages › Harness compte ces agents : « 1 agent déjà lancé aura les outils au prochain démarrage de son process : « Relancer » sur sa conversation. »

### Ce qu'un agent voit et touche

- **Entre projets qui ne sont pas privés, rien n'est cloisonné.** L'agent d'un projet qui n'est pas privé lit et agit sur le sien et sur tous les projets qui ne sont pas privés : tickets, agents, messages, arrêts et messages aux sous-agents et aux tâches de fond, réponses aux demandes, Kanban, images et fichiers des tickets, sortie des tâches de fond. Il agit toujours, aussi, par ses autres outils.
- **Un projet privé est cloisonné dans les deux sens**, pour les agents d'Escouade (le jeton de chaque agent dit son projet ; relu à chaque appel ; ses fichiers et le reste : voir *Projets privés*) :
  - il n'existe pas pour les agents des autres projets : absent de `list_projects`, `list_agents` et `list_requests` sans argument ; un outil qui le nomme, par son id, son nom, un de ses tickets ou de ses agents, répond « introuvable » mot pour mot comme pour un nom que rien ne porte, et les choix qu'il propose ne citent que ce que l'appelant voit ;
  - ses agents ne voient que lui : `list_projects` ne donne que lui, et tout autre projet, privé ou non, est introuvable de la même façon ;
  - il n'est touché que par ses propres agents. Aucun refus ne nomme un projet privé.
- « Claude (hors Escouade) », le jeton de l'utilisateur, voit et agit sur tout ; la fenêtre aussi (onglets, vue d'ensemble, statistiques). `get_usage` ne nomme aucun projet.
- Un agent que le cœur n'a plus n'agit nulle part.
- Les outils passent par le cœur comme la fenêtre : mêmes événements, synchro Jira / Trello / GitHub, replanification du Kanban.

### Les outils

Chaque outil est annoté lecture seule ou non. Réponse : un bloc de texte JSON, clés en camelCase ; `get_ticket_image` répond un bloc image puis un bloc de texte JSON. Formats dans docs/PROTOCOL.md. Un projet ou un agent se nomme par son nom (sans tenir compte de la casse) ou son id, un ticket par sa clé ou son id ; un nom inconnu ou ambigu donne une erreur qui propose jusqu'à cinq choix.

#### Lecture

- `list_projects` : les projets, avec les deux réglages du remote control (`remoteControl`, celui du projet, « Remote control pour les agents de ce projet » ; `remoteControlAll`, le global, « Remote control pour tous les agents », le même sur chaque ligne : allumé, il couvre tous les agents de tous les projets, et l'interrupteur du projet est grisé dans la fenêtre).
- `list_agents` : statut, attend une réponse et combien, ticket, modèle, compte, coût, sous-agents en cours (`subagentsRunning`, au premier plan ou en arrière-plan) et tâches de fond en cours (`backgroundTasksRunning`, jamais les sous-agents : le « N en fond » de la carte ; un agent « Terminé » peut ainsi avoir encore du travail qui tourne), remote control, workflow (`workflow` : nom, étape, tour, depuis ; `null` sans workflow).
- `list_tickets` : par colonne, dépendances, agent, raison du blocage, compte, modèle et effort quand ils ne sont pas sur « Auto ».
- `get_ticket` : description, critères, boucles, avancement ; la liste de ses images (rang, fichier, nom, type, taille) et celle de ses autres fichiers (rang, fichier, nom, taille) ; le nombre de ses fichiers joints (`files`, que son agent reçoit par leurs chemins) ; ticket externe ; son compte, son modèle et son effort (`auto` quand ils suivent le compte courant, le Kanban ou le compte) et pourquoi il est repassé en « Auto » ; et de quoi suivre le flux : son agent et les demandes qui attendent, sa branche et son worktree, ce qui le bloque (raison, conflit), l'étape de validation en cours, le résultat une fois terminé (texte, adresse, commit du merge). Un agent d'Escouade lit le sien sans le nommer.
- `get_ticket_image` : une image d'un ticket, par son fichier ou son rang dans la liste de `get_ticket` : le contenu en bloc image, puis ses nom, type et taille. Un agent d'Escouade omet le ticket pour lire le sien.
- `get_ticket_file` : un morceau d'un autre fichier d'un ticket, de tout format, désigné comme une image : sa fin par défaut, ou à partir d'un `offset` ; 8 Kio par défaut, 64 Kio au plus ; en texte quand c'en est, en base64 sinon. Claude hors Escouade reçoit en plus le chemin du fichier, pour le lire en entier ; un agent d'Escouade ne reçoit pas de chemin.
- `get_board` : les réglages du Kanban d'un projet, avec son nom et son id : après validation, branche cible, stratégie, brouillon, nettoyage, commits conventionnels, conflit, en parallèle, pilote auto, modèle, effort et mode des agents de tickets (vides quand ce sont ceux de l'app, rendus à côté), tests d'abord (vrai seulement si le Kanban a une commande de test), commande de test, préfixe ; et le modèle, l'effort et le mode du compte où partirait un nouvel agent du projet. Sa réponse redonnée telle quelle à `set_board` est toujours acceptée et ne change rien.
- `get_usage` : les compteurs de chaque compte, le compte en cours, la pause du pilote auto.
- `get_agent_summary` : dernière réponse (2 000 caractères au plus), fichiers touchés, nombre et types des demandes qui attendent, sous-agents et tâches de fond en cours, remote control et workflow comme `list_agents`, plus l'historique de ses étapes de workflow (les 100 derniers passages) ; jamais la conversation.
- `list_background_work` : les sous-agents et les tâches de fond d'un agent (voir plus bas).
- `get_background_output` : la fin de la sortie d'une commande ou d'une surveillance, ou le rapport d'un sous-agent (voir plus bas).
- `list_requests` : les questions et les demandes de permission qui attendent la réponse d'un agent, d'un projet ou de tous, dans l'ordre où elles sont venues : l'agent et son ticket, le type (permission, question ou plan), depuis quand, l'outil et ce sur quoi il agit, son entrée bornée (le contenu d'un fichier à écrire, les chaînes d'un Edit, les options d'une commande, les arguments d'un outil MCP), la commande ou le plan entiers jusqu'à 20 000 caractères, la raison, les questions avec leurs options et si plusieurs se choisissent, le plan entier.
- `wait_ticket` : 1 à 20 tickets. Attend que l'un change (sa colonne, son blocage, son conflit, son agent qui attend une réponse ou se met au repos en « À tester », où il peut être validé, ou sa suppression) et dit quoi ; 120 s par défaut, 240 s au plus, sinon `changed: false`. Chaque réponse donne un curseur `since` : repassé à l'appel suivant, il fait répondre aussitôt un ticket dont l'état diffère de celui qu'il garde, un changement survenu entre les deux appels compris ; sans lui, c'est l'état de chaque ticket à l'appel qui sert de comparaison. Un client qui annule la requête ou coupe la connexion arrête l'attente.
- `get_workflow` : les workflows d'un projet (étapes, portes, sous-agents, retours avec leur condition et leur maximum, fichiers d'état) et, sur chaque étape, les agents qui y sont (`agentsHere` : id, nom, depuis quand, Nᵉ passage) ; les agents qui suivent un workflow sans être sur une étape sont à part (`agentsElsewhere`). Un projet privé ne dit rien de ses workflows, ni de ses agents sur eux, aux agents des autres projets.

**Remote control**, lu seulement, par `list_agents` (chaque agent) et `get_agent_summary` :

- `remoteControl` : vrai quand il est en vigueur pour l'agent maintenant ; sa session est alors joignable depuis claude.ai, son process restant debout.
- `remoteSource` : ce qui l'a allumé. `"hand"`, l'utilisateur au clic droit de l'agent ; `"project"`, le réglage du projet ; `"global"`, le réglage de tous les agents, qui passe avant celui du projet quand les deux sont allumés ; `"off"`, l'agent que l'utilisateur a mis à part par « Désactiver le remote control » au clic droit, à qui aucun réglage ne le redonne, redémarrage compris, tant qu'il ne l'allume pas à la main ; `null` quand il n'est pas en vigueur et que l'agent n'est pas mis à part. `remoteControl: true` va donc avec `"hand"`, `"project"` ou `"global"`, `remoteControl: false` avec `"off"` ou `null`.
- `remoteState` : l'état du lien avec claude.ai que Claude Code rapporte pour l'agent. `"ready"`, en attente de connexion ; `"connected"` ; `"unavailable"`, en vue Claude Code, quand l'interface tourne mais que Claude Code n'a pas ouvert le remote control (compte sans abonnement claude.ai, réseau, proxy ou règle de l'organisation) ; `null` sans lien en cours.
- `interfaceRemote` : pour un agent en vue Claude Code, vrai quand son interface tourne et a été lancée avec le remote control, donc liée à claude.ai jusqu'à son prochain lancement, même si `remoteControl` est repassé à faux ; faux quand elle tourne sans ; `null` quand aucune interface ne tourne.

#### Actions

- `create_ticket` : avec, au choix, des images ou d'autres fichiers (voir plus bas), et le compte (par son nom ou son id), le modèle et l'effort de ses agents (`auto` par défaut).
- `update_ticket` : modifie un ticket « À faire » ; ajoute des images à la fin, en retire par leur fichier ou leur rang (celles qu'on ne nomme pas sont gardées) ; de même pour les fichiers ; compte, modèle et effort comme `create_ticket`.
- `move_ticket` : réordonne un ticket « À faire » : en tête, en bas, avant un autre.
- `start_ticket` : avec, au choix, le compte, le modèle et l'effort de ce ticket seul (le compte étant fixé comme dans sa fenêtre), qui passent avant ceux du Kanban pour son agent : garder Opus sur un ticket d'architecture quand le Kanban est sur Sonnet. `""` remet ceux du Kanban. Le ticket garde son choix s'il est renvoyé puis relancé. La réponse dit avec quoi son agent part, sans `effort` quand le modèle n'en prend pas (Haiku, dont l'agent part aussi en mode « Demander » si son mode serait Auto).
- `set_board` : règle le Kanban d'un projet, n'importe lesquels de : après validation, branche cible, stratégie, brouillon, nettoyage, commits conventionnels, conflit, en parallèle, pilote auto, modèle, effort, mode (`auto`, `default`, `plan`, `acceptEdits`) et tests d'abord. Les champs laissés de côté ne changent pas ; `""` pour le modèle, l'effort ou le mode remet celui de l'app. La fenêtre se met à jour et le Kanban replanifie (allumer le pilote auto peut démarrer des tickets). Cela vaut pour les prochains agents de tickets, pas pour ceux déjà lancés : un agent qui mène le Kanban (un « chef d'escouade ») peut ainsi passer les agents de tickets sur Sonnet pour ménager le quota. Un agent d'Escouade règle comme la fenêtre, mode compris, le Kanban de tout projet qu'il voit. Sa réponse est le Kanban après le changement.
- `create_agent` : projet, message de départ, worktree ou non, modèle.
- `send_message` : 32 000 caractères au plus (côté MCP seulement).
- `stop_agent`.
- `stop_background_work`, `message_subagent` : voir plus bas.
- `answer_permission` : autoriser ou refuser une permission, avec un message au refus ; approuver ou renvoyer un plan, comme la carte de la fenêtre.
- `answer_question` : répondre à une question avec les libellés des options ou un texte libre, au format de la carte de question.

#### Le flux du Kanban

Comme les boutons de la fenêtre :

- `approve_ticket` (« Valider… ») : les tests si le Kanban les lance, le commit, puis l'action à la validation du projet (merge dans la branche cible, push et pull request, push de la branche ou branche gardée) ; une branche déjà fusionnée à la main dans la cible est close sans fusionner de nouveau. Il répond une fois la validation partie ; la suite se suit par `wait_ticket` ou `get_ticket`.
- `reject_ticket` (« Renvoyer ») : le ticket repart « En cours » avec le même agent et le même worktree, et le commentaire lui est envoyé.
- `resume_ticket` (« Reprendre ») : un ticket bloqué « En cours ».
- `resolve_conflict` (« L'agent résout ») : un ticket « À tester » arrêté sur un conflit de merge.
- `dismiss_block` (« Annuler ») : le blocage d'un ticket « À tester » (conflit ou étape en échec) s'efface, le ticket reste en place.
- `delete_ticket` (« Supprimer ») : quelle que soit la colonne ; refusé tant que la validation du ticket tourne.

`approve_ticket`, `reject_ticket`, `resume_ticket`, `resolve_conflict` et `dismiss_block` répondent aussi `since` : le curseur de `wait_ticket` du ticket tel qu'il était avant l'action, à lui passer pour que la suite, une validation rapide comprise, soit dite aussitôt.

#### Réservés aux agents d'Escouade

Refusés à Claude hors Escouade (notés « refusé ») :

- `report_progress` : une ligne d'état de 120 caractères au plus, sans caractère caché, affichée sous le nom de l'agent, dans la vue d'ensemble et sur la carte de son ticket « En cours », jusqu'au prochain appel.
- `split_ticket` : 1 à 10 tickets créés en fin de « À faire », après celui de l'agent, à la suite l'un de l'autre ou tous en parallèle. Chacun reprend, en copies à lui, toutes les images du ticket de l'agent, ou seulement celles qu'il nomme, ou aucune ; de même pour les fichiers (`files` : tous par défaut, ceux nommés, ou aucun avec `[]`). Par défaut, une image ou un fichier qui a disparu (ou un fichier que les réglages ne permettent plus : il dépasse la taille maximale) est sauté, et la réponse le dit (`{ tickets, skipped }`, le tableau des clés quand rien n'est sauté) ; nommé, c'est une erreur, sans rien créer.

#### Docs

Douze outils pour la spec, les décisions et les documents d'un projet (voir *Docs du projet* et *Documents*) : en lecture seule `get_spec_tree`, `read_spec_page`, `search_spec`, `list_decisions`, `read_decision`, `list_documents`, `read_document`, `search_documents` ; en écriture `write_spec_page`, `propose_decision`, `replace_decision`, `add_document`. Ils écrivent dans la branche des docs du projet sans rien commiter. Aucun n'accepte une décision : seul un humain le fait, dans la fenêtre.

#### Sous-agents et tâches de fond d'un agent

Quatre outils font ce que la fenêtre fait pour les sous-agents et les tâches de fond (commandes, surveillances, workflows, agents distants, coéquipiers) : mêmes bornes, mêmes refus, mêmes événements. Formats dans docs/PROTOCOL.md.

- **`list_background_work`** (lecture seule) : les deux listes d'un agent, ce qui tourne d'abord, puis ce qui est fini, le dernier fini d'abord (l'ordre de la fenêtre), dates en millisecondes.
  - Un sous-agent : son id (celui de l'appel qui l'a lancé), son type, sa description, son modèle, son état, s'il est en arrière-plan, ses dates, ses outils, ses tokens, ce qu'il fait, sa tâche (`taskId`) et son parent.
  - Une tâche : son id (celui de Claude Code), son type (`command`, `monitor`, `remote`, `teammate`, `workflow`), sa description, sa commande, son état, ses dates, son résumé (le dernier mot de Claude Code sur elle, code de sortie d'une commande compris), le sous-agent qui l'a lancée (`parent`, par sa tâche) et, pour un workflow, son nom, sa phase, ses outils et ses tokens.
  - Le chemin du fichier de sortie n'est jamais donné. Une ligne de commande pouvant porter un secret (`curl -H "Authorization: …"`), un agent que le cœur n'a plus ne reçoit de chaque ligne que `id`, `type`, `status`, `startedAt` et `endedAt`.
- **`get_background_output`** (lecture seule) :
  - Pour une commande ou une surveillance, la fin de la sortie : 8 Kio d'une tâche qui tourne (son processus, sinon son fichier de sortie), jusqu'à 256 Kio du fichier une fois finie ; seul le fichier `<tâche>.output` d'un dossier `tasks` est lu, jamais un chemin réseau. `bytes` (de 1 à 65 536, 8 192 par défaut) en demande moins, la fin gardée et coupée à un caractère : Claude Code refuse une réponse d'outil de plus d'environ 25 000 jetons. La fenêtre garde ses bornes ; un client MCP ne peut pas en demander autant. Une valeur hors de la plage est ramenée dedans ; un `bytes` illisible (`"abc"`, `-1`) est refusé par le schéma, sans que la valeur soit renvoyée ni journalisée. `source` : `process`, `file`, `report` ou `none`.
  - Un agent distant, un coéquipier et un workflow n'ont pas de sortie.
  - Pour un sous-agent, son rapport tel que la fenêtre le montre (16 000 octets au plus, rendu entier : `bytes` ne s'y applique pas), vide tant qu'il n'a rien rendu ; son `taskId` désigne aussi le sous-agent. En vue Claude Code, le rapport vient du transcript du sous-agent, lu sans rien changer : l'outil reste en lecture seule.
  - La sortie d'une commande est aussi sensible que le contenu d'une image (clés, chemins, données).
- **`stop_background_work`** (agit, destructif) : « Arrêter » de la fenêtre, pour une tâche de fond, un workflow ou un sous-agent en arrière-plan. Mêmes refus que le bouton : ne tourne plus, sous-agent au premier plan (l'agent l'attend, et « Arrêter » de l'agent l'arrête avec son tour), pas encore de tâche donnée par Claude Code, agent en vue Claude Code (son interface le fait), pas de processus, agent archivé. La réponse dit que l'arrêt est demandé, pas que la ligne a fini : sa fin arrive avec les trames de Claude Code, et `list_background_work` la montre.
- **`message_subagent`** (agit) : « Lui écrire » de la fenêtre, pour un sous-agent en arrière-plan qui tourne. Claude Code ne laisse aucun hôte écrire à un sous-agent : le message va à l'agent, à qui l'on demande de le transmettre tel quel au sous-agent par son outil `SendMessage` (voir docs/PROTOCOL.md, « Lui écrire »). Il coûte un tour de l'agent, donc du quota. `queued` : l'agent est au travail, le message attend la fin de son tour.
  - Refusés, sans rien envoyer : un sous-agent qui ne tourne plus (terminé, arrêté, en échec ou interrompu, même avec une tâche donnée : « Ce sous-agent ne tourne plus. »), un sous-agent au premier plan ou sans tâche donnée, un agent en vue Claude Code ou archivé, et un `subagent` qui est une tâche de fond.
  - Une erreur pour un texte vide ou de plus de 32 000 caractères (des caractères, pas des octets : « Le message fait {n} caractères : 32000 au plus. », sans renvoyer le texte). La même borne vaut pour `send_message`, côté MCP seulement.

#### Images d'un ticket

- `create_ticket`, `update_ticket` et `split_ticket` ont les limites et les refus de la fenêtre : PNG, JPEG, GIF et WebP (le type est lu dans le contenu, pas dans celui qu'on déclare), 5 Mo par image, 18 Mo en tout, 20 images au plus par ticket, les gardées comptées. Une image refusée ne laisse rien (ni ticket, ni clé, ni fichier).
- Le total de 18 Mo est tenu avant de lire : l'image qui le ferait dépasser est refusée avant d'être lue. `update_ticket` vérifie d'abord que le ticket est « À faire » et refuse sinon sans lire les fichiers donnés.
- Une image est donnée par son contenu en base64 (`data`, avec un `name`) ou par le chemin d'un fichier local (`path`).
- **Un agent d'Escouade n'attache par `path` qu'un fichier sous son dossier de travail** : les liens (symboliques ou jonctions) qui en sortent sont refusés, la vérification se faisant sur le fichier ouvert. C'est un garde-fou du serveur, pas une frontière de sécurité (un agent qui a la main sur sa machine a d'autres voies). Un fichier hors du dossier reçoit le même refus qu'il existe ou non, pour ne rien dire de son existence. Claude hors Escouade lit n'importe quel chemin absolu.
- Refusés : un chemin relatif, un chemin réseau ou de périphérique Windows (`\\…`, `//…`), ce qui n'est pas un fichier ordinaire, un fichier de plus de 5 Mo (refusé sur sa taille, avant d'être lu).
- Le nom d'une image est nettoyé (ce qui suit le dernier `/` ou `\`, sur une ligne, sans caractère caché ; 100 caractères au plus) : il s'affiche et part dans le premier message de l'agent.
- Une image se désigne par son `file` ou son `index`, jamais par un chemin : une référence qui ne correspond à aucune image du ticket est refusée sans rien toucher. Un ticket parti garde ses images : il ne se modifie pas.

#### Autres fichiers d'un ticket

- Les fichiers de tout format qu'on joint à un ticket dans sa fenêtre (voir *Conversation*, « Fichiers joints ») se joignent aussi par le serveur : `create_ticket` (`files`), `update_ticket` (`addFiles`, `removeFiles` : ceux qu'il ne nomme pas restent, un fichier retiré est effacé), `split_ticket` (`files`).
- Ils prennent le chemin de la fenêtre : chaque nouveau fichier est une copie dans les brouillons, faite comme le formulaire la fait, vérifiée puis déplacée dans le dossier `files/` du ticket.
- Mêmes limites et mêmes refus que la fenêtre : 20 fichiers au plus par ticket (les gardés comptés), chacun pas plus gros que « Taille maximale d'un fichier joint » (Réglages › Harness › Pièces jointes). Elles sont tenues avant toute copie : le nombre avant de regarder un fichier, la taille d'un contenu sur la longueur de son base64, celle d'un fichier sur sa taille une fois ouvert, avant d'en lire un octet ; le ticket « À faire » vérifié d'abord. Un refus ne laisse rien, pas même une copie dans les brouillons.
- Un fichier se donne par son contenu en base64 (`data`, avec un `name`, pour un petit fichier : une requête pèse 32 Mio au plus) ou par son chemin (`path`, copié), aux règles des images : un agent d'Escouade ne donne qu'un fichier de son dossier de travail, vérifié sur le fichier ouvert. Le nom est nettoyé comme la fenêtre nettoie celui d'une copie.
- Un fichier se désigne par son `file` (`<uniq>/<nom>`) ou son `index` dans `files` de `get_ticket`, jamais par un chemin.

#### Arguments invalides

Un appel dont les arguments ne collent pas au schéma est noté au journal d'un message fixe, et ne renvoie au client que le texte de l'erreur coupé avant toute valeur citée (jamais un chemin ni du base64), 300 caractères au plus.

### Garde-fous de l'action

Ceux de la fenêtre et du pilote auto :

- `start_ticket` et `create_agent` ne sont refusés pour la pause que si aucun compte n'est utilisable pour le projet ; `send_message` que si le compte de l'agent visé est retenu (le refus dit jusqu'à quelle heure). `message_subagent` de même, mais après tous ses autres refus : un client ne doit pas attendre la fin de la retenue pour être refusé de toute façon.
- Le maximum « En parallèle » du projet n'est pas dépassé. Un ticket qui attend d'autres tickets n'est pas lancé ; une dépendance qui ferait attendre deux tickets l'un l'autre est refusée. Un ticket parti n'est ni modifié, ni déplacé, ni relancé.
- Un agent ne s'écrit pas et ne s'arrête pas lui-même ; on n'écrit pas à un agent archivé. Un agent d'Escouade n'arrête rien de son propre agent, sous-agents et tâches de fond compris, et n'écrit pas à ses propres sous-agents.
- Un agent ne répond jamais à ses propres demandes. Une demande qui n'attend plus de réponse (l'utilisateur ou un autre client y a répondu, l'agent ne tourne plus) est refusée avec ce motif : jamais deux réponses. Un agent en vue Claude Code n'a rien à répondre ici (ses demandes sont dans son terminal) et le refus le dit.
- **`set_board` a les contrôles de la fenêtre** : modèle connu ; effort parmi `low`, `medium`, `high`, `xhigh` et `max` ; mode parmi `auto`, `default`, `plan` et `acceptEdits` ; branche cible = une branche locale du projet ; « En parallèle » de 1 à 6. Refusés : `bypassPermissions` (sauf s'il est déjà le mode du Kanban) et `testsFirst: true` sans commande de test, pour l'allumer seulement. Aucun champ donné : erreur. L'aller-retour `get_board` → `set_board` passe toujours : une cible égale à celle du Kanban (même vide, sur une HEAD détachée) ou à celle que `get_board` montre est laissée telle quelle, sans contrôle, même si la branche a été supprimée depuis ; toute autre doit être une branche locale du projet (une cible vide qui diffère de celle du Kanban est une erreur) ; un `testsFirst` déjà allumé passe.
- `start_ticket`, qui n'a pas de mode, donne les mêmes contrôles au modèle et à l'effort d'un ticket, et à eux seuls.

### Garde-fous du flux du Kanban

- Chaque outil est refusé là où la fenêtre refuse : `approve_ticket` et `reject_ticket` pour un ticket qui n'est pas « À tester », dont l'agent travaille ou dont la validation tourne déjà ; `resume_ticket` pour un ticket qui n'est pas « En cours », qui n'est pas bloqué ou dont l'agent attend la remise à zéro de son quota (le refus dit jusqu'à quand) ; `resolve_conflict` pour un ticket « À tester » sans conflit ; `dismiss_block` pour un ticket « À tester » qui n'est pas bloqué, ou dont la validation tourne ; `delete_ticket` pour un ticket dont la validation tourne (« La validation de ce ticket est en cours : attends sa fin pour le supprimer. »).
- `reject_ticket` refuse un commentaire vide (« Dis ce qui ne va pas. »). Le commentaire que l'agent lit est suivi de « (renvoyé par <auteur> via Escouade) », jamais pris pour les mots de l'utilisateur.
- Un message que le flux envoie à un agent (`reject_ticket`, `resolve_conflict`, `resume_ticket`) n'est remis que tant que le compte de cet agent n'est pas retenu, comme `send_message`. `resume_ticket` d'un ticket dont l'agent a disparu, qui repart avec un nouvel agent, est refusé quand aucun compte n'est utilisable pour le projet.
- **Un agent d'Escouade ne valide ni ne renvoie son propre ticket** (valider, ce serait fusionner son travail sans relecture), et n'agit pas non plus sur lui avec `resume_ticket`, `resolve_conflict`, `dismiss_block` et `delete_ticket` : c'est à l'utilisateur ou à un autre client de le faire.

### Origine toujours dite

- Un texte venu du serveur n'est jamais pris pour les mots de l'utilisateur. `send_message`, `message_subagent` (le texte que l'agent doit transmettre) et le premier message de `create_agent` sont remis à l'agent, et montrés dans sa conversation, sous « Message de <auteur> : » (« Message from <author>: »), l'auteur étant le nom de l'agent qui appelle ou « Claude (hors Escouade) ». Un texte qui commence par `/` n'est plus une commande, et l'en-tête ne compte pas dans le nom donné à l'agent.
- La description d'un ticket créé par `create_ticket` ou `split_ticket` se termine, après une ligne vide, par « Créé par <auteur> via Escouade » : visible sur la carte et dans le protocole de son agent. `update_ticket` termine de même la description qu'il écrit par « Modifié par <auteur> via Escouade » (pas quand il ne change que des images ou des dépendances). Une ligne d'origine finale de la description donnée est remplacée, jamais empilée.
- Une demande à laquelle le serveur a répondu le dit : sa carte, dans la conversation de l'agent qui la posait, montre « Autorisé par <auteur> via Escouade », « Refusé par <auteur> via Escouade » ou « Répondu par <auteur> via Escouade » (« Allowed by <author> through Escouade »…). Rien de tel quand l'utilisateur a répondu dans la fenêtre.
- Une approbation (`allow`, un plan approuvé) arrive à Claude sans auteur : Claude Code écrit lui-même « User has approved… », rien de ce que le serveur envoie n'y est lu ; la carte dit toujours qui a approuvé. Un refus dit à Claude qui a refusé (« Refusé par <auteur> via Escouade : <message> »), jamais que l'utilisateur l'a fait. Chaque réponse à une question que Claude Code reçoit est suivie de « (réponse de <auteur> via Escouade) » (« (answer from <author> through Escouade) »), options choisies comme texte libre, dans la langue des textes pour Claude ; la carte garde la réponse telle qu'elle a été choisie.
- L'agent repasse en cours comme pour une réponse de la fenêtre : la pastille d'attente s'en va.

### Autorisations

Les outils d'Escouade passent par les mêmes demandes de permission que les autres (`mcp__escouade__*`). La carte nomme l'outil en clair (« Escouade · Créer un ticket », « Escouade · Lancer un agent »…) et liste ce que Claude lui donne, un argument par ligne, les textes sur plusieurs lignes tels quels, 2 000 caractères au plus chacun (au-delà, la carte dit combien ne sont pas montrés et renvoie à la conversation). La vue d'ensemble n'y répond pas : « À lire dans la conversation avant de répondre. »

### Activité

- Réglages › Harness › « Activité MCP » : les 200 derniers appels et refus, en mémoire, les plus récents en tête (heure, appelant, outil, résumé, résultat « Fait » / « Refusé » / « Erreur »), mis à jour en direct, avec « Effacer » ; jamais un jeton.
- Chaque appel y figure, refusé ou non, un outil inconnu ou des arguments invalides compris, et chaque requête refusée avant d'atteindre un outil.
- Pour chaque `set_board` réussi, le résumé garde ce qui a été demandé et chaque champ changé avec sa valeur avant et après (« model: opus → sonnet · effort: (défaut) → medium »), ou « Aucun changement ».
- Images et fichiers : combien (`images: 2`, `addImages: 1 · removeImages: 2`, `files: 2`, `addFiles: 1 · removeFiles: 2`), jamais leur contenu ni un chemin. Sous-agents et tâches de fond : l'outil, l'agent, l'id, l'issue, jamais le texte d'un message ni une sortie.

## Réglages

- Réglages › Harness › « Serveur MCP » :
  - l'interrupteur « Les agents peuvent piloter Escouade », enregistré tout de suite, hors brouillon ;
  - sous lui, la liste des déclarations : harness et compte (« Codex · compte Principal »), le fichier, son état (« Déclaré », « Pas déclaré » et pourquoi), et en cas d'échec la commande ou l'entrée à coller (jeton masqué, « Copier » ou « Copier la commande » donne le vrai, écrit pour PowerShell ou pour sh) ; « Déclaré dans Claude · compte Principal, compte Pro » ;
  - une note qui dit que le jeton est écrit en clair ;
  - le nombre d'agents qui auront les outils au prochain démarrage ;
  - « Activité MCP ».
- Le remote control se règle dans la fenêtre seulement : Réglages › Harness, Réglages › Projets, clic droit d'un agent.

## Limites

- Pas de pont stdio : Claude Desktop n'est pas pris en charge.
- Cursor CLI, Mistral Vibe, Antigravity CLI et Gemini CLI ne sont pas encore déclarés pour l'utilisateur (leurs agents d'Escouade reçoivent le serveur).
- Le jeton est écrit en clair dans la config de chaque harness.
- Depuis WSL en NAT, les outils ne joignent pas l'app (voir *Claude Code dans WSL*).
- **Exclus volontairement.** Aucun outil ne lance une commande de l'appelant, ni ne supprime un projet ou un agent : un agent poussé par un contenu piégé pourrait s'en servir pour monter en droits. Les tests, le merge et la suppression d'un ticket ne partent que par les outils du flux du Kanban, qui font ce que font les boutons de la fenêtre.
- **Jamais « Toujours autoriser ».** `answer_permission` n'offre que `allow` et `deny` ; `always` est refusé avec la raison : une règle de permission durable reste la décision de l'utilisateur, dans la fenêtre. De même, `set_board` ne met pas le mode `bypassPermissions` (sauf s'il l'est déjà) et ne règle pas la commande de test.
- **Remote control en lecture seule.** Aucun outil ne l'allume, ne l'éteint ni ne le règle, pour un agent, un projet ou tous : l'allumer expose une session sur internet (joignable depuis claude.ai et l'app mobile). Les descriptions de `list_projects`, `list_agents` et `get_agent_summary` le disent.
- **Montées en droits assumées.** `approve_ticket` est ouvert à tout client du serveur, Claude hors Escouade comme les agents (dans les projets qu'ils voient, jamais sur leur propre ticket), et `delete_ticket` supprime comme la fenêtre : un agent poussé par un contenu piégé peut valider ou supprimer le ticket d'un autre agent. Une fois l'interrupteur allumé, tout client voit les demandes et peut y répondre, sans réglage de plus ni limite de ce qu'il peut autoriser : un tel agent peut autoriser une commande demandée par un autre agent d'un projet qu'il voit, une demande à la fois, jamais durablement, jamais dans un projet privé qui n'est pas le sien. `stop_background_work` n'arrête que ce que l'agent a lancé et ne lance aucune commande ; `message_subagent` n'est qu'un message à l'agent sous le nom de son auteur ; `get_background_output` ne lit que le fichier de sortie qu'a gardé Claude Code pour la tâche.
- Un agent qui a les outils agit aussi par ses autres outils : entre projets non privés, le serveur n'est pas un cloisonnement.
