# La vue terminal

La vue terminal fait tourner un agent dans l’interface de son harness (Claude Code, Codex, OpenCode, Copilot CLI, Cursor CLI, Mistral Vibe, Antigravity CLI ou Gemini CLI), dans un terminal, sans perdre sa session. Cette page explique comment changer de vue, ce qu’Escouade continue de suivre et ce qui change.

## À quoi ça sert

La conversation d’Escouade montre l’essentiel du travail d’un agent, mais pas tout ce que propose Claude Code. Dans sa propre interface, tu retrouves tout : les commandes `/`, les raccourcis, les dialogues, `/model`, `Maj+Tab` pour changer de mode… C’est utile quand tu veux travailler comme tu le ferais dans un terminal, tout en gardant l’agent dans la liste d’Escouade avec ses statuts, ses notifications et son coût.

## Passer d’une vue à l’autre

Dans l’en-tête de la conversation d’un agent, les deux segments « Escouade » et « Claude Code » choisissent la vue : « Claude Code » lance l’interface de Claude Code dans un terminal qui prend la place de la conversation, « Escouade » te ramène à la conversation. La vue affichée est nommée, l’autre n’est qu’une icône. Le menu de la carte de l’agent, dans la barre latérale (clic droit), propose la même chose : « Vue Claude Code » ou « Vue Escouade ». La session est la même dans les deux sens : Escouade arrête le processus de l’une des vues et reprend la session dans l’autre (`--resume`), donc l’agent n’oublie rien. Le choix est gardé pour chaque agent. L’interface démarre quand tu affiches l’agent (le terminal dit alors « Démarrage de Claude Code… »), y compris après un redémarrage d’Escouade, et reprend sa session. Un agent en remote control fait exception : au démarrage d’Escouade, son interface est lancée en fond pour qu’il soit joignable (voir [plus bas](#le-remote-control-dans-cette-vue)).

Le passage à la vue Claude Code est refusé dans deux cas, avec un message ; le retour à la vue Escouade, lui, n’est refusé dans aucun :

- **L’agent travaille encore** : change de vue quand son tour est fini.
- **L’agent travaille sur un ticket** : un agent de ticket reste dans la vue d’Escouade, que son tableau pilote (voir [le Kanban](board)). Ni le bouton ni l’entrée du menu de sa carte n’existent pour lui, ni pour un agent archivé.

Si l’interface s’arrête (tu as tapé `/exit`, ou le processus a été tué), le terminal affiche « Claude Code s’est arrêté. » et un bouton « Relancer » ; si elle ne démarre pas, « Claude Code n’a pas démarré : » suivi de l’erreur, avec le même bouton. Archiver ou supprimer l’agent ferme aussi son terminal.

## Ce qu’Escouade continue de suivre

L’interface de Claude Code est un terminal, mais Escouade sait toujours ce que fait l’agent. Il le lit de deux façons : les hooks de Claude Code, que l’interface appelle sur ta machine (`127.0.0.1`, avec un jeton propre à chaque interface), et le transcript de la session, que l’app relit en continu. Tes propres hooks restent actifs.

Grâce à cela, un agent en vue Claude Code garde :

- **sa pastille et ses statuts** : « En cours », « Question » quand il attend une autorisation ou une réponse, « Terminé » à la fin d’un tour ;
- **ses notifications** : le clignotement, le carillon et la notification système quand il attend une autorisation ou une réponse, puis quand il termine (voir [les notifications](notifications)) ;
- **sa conversation** : ce qui se passe dans l’interface s’écrit aussi dans la conversation d’Escouade, que tu retrouves en revenant à la « Vue Escouade » ;
- **la recherche** (`Ctrl+K`, voir [suivre ses agents](overview#rechercher-dans-les-conversations)) ;
- **le bandeau « Plan »** (voir [l’avancée d’un plan](plan)) ;
- **ses sous-agents** : leur liste dans la barre latérale et, en lecture seule, leur conversation (voir [plus bas](#les-sous-agents)) ;
- **ses tâches de fond** : leur liste dans la barre latérale et leur vue, sans « Arrêter » (voir [plus bas](#les-taches-de-fond)) ;
- **ses statistiques et son coût** (voir [les statistiques](stats)). L’interface ne donne pas le coût de chaque tour : Escouade l’estime d’après les tarifs publics.

Quand l’agent attend une réponse, c’est dans le terminal que tu la donnes : la [vue d’ensemble](overview) le rappelle (« Attend ta réponse dans l’interface de Claude Code ») et son bouton « Répondre » ouvre l’agent.

## Les sous-agents

Les sous-agents d’un agent en vue Claude Code sont listés comme ailleurs, sous sa carte dans la barre latérale et dans le bandeau « Plan » (voir [les sous-agents dans la barre latérale](plan#les-sous-agents-dans-la-barre-laterale)). Un clic sur l’un d’eux ouvre sa conversation à la place du terminal ; le fil d’Ariane, ou `Échap`, ramène à l’agent.

Cette conversation est en **lecture seule**. Escouade la lit dans le transcript que Claude Code tient pour ce sous-agent, à côté de celui de l’agent : elle est lue à l’ouverture de la vue, relue toutes les 2 secondes tant que le sous-agent tourne, puis une dernière fois quand il finit. La vue le dit en haut (« Lecture seule : son transcript, tel que Claude Code l’écrit. On agit sur ce sous-agent dans l’interface de Claude Code. »). Il n’y a ni « Arrêter » ni « Lui écrire » : pour arrêter un sous-agent ou lui écrire, passe par l’interface de Claude Code, dans le terminal. Si Escouade ne trouve pas ce transcript (Claude Code ne l’a pas encore écrit, ou le range ailleurs), la vue dit « Pas encore de message de ce sous-agent. »

Dans cette vue, aucun signal ne dit à Escouade qu’un sous-agent en arrière-plan a fini. Escouade le déduit de ce que Claude Code rejoue à l’agent dans son transcript : le rapport que le sous-agent lui rend, ou la notification de sa fin. Tant que ce message n’est pas arrivé, la ligne du sous-agent reste « En cours ». Son rapport est celui qu’il a rendu à l’agent.

## Les tâches de fond

Les tâches que Claude Code lance en arrière-plan pour un agent en vue Claude Code (commandes, surveillances…) sont listées comme ailleurs, sous sa carte dans la barre latérale, leur nombre sur la carte (voir [les tâches de fond](plan#les-taches-de-fond)). Un clic sur l’une d’elles ouvre sa vue à la place du terminal ; le fil d’Ariane, ou `Échap`, ramène à l’agent.

Dans cette vue, aucun signal ne les annonce à Escouade. Elle lit le début d’une tâche dans le résultat de l’outil que Claude Code écrit dans le transcript de l’agent, avec la commande de l’appel, et sa fin dans la notification qu’il y rejoue. Tant que cette notification n’est pas arrivée, la ligne de la tâche reste « En cours », et l’heure de départ n’est pas exacte à la seconde. Il n’y a pas de « Arrêter » : la vue dit « Cette tâche s’arrête depuis l’interface de Claude Code. », et tu arrêtes la tâche dans le terminal. La sortie se lit dans le fichier de sortie que Claude Code écrit pour la tâche (la vue dit « lue dans le fichier de sortie »), puisqu’aucune requête ne peut la demander à Claude Code. Par le [serveur MCP](mcp#les-sous-agents-et-les-taches-de-fond-d-un-agent), Claude peut les lister et lire leur sortie, mais `stop_background_work` et `message_subagent` sont refusés pour un agent en vue Claude Code : on y arrête une tâche et on y écrit à un sous-agent depuis l’interface de Claude Code.

## Écrire à un agent dans cette vue

Le champ de saisie d’Escouade n’est pas affiché : tu écris dans le terminal, comme dans Claude Code. Pour joindre une image, par exemple une capture d’écran copiée, colle-la dans le terminal (`Ctrl+V`, `Cmd+V` sous macOS) ou dépose-la dessus : Escouade l’enregistre dans un fichier et tape son chemin, que Claude Code prend pour l’image (« [Image #1] »). Du texte collé est tapé tel quel.

Un autre fichier (une archive, un tableur, un document…) se joint de la même façon : colle-le dans le terminal, ou dépose-le dessus (un cadre en pointillés montre la zone de dépôt). Escouade le copie toujours, et ne lit jamais le fichier à son emplacement d’origine : l’interface de Claude Code est lancée une fois, et seul le dossier de l’agent y est lisible sans demander. Elle tape ensuite le chemin de la copie (`/mnt/c/…` pour un Claude Code dans WSL), que Claude lit avec ses outils. La copie est dans le dossier de l’agent, avec celles du chat (voir [les pièces jointes](chat#pieces-jointes)). La taille maximale des [réglages](settings-claude-code#pieces-jointes) s’applique ; l’avertissement de taille, lui, n’existe pas ici, faute de puce.

Pour dicter plutôt que taper, clique sur le micro en haut à droite du terminal (ou appuie sur `Ctrl+Maj+Espace`, `⇧⌘Espace` sous macOS) : un clic commence et un second finit, un appui long dicte tant qu’il est maintenu. Le texte est tapé dans le prompt de Claude Code comme un collage, à son curseur, et n’est pas validé : tu le relis, puis tu appuies sur `Entrée`. La transcription se fait sur ta machine (voir [dicter un message](chat#dicter-un-message)).

Un message envoyé à l’agent d’ailleurs, par exemple par le [serveur MCP](mcp), est tapé dans son interface. Il est refusé tant que l’agent attend une réponse dans son terminal (« L’agent attend une réponse dans son terminal : réponds-y d’abord. »). Il l’est aussi pour une interface lancée en fond que personne n’a encore affichée, tant qu’elle n’est pas prête (voir [plus bas](#le-remote-control-dans-cette-vue)). Une interruption demandée depuis Escouade devient la touche `Échap` dans l’interface.

## Le remote control dans cette vue

Le [remote control](remote-control) suit les mêmes règles que dans la vue Escouade : tu l’actives par un clic droit sur la carte de l’agent ou par un réglage (celui d’un projet ou le réglage global), et l’agent est accessible depuis claude.ai et l’app Claude sous le nom `projet · agent`. Le passage à la vue Claude Code n’est pas refusé quand il est activé. C’est l’interface de Claude Code qui tient alors la session distante.

- **Le lancement.** L’interface d’un agent en remote control est lancée avec `--remote-control=<projet · agent>` : Claude Code affiche le lien de la session. Quand l’interface reprend sa session (redémarrage d’Escouade, relance), Claude Code rattache la même session distante, avec le même lien. Une interface fermée archive sa session distante.
- **Au démarrage d’Escouade.** L’interface d’un tel agent est lancée en fond, sans que tu l’ouvres. Ouvrir l’agent affiche l’interface déjà lancée ; si son remote control s’éteint avant, elle s’arrête, sauf en plein tour. Ces lancements se font au plus 3 à la fois : un démarrage garde sa place jusqu’à ce que l’interface soit prête, 15 secondes au plus.
- **Un changement.** Escouade ne peut pas allumer ni couper le remote control d’une interface déjà ouverte : un changement vaut au prochain lancement de l’interface. Après un clic droit, un message le dit ; pour un réglage, c’est sa description. D’ici là, la commande `/remote-control` de Claude Code le fait.
- **Le menu de la carte.** Il propose « Activer le remote control » ou « Désactiver le remote control ». « Ouvrir sur claude.ai » et « Copier le lien claude.ai » sont proposés aussi, une fois l’interface connectée : le lien vient de l’interface, qui l’affiche dans la sienne.
- **L’icône de la carte.** Son infobulle dit où en est l’interface : aucune interface lancée (pas joignable pour l’instant), connexion…, connecté, au prochain lancement de l’interface (ouverte avant que le remote control s’allume), ou pas ouvert par Claude Code (compte sans abonnement claude.ai, réseau, proxy ou règle de l’organisation). Éteint pour Escouade mais encore actif dans l’interface ouverte, le remote control garde une icône barrée, dans la couleur d’avertissement, jusqu’au prochain lancement, si l’interface a vraiment eu son lien. Le détail est dans [le remote control](remote-control#l-icone-d-une-interface).
- **Un message refusé.** Tant qu’une interface lancée en fond n’a jamais été affichée et n’est pas prête (écran du premier lancement, dossier à approuver), un message de la fenêtre ou du serveur MCP est refusé : ouvre l’agent pour voir l’interface.

## Ouvrir les nouveaux agents dans cette vue

Dans Réglages › Comptes et harness, groupe « Processus », le choix « Vue des nouveaux agents », « Escouade » (par défaut) ou « Terminal du harness », fait démarrer chaque nouvel agent dans la conversation d’Escouade ou dans cette vue. Chaque agent peut ensuite changer de vue depuis son en-tête. Les agents de ticket restent toujours dans la vue d’Escouade, comme ceux d’un harness qui n’a pas de vue terminal. Voir [les réglages des harness](settings-claude-code).

## La confiance du dossier

La première fois que l’interface de Claude Code se lance dans un dossier, elle demande si tu lui fais confiance. Escouade ne répond jamais à ta place. Les worktrees créés dans le dossier des worktrees par défaut du projet (`.claude/worktrees`) héritent de la réponse donnée pour le projet. Un « Dossier des worktrees » hors du projet n’en hérite pas : l’interface pose la question au premier lancement de chaque worktree (voir [les worktrees](worktrees)).

## Avec Codex

Un agent sur Codex a la même vue : le bouton s’appelle « Vue Codex » et lance l’interface de Codex (`codex`) dans le dossier de l’agent, sur son compte, avec son modèle et son effort. Les deux vues partagent la même conversation de Codex : le terminal reprend la session (`codex resume`), et l’app-server de Codex la reprend au retour dans la vue Escouade, dans le même dossier de Codex. Ce qui change par rapport à Claude Code :

- **Le suivi.** Codex lance ses hooks comme des commandes : Escouade lui donne `escouade-hook`, installée avec l’app. Ces hooks sont déclarés pour cette seule interface, par les options de sa ligne de commande (`-c hooks.…`), et leur approbation aussi : rien n’est écrit dans ton `config.toml`, et tes propres sessions de Codex ne les voient pas. Sans le jeton qu’Escouade donne à l’interface, `escouade-hook` ne fait d’ailleurs rien. La carte de l’agent dit quand il travaille, quand il attend ton autorisation et quand il a fini ; sa conversation vient du journal de la session (`sessions/…/rollout-….jsonl` dans le dossier de Codex), relu en continu.
- **La confiance du dossier.** Sur un projet que Codex ne connaît pas, son interface demande d’abord si tu fais confiance au dossier. Escouade ne répond jamais à ta place, et ne tape aucun message tant que la question est posée : « Codex demande si tu fais confiance à ce dossier : réponds dans son terminal, puis renvoie ton message. » L’app-server de Codex, lui, enregistre cette confiance de lui-même dans ton `config.toml` la première fois qu’un agent Codex travaille dans la vue Escouade : un projet où un agent Codex a déjà tourné ne pose plus la question.
- **Les modes.** « Auto » lance l’interface avec `--approve-for-me` (le relecteur automatique de Codex), « Bypass » sans demande ni bac à sable, les autres avec `-a on-request` et le bac à sable du mode. Le mode Plan de Codex se choisit dans son interface.
- **Les consignes ajoutées** de l’agent ne sont données qu’à une nouvelle session : une session reprise garde celles qu’elle avait.
- **Interrompre** envoie `Échap`, comme chez Claude Code. Une image collée dans le terminal (PNG ou JPEG) y est tapée par son chemin.
- **Ce qui n’est pas suivi dans cette vue** : le bandeau « Plan », les sous-agents et le coût (Codex n’en donne pas).

## Avec OpenCode

Un agent sur OpenCode a sa vue : le bouton s’appelle « Vue OpenCode » et ouvre l’interface d’OpenCode (`opencode attach`) dans un terminal, branchée sur la session de l’agent, depuis l’en-tête de sa conversation ou le clic droit sur sa carte, même en plein tour. Contrairement aux autres vues, elle ne remplace pas le processus de l’agent : elle s’y ajoute.

- **Le suivi.** Escouade garde le fil : la conversation, l’état de la carte, les notifications et le coût restent à jour pendant que tu travailles dans l’interface, et un message tapé là y apparaît avec tout son tour. Une permission répondue dans l’interface se ferme dans Escouade, et l’inverse.
- **Le mot de passe** du serveur d’OpenCode passe par l’environnement de l’interface, jamais par sa ligne de commande.
- **Revenir à la vue Escouade**, ou fermer le terminal, n’arrête que l’interface : le serveur d’OpenCode continue pour les autres agents.
- **Une interface dont l’agent n’a plus de processus** (arrêté, relancé sur un autre serveur) s’arrête, et « Relancer » la rebranche.

## Avec Copilot CLI

Un agent sur Copilot CLI a la même vue : le bouton s’appelle « Vue Copilot CLI » et lance l’interface de Copilot (`copilot`) dans le dossier de l’agent, sur son compte, avec son modèle et son effort. Les deux vues partagent la même session de Copilot : le terminal la reprend (`copilot --resume`), et le serveur de Copilot la reprend au retour dans la vue Escouade, dans le même dossier de Copilot (`COPILOT_HOME`). Ce qui change par rapport à Claude Code :

- **Le suivi.** Copilot lance ses hooks comme des commandes : Escouade lui donne `escouade-hook`, installée avec l’app, dans un plugin propre à l’agent, passé à cette seule interface par sa ligne de commande (`--plugin-dir`). Rien n’est écrit dans ton dossier de Copilot (ni `hooks/`, ni `settings.json`), et tes propres sessions de Copilot ne les voient pas. Sans le jeton qu’Escouade donne à l’interface, `escouade-hook` ne fait d’ailleurs rien, même avec ton `GH_TOKEN` dans son environnement. La carte de l’agent dit quand il travaille, quand il attend ton autorisation, ta réponse à une question ou ton avis sur un plan, et quand il a fini ; sa conversation et son coût viennent du journal de la session (`session-state/<session>/events.jsonl` dans le dossier de Copilot), relu en continu.
- **La confiance du dossier.** Sur un dossier que Copilot ne connaît pas, son interface demande à chaque lancement si tu fais confiance au dossier. Escouade ne répond jamais à ta place, et ne tape aucun message tant qu’il ne sait pas la question passée : « Copilot demande si tu fais confiance à ce dossier : réponds dans son terminal, puis renvoie ton message. » « Yes, and remember this folder » évite la question aux lancements suivants ; avec « Yes » seul, rien n’est enregistré, et Escouade attend que tu aies écrit toi-même un premier message dans le terminal.
- **Les modes.** « Bypass » lance l’interface avec `--allow-all`, « Plan » en mode plan, « Édits auto » avec les écritures autorisées (`--allow-tool=write`), « Auto » en autopilot avec les écritures, `git add` et `git commit` autorisés, comme Escouade le fait dans sa vue.
- **Les consignes ajoutées** de l’agent ne sont pas données à l’interface : Copilot n’a pas d’option pour elles, et les reprend seulement dans la vue Escouade.
- **Une copie** (« Dupliquer la conversation ») est faite par Copilot avant que l’interface ne démarre, puis reprise par elle.
- **Interrompre** envoie `Échap` deux fois, comme Copilot le demande (« esc again to interrupt »).
- **Ce qui n’est pas suivi dans cette vue** : le bandeau « Plan » (les tâches de Copilot).

## Avec Cursor CLI

Un agent sur Cursor CLI a aussi sa vue : le bouton s’appelle « Vue Cursor CLI » et lance l’interface de Cursor (`agent`) dans le dossier de l’agent, avec son modèle et son mode, dans le dossier de configuration de l’agent : ses règles « Pour cet agent » valent dans les deux vues. Ce qui change par rapport à Claude Code :

- **Deux conversations reliées.** Cursor tient les conversations de son interface à part de celles qu’Escouade pilote : l’interface ne reprend pas l’historique de la session de l’agent, ni l’inverse. Escouade les relie : en ouvrant la vue, il écrit ce qui s’est dit jusque-là dans une règle que l’interface lit (`.cursor/rules/escouade-relais.mdc` dans le dossier de l’agent, gardée hors de git et retirée au retour) ; au retour dans la vue Escouade, ce qui s’est dit dans l’interface part avec ton message suivant. L’interface garde sa propre conversation d’une ouverture à l’autre (`agent --resume`).
- **Un suivi limité.** Cursor ne prévient pas Escouade par des hooks fiables : Escouade lit le journal de l’interface (`~/.cursor/projects/<dossier>/agent-transcripts/`). La carte de l’agent dit quand il travaille et quand il a fini, avec la notification de fin de tour, et sa conversation montre tes messages, ses réponses et ses outils. Escouade ne sait pas quand Cursor attend ton autorisation ou ta réponse, ni le résultat de ses commandes et de ses modifications, ni les tokens : la bande « Suivi limité », au-dessus du terminal, le rappelle. Regarde le terminal.
- **La confiance du dossier.** Sur un dossier qu’il ne connaît pas, Cursor demande d’abord si tu lui fais confiance (« Trust this workspace »). Escouade ne répond jamais à ta place, et ne tape aucun message tant que la question est posée.
- **Les modes.** « Plan » lance l’interface en mode plan, « Bypass » en « Run Everything » (`--force`) ; « Demander » et « Accepter les modifications » laissent Cursor demander ses commandes.
- **Le shell** de Cursor reste PowerShell sous Windows, même si Escouade a été lancé depuis Git Bash.
- **Interrompre** envoie `Ctrl+C`, comme dans l’interface.
- **Ce qui n’est pas suivi dans cette vue** : le bandeau « Plan », les sous-agents, les permissions et les questions en attente, le résultat des outils, les tokens.

## Avec Mistral Vibe

Un agent sur Mistral Vibe a aussi sa vue : le bouton s’appelle « Vue Mistral Vibe » et lance l’interface de Vibe (`vibe`) dans le dossier de l’agent, avec son modèle et son mode, dans le `VIBE_HOME` de l’agent. Les deux vues partagent la même session : l’interface la reprend (`vibe --resume`) avec tout son historique, et son `vibe-acp` la reprend au retour dans la vue Escouade, sans rien perdre ni rien relayer. Ce qui change par rapport à Claude Code :

- **Le suivi.** Escouade lit le journal de la session, là où Vibe la garde (`logs/session/unified/<session>/journal/` du `VIBE_HOME` de l’agent), d’un segment au suivant : tes messages, ses réponses, ses outils et leur fin, l’autorisation ou la question qu’il attend, et la fin de chaque tour. La carte de l’agent dit quand il travaille, quand il attend ta réponse dans le terminal, et quand il a fini. Ses hooks (`pre_tool`, `post_tool`, `post_agent`) lancent `escouade-hook`, installée avec l’app, qui dit à Escouade la session où l’interface travaille ; ils sont écrits dans le `VIBE_HOME` de l’agent à l’ouverture de la vue et retirés au retour, jamais dans ton `~/.vibe`.
- **Une session pas encore écrite.** Vibe ne garde une session qu’après un premier message : celle d’un agent qui n’a encore rien dit n’est pas reprise, l’interface en ouvre une nouvelle, et ses hooks la donnent à l’agent.
- **Le serveur MCP d’Escouade** passe par les variables de l’interface (`VIBE_MCP_SERVERS`, et le jeton dans une autre variable), jamais sur sa ligne de commande ni dans un fichier.
- **La confiance du dossier.** Escouade l’écrit dans le `VIBE_HOME` de l’agent : Vibe ne pose pas la question.
- **Les modes.** « Demander » lance son agent `ask`, « Édits auto » `accept-edits`, « Bypass » `auto-approve` ; « Plan » demande, comme « Demander », Vibe n’ayant pas de mode plan. Pour un projet privé, les chemins des autres projets restent refusés à ses outils de fichiers, et « Bypass » reste en `ask`.
- **Interrompre** envoie `Échap`, comme dans l’interface.
- **Ce qui n’est pas suivi dans cette vue** : le bandeau « Plan », le coût et les tokens, la réflexion (réglée par `/thinking` dans l’interface).

## Avec Antigravity CLI

Un agent sur Antigravity CLI a aussi sa vue : le bouton s’appelle « Vue Antigravity CLI » et lance l’interface d’Antigravity (`agy`) dans le dossier de l’agent, avec son modèle, son effort et son mode, dans le dossier personnel de l’agent, celui où sa conversation est gardée. Les deux vues partagent la même conversation : l’interface la reprend (`agy --conversation`) avec tout son historique, et `agy -p` la reprend au retour dans la vue Escouade, sans rien perdre ni rien relayer. Ce qui change par rapport à Claude Code :

- **Le suivi.** Escouade lit le transcript de la conversation (`brain/<conversation>/.system_generated/logs/transcript_full.jsonl` sous le `.gemini` de l’agent), une ligne par étape : tes messages, ses réponses, ses outils et leur fin, la question qu’il te pose, et la fin de chaque tour. La carte de l’agent dit quand il travaille, quand il attend ta réponse à sa question, et quand il a fini. Ses hooks (`SessionStart`, `PreToolUse`, `PostToolUse`, `Stop`) lancent `escouade-hook`, installée avec l’app ; ils sont écrits dans le dossier de l’agent (`.gemini/config/hooks.json`) à l’ouverture de la vue et retirés au retour, jamais dans ton `~/.gemini`.
- **Une conversation pas encore écrite.** Antigravity ne garde le transcript d’une conversation qu’après un premier message : celle d’un agent qui n’a encore rien dit n’est pas reprise, l’interface en ouvre une nouvelle, et son premier hook la donne à l’agent.
- **Le serveur MCP d’Escouade** est dans la config MCP du dossier de l’agent, son jeton écrit dans ce fichier (Antigravity n’y développe aucune variable), jamais sur sa ligne de commande.
- **Ses premiers écrans.** Escouade écrit le choix des couleurs et la confiance du dossier dans le dossier de l’agent : Antigravity ne pose pas la question. L’adresse de ton compte Google n’est pas affichée dans son en-tête.
- **Les modes.** « Demander » est son mode par défaut, « Accepter les modifications » `accept-edits`, « Plan » `plan`, « Bypass » ses règles qui permettent tout ; les chemins d’un projet privé restent refusés à ses outils de fichiers.
- **Interrompre** envoie `Échap`, comme dans l’interface : ce qu’il a écrit jusque-là finit le tour.
- **Ce qui n’est pas suivi dans cette vue** : une autorisation qu’il te demande dans le terminal (aucun hook ni le transcript ne la disent : la carte reste « travaille » jusqu’à ta réponse), le bandeau « Plan », le coût et les tokens, la réflexion.

## Avec Gemini CLI

Un agent sur Gemini CLI a aussi sa vue : le bouton s’appelle « Vue Gemini CLI » et lance l’interface de Gemini (`gemini`) dans le dossier de l’agent, avec son modèle et son mode, dans le dossier de configuration de l’agent (`GEMINI_CLI_HOME`), celui où sa session est gardée. Les deux vues partagent la même session : l’interface la reprend (`gemini --resume`) avec tout son historique, et `gemini --acp` la reprend au retour dans la vue Escouade. Une session qui n’a encore rien écrit commence sous l’identifiant qu’Escouade lui donne (`--session-id`). Ce qui change par rapport à Claude Code :

- **Le suivi, par ses hooks.** Son journal réécrit un message à chaque changement : Escouade ne le lit pas. Ses hooks lancent `escouade-hook`, installée avec l’app : `BeforeAgent` (ton message), `BeforeTool` et `AfterTool` (chaque outil et sa fin), `Notification` (une autorisation qu’il te demande dans le terminal : la carte attend ta réponse), `AfterAgent` (la fin du tour, avec toute sa réponse). Ils sont écrits dans les réglages du dossier de l’agent à l’ouverture de la vue et retirés au retour, jamais dans ton `~/.gemini`.
- **Le serveur MCP d’Escouade** est dans les réglages du dossier de l’agent (`mcpServers`), son jeton écrit dans ce fichier, jamais sur sa ligne de commande.
- **Les modes.** « Demander » est son mode `default`, « Édits auto » `auto_edit`, « Plan » `plan`, « Bypass » `yolo` ; les chemins d’un projet privé restent refusés par ses règles (`--policy`).
- **Interrompre** envoie `Échap`, comme dans l’interface ; aucun hook ne dit ce tour arrêté, Escouade le finit lui-même.
- **Ce qui n’est pas suivi dans cette vue** : ses textes entre deux outils (seule sa réponse entière arrive, en fin de tour), une limite d’usage (son interface propose elle-même d’attendre ou de changer de modèle), le bandeau « Plan », le coût et les tokens.

## Sous Windows avec WSL

Quand Claude Code est installé dans WSL, Escouade le lance par `wsl.exe` (voir [les réglages des harness](settings-claude-code)). Le suivi en direct de la vue Claude Code demande alors le réseau miroir de WSL : sans lui, les hooks de l’interface n’atteignent pas l’app. Les réglages le signalent sous le chemin de `claude` (« Le réseau de WSL est en NAT… »). Pour passer en mode miroir, ajoute ceci à `%UserProfile%\.wslconfig`, puis lance `wsl --shutdown` :

```ini
[wsl2]
networkingMode=mirrored
```
