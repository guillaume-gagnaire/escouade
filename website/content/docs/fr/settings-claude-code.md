# Réglages : Harness

L’onglet « Harness » (« Claude Code » jusqu’à la 1.7) liste les harness des agents, avec leur exécutable et leur version, et règle la vue des nouveaux agents, la vie des processus, les pièces jointes, le remote control et le serveur qui laisse Claude piloter Escouade.

## Chaque harness

Un groupe par harness, sous son nom :

| Ligne | Ce qu’elle dit |
|---|---|
| « Chemin de l’exécutable » | Le programme du harness (voir ci-dessous pour Claude Code). |
| « Version » | La version trouvée, seule : une version plus ancienne que celles testées est refusée, avec sa raison et la commande qui la met à jour ; une plus récente est utilisée, sans avertissement. Quand aucun compte de ce harness n’est connecté, la ligne ajoute « Aucun compte connecté : ajoute ou connecte un compte de ce harness dans l’onglet Comptes. » |
| « Modèles » (OpenCode) | Les modèles des fournisseurs connectés à OpenCode, avec une recherche : les favoris d’abord, puis un groupe par fournisseur. L’étoile d’un modèle en fait un favori, qui vient en premier dans le menu des modèles de ses agents. |
| « Installation » | Pour un harness introuvable : la commande qui l’installe (Claude Code : `irm https://claude.ai/install.ps1 \| iex` sous Windows, `curl -fsSL https://claude.ai/install.sh \| bash` sous macOS et Linux). Pour Claude Code, la ligne dit aussi si une installation par npm a été interrompue, avec la commande qui la répare (voir le [dépannage](troubleshooting)). |

Les comptes d’un harness ne se connectent pas ici mais dans l’onglet [« Comptes »](settings-accounts) : chacun y a son état et son bouton « Se connecter… ». Un harness introuvable n’apparaît que dans cet onglet. Au premier lancement, si aucun n’est trouvé, l’écran d’accueil dit « Aucun harness trouvé » et donne la commande d’installation de chacun ; Escouade marche dès qu’un harness est trouvé.

Dès que deux harness sont installés, le projet a son « Harness par défaut » (Réglages › Projets), le bouton « ▾ » à côté de « Nouvel agent » propose « Nouvel agent sur … » pour chacun, et la fenêtre d’un ticket a son champ « Harness » (celui du projet par défaut), que le pilote auto suit. Dès que des agents tournent sur deux harness, le nom du harness de chacun s’affiche à côté de lui : sur sa carte, dans la vue d’ensemble, dans son en-tête et sur la carte de son ticket. Les menus du composer (modèles, effort, modes, commandes /) sont ceux du harness de l’agent.

## Exécutable de Claude Code

Le groupe « Claude Code » commence par un champ.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Chemin de l’exécutable » (vide = détection automatique) | Le `claude` qu’Escouade lance pour les agents. Un compte qui a son propre exécutable, dans l’onglet [« Comptes »](settings-accounts), lance le sien. | Vide |

### Où Escouade cherche

Champ vide, Escouade cherche `claude` dans le `PATH`, puis aux emplacements habituels des installeurs :

- sous Windows : `~\.local\bin\claude.exe`, `~\.claude\local\claude.exe` et `~\AppData\Roaming\npm\claude.cmd` ;
- sous macOS et Linux : `~/.local/bin/claude`, `~/.claude/local/claude`, `/opt/homebrew/bin/claude` et `/usr/local/bin/claude`.

Sous Windows, s’il n’en trouve aucun, Escouade cherche un Claude Code installé dans WSL, dans la distribution que nomme « Distribution WSL » dans l’onglet [« Terminaux »](settings-terminals) (vide = la distribution par défaut).

Champ rempli, tu peux écrire :

- un chemin complet ;
- un chemin qui commence par `~`, le dossier personnel (Windows, macOS et Linux) ;
- sous Windows, un chemin sans son `.exe` ni son `.cmd` ;
- sous Windows, un Claude Code installé dans WSL : `~/.local/bin/claude`, `/home/<utilisateur>/.local/bin/claude` ou `\\wsl.localhost\<distribution>\home\<utilisateur>\.local\bin\claude` (`\\wsl$\<distribution>\…` marche aussi).

### La ligne sous le champ

Un quart de seconde après que tu as fini de taper, Escouade cherche le `claude` du champ et dit ce qu’elle a trouvé sous le champ :

- « Trouvé : » suivi du chemin ;
- « Trouvé dans WSL (Ubuntu) : » suivi du chemin Linux, pour un Claude Code de WSL ;
- « Introuvable. » : sous Windows, la ligne rappelle alors les formes de chemin acceptées, celles de WSL comprises ; ailleurs elle dit « Introuvable. Indique le chemin de claude. ».

Champ vide, le texte grisé dit déjà si `claude` est trouvé : « claude (trouvé dans le PATH) » ou « introuvable — indique le chemin de claude.exe ». Après « Enregistrer », un nouveau chemin ou une nouvelle distribution WSL fait chercher à nouveau, pour mettre à jour le bandeau du [Kanban](board) et le texte du champ.

> [!WARNING]
> Si le `claude` trouvé est dans WSL et que le réseau de WSL est en NAT, un avertissement s’ajoute : le serveur MCP d’Escouade et le suivi de la [vue Claude Code](claude-code-view) n’atteignent pas l’app. Passe WSL en mode miroir (`networkingMode=mirrored` sous `[wsl2]` dans `%UserProfile%\.wslconfig`), puis lance `wsl --shutdown`.

## Mise à jour de Claude Code

Les `claude` qu’Escouade lance ne se mettent pas à jour eux-mêmes : Escouade arrête ses processus avec tout ce qu’ils ont lancé, et un arrêt pouvait couper une mise à jour de Claude Code en cours, au point de faire disparaître la commande `claude` (voir le [dépannage](troubleshooting)). C’est Escouade qui le met à jour.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Mettre à jour Claude Code automatiquement » | Escouade lance `claude update` quelques secondes après son démarrage, puis une fois par jour tant qu’il est ouvert, à part de ses agents : ni l’arrêt d’un agent ni la fermeture d’Escouade ne l’interrompent. | Activé |

La ligne sous l’interrupteur dit où en est la dernière mise à jour : « À jour (2.1.302). », « Mis à jour en 2.1.302 (depuis la 2.1.301). », « Mise à jour en cours… » ou « Échec de la mise à jour : » suivi de l’erreur. Le [journal de l’app](troubleshooting) le note aussi.

Désactivé, plus rien ne met Claude Code à jour : lance `claude update` toi-même de temps en temps. Si tu as coupé la mise à jour dans Claude Code lui-même (`DISABLE_AUTOUPDATER` ou `DISABLE_UPDATES` dans ton environnement ou dans le `env` de son `settings.json`, ou `autoUpdates: false`), Escouade la laisse coupée, et la ligne le dit : « Coupée dans Claude Code lui-même (DISABLE_AUTOUPDATER) : Escouade ne le met pas à jour non plus. »

## Nouveaux agents

Le modèle, l’effort et le mode avec lesquels partent les nouveaux [agents](managing-agents) se règlent compte par compte, dans sa fiche de [Réglages › Comptes](settings-accounts), parmi ceux de son harness : il n’y a plus de réglage commun ici. Dans le champ de message d’un agent, tu peux les changer pour cet agent seul. Les tickets du Kanban prennent ceux du ticket, sinon de l’onglet [« Kanban »](settings-board) du projet, sinon du compte qui les prend.

### Les niveaux d’effort

| Niveau | Sens |
|---|---|
| « Bas » | Réponses rapides, peu de réflexion |
| « Moyen » | Équilibré |
| « Élevé » | Réflexion approfondie |
| « Très élevé » | Réflexion très approfondie |
| « Max » | Réflexion maximale, plus de tokens |

### Les modes de permission

| Mode | Sens |
|---|---|
| « Auto » | Un classifieur approuve les actions sûres et demande pour le reste |
| « Demander » | Claude demande ton accord avant chaque action sensible |
| « Plan » | Claude analyse et propose un plan sans rien modifier |
| « Édits auto » | Les modifications de fichiers sont acceptées sans demander |
| « Bypass » | Aucune demande de permission (à réserver aux environnements sûrs) |

Les demandes d’autorisation et leur traitement sont décrits dans [la conversation](chat), et le mode « Plan » dans [le plan](plan).

> [!NOTE]
> Avec Haiku, le choix de l’effort est grisé dans le champ de message de l’agent (« Haiku ne gère pas l’effort »), comme le mode « Auto » (« indisponible avec ce modèle ») quand le Claude Code du compte de l’agent dit que son Haiku ne le prend pas : c’est Haiku 4.5, que Claude Code fait tourner jusqu’à la 2.1.292 ; depuis la 2.1.293, c’est Haiku 5.5, qui le prend.

## Processus

| Réglage | Effet | Par défaut |
|---|---|---|
| « Reprise automatique après la limite d’usage » | « continue » est envoyé à l’agent une fois le quota réinitialisé. | Activé |
| « Reprendre sur un autre compte un agent de ticket arrêté par la limite » | Sinon il attend la remise à zéro de son compte. Absent tant qu’il n’y a qu’un compte. | Désactivé |
| « Continuer sur le harness suivant quand tous les comptes d’un harness sont épuisés » | L’agent passe seul sur le harness de secours suivant de son projet, et le pilote auto le suit pour les tickets ; sinon il attend la remise à zéro. Le harness quitté n’est pas repris avant sa remise à zéro, et l’agent n’y revient jamais seul. Absent tant qu’un seul harness est trouvé. | Désactivé |
| « Modèle de passation · <harness> » | Le modèle d’un agent qui passe sur ce harness. Un par harness, affiché avec le réglage précédent. | Le meilleur |
| « Vue des nouveaux agents » | « Escouade » : un nouvel agent tourne dans la conversation d’Escouade. « Terminal du harness » : il tourne dans l’interface de son harness elle-même, dans un terminal (la vue Claude Code). Affiché tant qu’un harness a une vue terminal ; les agents d’un harness qui n’en a pas restent dans la vue d’Escouade. | Escouade |
| « Les agents tiennent une liste de tâches » | Les agents reçoivent l’outil de liste de tâches de Claude Code. | Activé |
| « Arrêter les processus des agents inactifs après » (minutes, 0 = jamais) | Le processus Claude d’un agent inactif est arrêté après ce délai. | 30 |

### Reprise automatique

Quand un agent s’arrête sur la limite d’usage, Escouade prévoit de lui écrire « continue » à la remise à zéro du quota. Désactiver le réglage annule aussi les reprises déjà prévues. La limite et les quotas sont expliqués dans [l’usage](usage).

### Reprise sur un autre compte

Avec plusieurs comptes, un agent de ticket arrêté par la limite repart tout seul, avec sa session, sur le compte que choisirait Escouade, quand un autre est utilisable, au lieu d’attendre la remise à zéro de son compte. Voir [les comptes](accounts) et l’onglet [« Comptes »](settings-accounts).

### Vue Claude Code

Chaque agent change de vue depuis son en-tête, et ce réglage ne fait que choisir la vue de départ. Les agents de ticket restent dans la vue d’Escouade. Voir [la vue Claude Code](claude-code-view).

### Liste de tâches

Si l’agent tient une liste, Escouade montre son avancée dans le bandeau « Plan » ; le modèle n’est pas obligé de s’en servir. Le réglage s’applique aux agents lancés ensuite. Voir [le plan](plan).

### Processus inactifs

Le champ est un nombre de minutes ; 0 veut dire jamais, et un champ vide vaut 0. La session d’un agent dont le processus a été arrêté reste reprenable. Un agent en remote control n’est jamais arrêté ainsi, pour rester joignable depuis claude.ai.

## Pièces jointes

Ce groupe décide de ce qui arrive aux fichiers joints envoyés par chemin (voir [les pièces jointes](chat#pieces-jointes)). Il ne touche pas à ce que Claude lit dans le message : images, PDF et textes dans leurs limites.

| Réglage | Effet | Par défaut |
|---|---|---|
| « Taille maximale d’un fichier joint » (Mo, 0 = aucune limite) | Au-delà, le fichier est refusé. | 0 |
| « Avertir au-delà de » (Mo, 0 = jamais) | Au-delà, la puce du fichier porte un avertissement ; le fichier se joint quand même. | 200 |
| « Joindre le fichier d’origine » | Activé : un fichier déposé est joint là où il est, sans copie : Claude le lit à son emplacement, quand Escouade en connaît le chemin (sous Windows seulement : sous Linux et macOS, les fichiers sont toujours copiés). Désactivé : Escouade en garde toujours une copie. Les tickets et la vue Claude Code copient dans tous les cas. | Activé |
| « Texte de plus de 256 Ko » | « Joint par son chemin », comme les autres fichiers, ou « Refusé », comme avant. | « Joint par son chemin » |
| « Purger les copies après » (jours, 0 = tant que leur propriétaire existe) | Les copies plus vieilles que ce nombre de jours sont effacées, avec leurs dossiers vides, au démarrage puis chaque heure. 0 = elles restent tant que leur agent, leur projet ou leur ticket existe. | 0 |

Un champ de nombre vidé vaut 0. La taille maximale vaut pour tous les fichiers envoyés par chemin : dans le chat, dans la vue Claude Code et dans les tickets. L’avertissement n’apparaît que sur une puce (chat et tickets). La purge n’efface jamais un fichier d’origine. Les copies des messages sont dans `attachments/<agent>/`, celles d’un ticket dans `tickets/<ticket>/files/`, sous le [dossier de données](data).

## Remote control

Ce groupe rend les agents de tous les projets accessibles depuis claude.ai et l’app Claude, sans activer le [remote control](remote-control) agent par agent.

### Remote control pour tous les agents

L’interrupteur est désactivé par défaut. Allumé, chaque agent de chaque projet, existant ou nouveau (agents des tickets compris), est accessible depuis claude.ai et l’app Claude. Escouade lance tout de suite le processus `claude` de chacun, au plus 3 à la fois, et il reste lancé : il démarre avec l’app et n’est jamais arrêté pour inactivité. La session apparaît sur claude.ai sous le nom « projet · agent » (pour un agent de ticket, son nom vient du ticket). En vue Claude Code, une interface déjà ouverte ne le prend qu’à son prochain lancement. Pour mettre un agent à part, fais un clic droit sur sa carte, puis « Désactiver le remote control ».

Le réglage s’enregistre avec « Enregistrer », comme la plupart des autres. Éteint, il retire le remote control aux agents qui le tenaient de lui, sans toucher à ceux que tu as activés à la main ni à ceux que le réglage de leur projet couvre encore.

Tant qu’il est allumé, l’interrupteur « Remote control pour les agents de ce projet » de chaque projet, dans [l’onglet « Projets »](settings-projects), se montre allumé et grisé : « Activé pour tous les projets dans Réglages › Harness ». Le choix propre de chaque projet est gardé, et revient quand celui-ci est éteint.

Pour un agent en [vue Claude Code](claude-code-view), le changement vaut au prochain lancement de son interface. La page sur [le remote control](remote-control) donne le détail : d’où il vient, comment mettre un agent à part et ce qui se passe au redémarrage.

## Serveur MCP

Ce groupe laisse les agents d’Escouade, et chaque harness installé lancé hors d’Escouade (Claude Code, Codex, OpenCode, Copilot CLI, dans un terminal ou leur app), lire tes projets, tes agents et tes tickets, et créer des tickets ou lancer des agents, par le [serveur MCP](mcp) d’Escouade.

### Les agents peuvent piloter Escouade

L’interrupteur est désactivé par défaut ; c’est le seul, il vaut pour tous les harness. Il donne le serveur à chaque agent d’Escouade, de tous les projets, avec un jeton à lui : il n’y a rien à permettre dans les réglages d’un projet. Il le déclare aussi dans la config de chaque compte actif ajouté de chaque harness installé (un harness sans compte n’a la déclaration nulle part) : `~/.claude.json` (Claude Code, par `claude mcp add`), `~/.codex/config.toml` (Codex), `~/.config/opencode/opencode.json` (OpenCode), `~/.copilot/mcp-config.json` (Copilot CLI) pour le compte Principal, le reste du fichier et ses commentaires gardés. Un harness installé ou un compte ajouté plus tard est déclaré aussitôt. Il s’enregistre tout de suite, sans passer par « Enregistrer » : l’activer écrit dans ces fichiers, ce qu’un « Annuler » ne pourrait pas défaire. Le désactiver change le jeton, retire l’entrée partout (chaque fichier redevient celui d’avant, à l’octet près) et écarte le serveur des agents : ceux déjà lancés perdent les outils tout de suite.

Un agent déjà lancé reçoit les outils au prochain démarrage de son processus : sa conversation affiche alors « Outils d’Escouade au prochain démarrage » avec « Relancer », qui reprend sa session dans un nouveau processus, et une ligne sous l’interrupteur dit combien d’agents sont dans ce cas (« 1 agent déjà lancé aura les outils au prochain démarrage de son process : « Relancer » sur sa conversation. »). Voir [Les agents d’Escouade et le serveur](mcp#les-agents-d-escouade-et-le-serveur).

Interrupteur allumé, la liste dessous donne chaque déclaration :

- « Déclaration dans les harness installés… » pendant qu’Escouade écrit les entrées ;
- puis une ligne par compte de chaque harness, « Codex · compte Principal » par exemple, avec son fichier et son état, « Déclaré » ou « Pas déclaré » ;
- pour un compte où ça a échoué, « Pas déclaré pour le compte Pro : » suivi de la raison, et ce qu’il faut faire à la main : pour Claude Code la commande à lancer (« À lancer à la main dans PowerShell : », ou « … dans sh : » sous macOS) et « Copier la commande » (« Commande copiée ») ; pour les autres l’entrée à ajouter au fichier (« À ajouter à la main dans … : ») et « Copier l’entrée » (« Entrée copiée »). Le jeton est masqué à l’écran ; le bouton copie le vrai. Un fichier qui n’est pas un TOML ou un JSON valide n’est pas touché ;
- « Le serveur n’a pas démarré : » suivi de la raison, à la place de la liste.

> [!WARNING]
> Le jeton est écrit en clair dans chacun de ces fichiers. Désactiver change le jeton.

## Activité MCP

Le groupe « Activité MCP » liste ce que le serveur a reçu et répondu, les appels les plus récents en tête. Chaque ligne donne l’heure, qui a appelé (le nom d’un agent, « Claude (hors Escouade) » ou, pour une requête refusée avant de savoir qui l’envoie, « Client inconnu »), l’outil, le résultat (« Fait », « Refusé » ou « Erreur »), puis ce qui a été demandé et, pour un refus ou une erreur, pourquoi. Pour un `set_board` réussi, la ligne dit aussi ce qui a changé, champ par champ, avant et après (« model: opus → sonnet · effort: (défaut) → medium »), ou « Aucun changement ». Les 200 derniers appels sont gardés en mémoire, pas enregistrés. Le bouton « Effacer » vide la liste ; sans appel, elle dit « Aucun appel pour l’instant. ».
