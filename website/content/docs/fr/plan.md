# L’avancée d’un plan

Quand un agent suit une liste de tâches ou travaille avec des sous-agents, Escouade montre où il en est dans un bandeau « Plan » au-dessus de la conversation, sur sa carte et dans la liste de ses sous-agents. Cette page explique d’où vient la liste, ce que le bandeau affiche, les sous-agents et les tâches de fond.

## Quand le bandeau apparaît

Le bandeau « Plan » apparaît entre l’en-tête de la conversation et les messages dès que l’agent a au moins une tâche, ou au moins un sous-agent au travail. Il n’y a rien à faire pour l’afficher : c’est l’agent qui décide de tenir une liste, et Escouade ne fait que la lire.

## D’où viennent les tâches

Escouade ne crée jamais de tâche et ne modifie aucun fichier de plan : il lit ce que l’agent fait. La liste vient de l’une de ces deux sources.

- **La liste de tâches de Claude Code.** L’agent la tient avec les outils de tâches de Claude Code (`TodoWrite`, ou `TaskCreate` et `TaskUpdate`). Le réglage « Les agents tiennent une liste de tâches » (Réglages › Comptes et harness, activé par défaut) donne cet outil aux agents lancés ensuite ; le modèle n’est pas obligé de s’en servir. Voir [les réglages des harness](settings-claude-code).
- **Un plan superpowers.** Pour un agent qui n’a pas de liste de tâches, Escouade lit le plan que l’agent a écrit dans `docs/superpowers/plans/` (skills `writing-plans`, `executing-plans`, `subagent-driven-development`) et le journal d’avancement que ces skills tiennent dans `.superpowers/sdd/`, pour savoir quelles tâches sont faites. Cette lecture se fait seulement dans le dossier de l’agent (son worktree, ou le projet) et ne modifie rien. Si l’agent tient aussi sa propre liste, c’est elle qui sert de liste ; le fichier de plan n’en donne alors que le titre et le nom.

Le bandeau garde au plus 100 tâches.

## Ce que montre le bandeau

En haut, une ligne résume le plan :

- le badge « Plan » et le titre du plan : celui du fichier de plan s’il y en a un, sinon le titre du ticket de l’agent, sinon le nom de l’agent ;
- les sous-agents au travail, en petits points suivis de « 3 sous-agents actifs » (voir plus bas) ;
- le compte des tâches faites, « 2/7 tâches », et le pourcentage du plan entier, « 29 % » ;
- une flèche qui replie ou déplie la liste.

Sous cette ligne, une barre a un segment par tâche : plein pour une tâche faite, estompé pour une tâche à faire, rempli à hauteur de l’avancement pour une tâche en cours.

La liste dessous donne une ligne par tâche, numérotée (01, 02…) :

| Élément | Ce qu’il dit |
|---|---|
| Icône | ✓ pour « Terminé », un cercle qui tourne pour « En cours », `!` pour « Bloqué », un tiret pour « À faire » |
| Titre | Les mots de l’agent, tels quels |
| `↳ nom` et le modèle | Le sous-agent qui travaille sur cette tâche ; un clic sur `↳ nom` ouvre sa conversation (voir [la conversation d’un sous-agent](#la-conversation-d-un-sous-agent)) |
| Texte gris | Ce que la tâche fait en ce moment, dans les mots de l’agent (le dernier outil du sous-agent, sinon la forme « en cours » de la tâche) |
| « après 03, 05 » | Pour une tâche à faire, les numéros des tâches qu’elle attend et qui ne sont pas finies |
| Barre et pourcentage | L’avancement de la tâche : 100 % quand elle est faite, 0 % quand elle est à faire. Pour une tâche en cours dans un fichier de plan, il est calculé sur ses étapes cochées (survole le pourcentage pour voir « 2/3 étapes »). Tant qu’aucune étape n’est cochée, ou que rien ne les compte, la ligne affiche à la place le mot de l’état (« En cours », « Bloqué »). |

La liste se centre d’elle-même sur la ligne à regarder : la tâche bloquée, sinon la tâche en cours, sinon la première à faire. Si tu fais défiler la liste, elle ne se recentre pas avant six secondes.

## Une tâche bloquée

Quand l’agent attend une réponse à une question ou à une autorisation (statut « Question »), sa première tâche en cours passe à « Bloqué » : l’icône est un `!` et la ligne dit « En attente de ta réponse ». Le bandeau ne sait pas à quelle tâche la question se rapporte : il retient la première qui est en cours. Réponds à la carte dans la conversation (voir [la conversation](chat#questions-et-autorisations)).

## Les sous-agents

Un sous-agent est un agent que Claude lance lui-même pour une partie du travail. Tant qu’il tourne, il compte dans « N sous-agents actifs » et il a son point dans l’en-tête du bandeau (jusqu’à six points, empilés, les suivants sont seulement comptés). La pile de points est un seul bouton. Si un seul sous-agent tourne, un clic sur le bouton ouvre sa conversation (survole-le pour lire le titre du sous-agent), qu’il travaille sur une tâche ou non. Si plusieurs tournent, le bouton ouvre un menu qui les liste tous, y compris ceux dont le point n’est pas dessiné, chacun avec son titre et son modèle : choisis-en un pour ouvrir sa conversation. S’il travaille sur une tâche de la liste, la ligne de la tâche le nomme et dit ce qu’il fait.

Quand un agent n’a **pas** de liste de tâches mais que des sous-agents tournent, le bandeau s’appelle « Sous-agents » : il donne le titre, la pile de points et le compte, sans liste ni repli. Il disparaît quand le dernier sous-agent a fini.

Ce qu’un sous-agent dit et fait se lit dans sa propre conversation (voir plus bas), et en abrégé dans la conversation de l’agent, sur la ligne de l’outil `Agent` (voir [la conversation](chat#les-appels-d-outils)).

## Replier, déplier, redimensionner

- Un clic sur la ligne du haut (hors de la pile de points des sous-agents, qui ouvre leur conversation) replie ou déplie la liste. Par défaut, elle est dépliée quand la fenêtre fait au moins 800 px de haut, repliée sinon. Ton choix est gardé pour cet agent jusqu’à ce que tu fermes Escouade.
- Une poignée sous la liste se glisse vers le haut ou le bas pour changer sa hauteur ; un double-clic la remet à la hauteur par défaut. La hauteur choisie sert pour tous les plans et est gardée d’un démarrage à l’autre. Les messages gardent toujours au moins 160 px.

## La ligne de la carte

Dans la barre latérale, la carte d’un agent qui a des tâches affiche une barre fine et « Plan 2/7 » (ou « Plan 2/7 · 3 sous-agents » quand des sous-agents tournent). Elle permet de suivre plusieurs agents d’un coup d’œil, sans ouvrir leur conversation. Un agent qui n’a que des sous-agents, sans tâches, n’a pas cette ligne, mais ses sous-agents ont leur groupe sous la carte (voir plus bas). Tant que des sous-agents ou des tâches de fond tournent, leur nombre s’ajoute à côté de son nom, chacun avec son icône (voir [les tâches de fond](#les-taches-de-fond)).

## Les sous-agents dans la barre latérale

Sous la carte de l’agent affiché, s’il a des sous-agents, un groupe « Sous-agents » les liste, même quand l’agent n’a aucune tâche ; les autres cartes n’en montrent que le nombre au travail. Un clic sur son titre le déplie ou le replie. Par défaut il est déplié tant qu’un sous-agent tourne et replié sinon ; ton choix est gardé jusqu’à ce que tu fermes Escouade.

Déplié, le groupe montre d’abord les sous-agents qui tournent, chacun avec son titre, son modèle et ce qu’il fait en ce moment (« Édite src/a.ts »). Il en montre cinq ; « N autres en cours » déroule les suivants. Dessous, une ligne « N sous-agents finis » déplie ceux qui ont fini, du dernier fini au premier, chacun avec son état (Terminé, Échoué, Arrêté ou Interrompu) : dix au plus, puis « N de plus » pour le reste. Un sous-agent lancé par un autre est décalé d’un cran. Le groupe ne défile pas : la barre défile déjà.

La barre ne garde que les 30 derniers sous-agents d’un agent : ceux qui tournent, puis les derniers finis. Le nombre de sous-agents finis compte ceux-là ; les plus anciens se lisent encore dans la conversation de l’agent (voir plus bas).

## La conversation d’un sous-agent

Chaque sous-agent a sa conversation, que tu lis en grand, à la place de celle de l’agent.

### L’ouvrir

Un clic suffit, sur l’un de ces éléments :

- la ligne du sous-agent dans la barre latérale ;
- la pile de points du bandeau « Plan » : d’un clic pour un seul sous-agent qui tourne, par un menu de tous ceux qui tournent pour plusieurs (qu’ils travaillent sur une tâche ou non) ;
- le `↳ nom` d’une tâche ;
- le bouton au bout de la ligne de son appel (l’outil `Agent`), dans la conversation de l’agent ;
- « Ouvrir sa conversation », au-dessus d’une demande de permission ou d’une question qu’il a posée.

En haut, un fil d’Ariane « agent › sous-agent » dit où tu es. Pour un sous-agent lancé par un autre, il montre toute la chaîne, et chaque maillon ouvre le sous-agent qu’il nomme. Un clic sur le nom de l’agent, ou `Échap`, te ramène à sa conversation, et le focus retourne là où tu avais cliqué (pour la pile de points du bandeau, sur son bouton, même si tu as choisi le sous-agent dans son menu) ; un clic sur la carte de l’agent dans la barre latérale y ramène aussi. `Échap` ne fait rien tant qu’un champ contient du texte que tu as tapé : un brouillon ne se perd pas pour une touche.

### Ce que la vue montre

- **L’état** du sous-agent, à côté du fil d’Ariane : En cours, Terminé, Échoué, Arrêté, Interrompu, ou « Lancé » quand rien n’en dit plus.
- **Sa fiche** : son type, son modèle, ses tokens, ses outils utilisés et sa durée (qui court tant qu’il tourne). Dessous, sa consigne, c’est-à-dire ce qu’on lui a demandé, repliée quand elle est longue (« Voir toute la consigne »).
- **Sa conversation** : ce qu’il dit, sa réflexion, ses appels d’outils (dépliables, comme dans la conversation de l’agent), puis les demandes de permission et les questions qu’il pose. Tu y réponds comme d’habitude, au bouton ou au clavier (voir [la conversation](chat#questions-et-autorisations)). Ces demandes restent aussi dans la conversation de l’agent, précédées de « Demande du sous-agent » et de son nom.
- **Sa fin** : « Le sous-agent travaille… » tant qu’il tourne, puis son rapport, rendu en Markdown, ou « Ce sous-agent n’a pas rendu de rapport. »

Les titres, les consignes et les rapports sont les mots de l’agent, tels quels : Escouade ne les traduit pas.

### L’arrêter

« Arrêter », en haut à droite, n’est proposé que pour un sous-agent qui tourne **en arrière-plan**. Il arrête ce sous-agent seul : l’agent continue. Claude Code fait ensuite faire un tour à l’agent pour constater l’arrêt, et ce tour compte comme les autres (tokens, coût). Tant que Claude Code n’a pas donné la tâche du sous-agent, le bouton reste là, grisé, et son infobulle dit pourquoi il ne peut pas encore agir.

Un sous-agent au premier plan n’a pas de bouton : l’agent l’attend, et « Arrêter » de l’agent l’arrête avec son tour. Arrêter seul un sous-agent au premier plan n’a jamais été essayé sur un vrai flux, Escouade ne le propose donc pas. Il n’y a pas non plus de bouton dans [la vue Claude Code](claude-code-view#les-sous-agents).

### Lui écrire

Claude Code ne permet pas d’écrire directement à un sous-agent. Pour un sous-agent qui tourne **en arrière-plan**, un champ « Lui écrire, par » suivi du nom de l’agent apparaît en bas de la vue. Ton message est envoyé à l’agent, avec la demande de le transmettre tel quel au sous-agent par l’outil `SendMessage` de Claude Code ; le champ le dit sous la zone de texte. `Ctrl+Entrée` envoie le message, et ne répond à aucune demande qui attend.

> [!NOTE]
> Le message passe par l’agent. Il paraît dans sa conversation sous la forme de cette demande, il lui coûte un tour, et le sous-agent ne le reçoit qu’à son prochain outil. L’agent est seulement prié de le transmettre : rien ne l’y oblige. Dans l’essai fait sur Claude Code 2.1.300, il l’a fait et le sous-agent en a tenu compte.

Le champ n’est pas proposé pour un sous-agent au premier plan (l’agent l’attend et ne pourrait lui passer le message qu’à sa fin), ni dans la vue Claude Code, ni pour un agent archivé. Ce que tu as tapé est gardé quand le sous-agent finit ou que tu changes de vue, tant qu’Escouade reste ouvert ; le champ reste alors, l’envoi grisé, avec la mention que le sous-agent a fini.

Claude peut aussi lister les sous-agents d’un agent, lire leur rapport, arrêter un sous-agent en arrière-plan et lui écrire par le [serveur MCP](mcp#les-sous-agents-et-les-taches-de-fond-d-un-agent), avec les mêmes refus que ces boutons, y compris pour un message à un sous-agent qui ne tourne plus.

### Après coup

La conversation d’un sous-agent fini se lit encore. Escouade garde avec la conversation de l’agent la consigne, le rapport et le bilan de chaque sous-agent (état, tokens, outils, durée). La consigne et le rapport sont coupés à 16 000 octets, et le texte coupé se termine par « … ». Quand un sous-agent finit sans rendre de rapport en forme, son dernier message sert de rapport. Le rapport d’un sous-agent au premier plan est le résultat de son appel, coupé à 8 000 octets.

Sa conversation se lit donc après un redémarrage d’Escouade, et même quand la barre n’a plus sa ligne (au-delà de 30 sous-agents, ou quand le plan est reparti de zéro) : le bouton de la ligne de son appel, dans la conversation de l’agent, l’ouvre toujours. Un sous-agent qui tournait quand Escouade s’est fermé est dit « Interrompu ».

## Les tâches de fond

Claude Code peut lancer des tâches en arrière-plan pour un agent : une commande (l’outil `Bash` avec `run_in_background`), une surveillance (l’outil `Monitor`, qui suit la sortie d’une commande et prévient l’agent à chaque ligne), un workflow, un agent distant ou un coéquipier. Les sous-agents ont leur liste (voir plus haut) et ne sont pas des tâches de fond. Escouade liste ces tâches sous la carte de l’agent, ouvre le détail de chacune et montre en direct la sortie d’une commande ou d’une surveillance. Elles n’ouvrent pas le bandeau « Plan » : il ne s’occupe que des tâches de la liste et des sous-agents.

### Dans la barre latérale

Sous la carte d’un agent qui a des tâches de fond, après le groupe des sous-agents, un groupe « Tâches de fond » les liste, construit comme lui. Un clic sur son titre le déplie ou le replie. Par défaut il est déplié tant qu’une tâche tourne et replié sinon ; ton choix est gardé jusqu’à ce que tu fermes Escouade.

Déplié, le groupe montre d’abord les tâches qui tournent, dans l’ordre où elles ont été lancées : cinq, puis « N autres en cours » déroule les suivantes. Chaque ligne donne un point d’état, le titre de la tâche, sa durée (qui court tant qu’elle tourne), son type (« Commande en fond », « Surveillance », « Workflow », « Agent distant » ou « Coéquipier ») et, tant qu’elle tourne, sa ligne de commande ou la phase du workflow. Dessous, « N tâches finies » déplie celles qui ont fini, de la dernière finie à la première, chacune avec son état (Terminée, En échec, Arrêtée ou Interrompue) et, pour une commande, son code de sortie (« En échec (code 3) ») : dix au plus, puis « N de plus ».

Sur la carte de l’agent, à côté de son nom, un nombre à l’icône d’horloge compte les tâches qui tournent : commandes, surveillances, workflows, agents distants et coéquipiers, pas les sous-agents. Elle disparaît quand plus rien ne tourne.

Une commande lancée au premier plan, que l’agent attend, ne figure pas dans la liste : elle y entre au moment où Claude Code l’envoie en arrière-plan, avec l’heure de son départ.

### La vue d’une tâche

Un clic sur une tâche ouvre sa vue à la place de la conversation de l’agent. Une seule vue à la fois : celle d’un sous-agent ouverte avant cède la place. En haut, un fil d’Ariane « agent › tâche » dit où tu es ; un clic sur le nom de l’agent, ou `Échap`, te ramène à sa conversation, et le focus retourne là où tu avais cliqué. Un clic sur la carte de l’agent dans la barre latérale y ramène aussi. `Échap` ne fait rien tant qu’un champ contient du texte que tu as tapé : un brouillon ne se perd pas pour une touche. À côté du fil d’Ariane, l’état de la tâche, et « Arrêter » tant qu’elle tourne.

Dessous :

- **Sa fiche** : son type, sa durée (qui court tant qu’elle tourne), le code de sortie d’une commande finie et, quand c’est un sous-agent qui l’a lancée, « Lancée par » suivi de son nom. Un workflow ajoute sa phase en cours, ses outils et ses tokens.
- **Sa commande**, pliée quand elle est longue (« Voir toute la commande »). Une tâche sans commande montre à la place sa description.
- **Son résumé**, une fois finie : le dernier mot de Claude Code sur elle (par exemple `Background command "Ticker" completed (exit code 0)`). Pour une tâche arrêtée, ce n’est que sa description, déjà dite : Escouade ne la répète pas.
- **Sa sortie**, décrite ci-dessous.

Les titres, les commandes, les phases, les résumés et les sorties sont les mots de l’agent et de Claude Code, tels quels : Escouade ne les traduit pas. Si la tâche disparaît de la liste de l’agent (le plan est reparti de zéro), la vue se ferme.

### La sortie en direct

La sortie d’une commande ou d’une surveillance se lit en direct, en lecture seule, comme dans un terminal, mais sans ses couleurs ni ses codes d’échappement. Escouade la relit toutes les 1,5 seconde tant que la tâche tourne et que la fenêtre est visible, puis une dernière fois quand elle finit ; chaque lecture remplace ce qui est affiché. La vue suit la fin tant que tu y es ; si tu remontes, le bouton « Suivre la fin » te ramène en bas.

Ce que tu lis est la **fin** de la sortie : 8 Kio au plus tant que la tâche tourne, jusqu’à 256 Kio une fois qu’elle a fini, pour que la première erreur d’un build raté reste lisible. Quand le début manque, la vue dit « Début tronqué : seule la fin de la sortie est montrée. » La sortie d’une tâche qui tourne vient de Claude Code lui-même tant que le processus de l’agent tourne ; une tâche finie se lit d’abord dans le fichier de sortie que Claude Code garde pour elle, et la vue dit alors « lue dans le fichier de sortie ». Ce fichier sert aussi quand il n’y a plus de processus (processus arrêté, Escouade rouverte, [vue Claude Code](claude-code-view#les-taches-de-fond)). Escouade ne lit que le fichier de la tâche (`<tâche>.output`, dans un dossier `tasks`), jamais un autre, et jamais un fichier d’une autre machine : un chemin réseau (`\\serveur\partage`) est ignoré et rien n’est ouvert. Quand il n’y a ni processus ni fichier, la vue dit « Aucune sortie disponible : le fichier de sortie est introuvable. »

Un workflow, un agent distant et un coéquipier n’écrivent pas dans une sortie à eux : la vue le dit (« Pas de sortie à suivre pour ce type de tâche. »).

### Arrêter une tâche

« Arrêter », en haut à droite, arrête cette tâche seule : l’agent continue. Claude Code annonce ensuite l’arrêt à l’agent, ce qui lui coûte un tour comme les autres (tokens, coût). Le bouton est proposé pour toute tâche qui tourne. Essayé sur une commande ; une surveillance, un workflow, un agent distant ou un coéquipier s’arrêtent par la même requête, mais cela n’a pas été essayé sur un vrai flux.

Claude peut aussi lister les tâches de fond d’un agent, lire la fin de leur sortie et les arrêter par le [serveur MCP](mcp#les-sous-agents-et-les-taches-de-fond-d-un-agent), avec les mêmes bornes et les mêmes refus.

### Ce qui reste après la fin

Les dix dernières tâches finies restent dans la liste, avec leur commande et leur résumé, tant que le plan ne repart pas de zéro (une nouvelle session Claude, ou ton premier message une fois que tout est fini : tant qu’une tâche de fond tourne, le plan n’est pas fini). La liste est gardée quand tu fermes Escouade. Une tâche qui tournait à ce moment-là, ou quand le processus de l’agent s’arrête, est dite « Interrompue » : une commande lancée en arrière-plan s’arrête avec le processus. La fin d’un tour de l’agent, elle, ne finit pas une tâche de fond.

Un agent qui a une tâche de fond en cours n’est pas arrêté comme inactif (voir [les processus inactifs](managing-agents#les-processus-inactifs)), et le bouton « Relancer » est grisé tant qu’elle tourne : une commande ou un serveur de dev ne finit pas seul, alors attendre ne sert à rien. Arrête d’abord ses tâches de fond, depuis leur vue (« Arrêter »). Le redémarrage automatique d’Escouade pour une mise à jour les attend aussi (voir [les mises à jour](updates#redemarrer-automatiquement)). Dans la conversation de l’agent, la fin d’une tâche reste une ligne discrète « Tâche de fond terminée » (voir [la conversation](chat#sous-agents-taches-de-fond-et-autres-sessions)) : elle n’ouvre pas la vue de la tâche.

### Dans la vue Claude Code

Les tâches de fond sont listées aussi quand l’agent est dans [la vue Claude Code](claude-code-view#les-taches-de-fond), avec leur vue, mais sans « Arrêter » : la vue dit « Cette tâche s’arrête depuis l’interface de Claude Code. ». Escouade y lit le début d’une tâche dans le résultat de l’outil que Claude Code écrit dans son transcript, et sa fin dans la notification qu’il y rejoue : l’heure de départ n’est donc pas exacte à la seconde. La sortie se lit dans le fichier de sortie.

### Limites

- Les tâches de fond ne sont pas dans la [vue d’ensemble](overview).
- Une commande qu’un sous-agent lance au premier plan n’est pas listée ; celle qu’il lance en arrière-plan l’est, avec « Lancée par ».
- Les surveillances MCP, les agents distants et les coéquipiers sont pris en charge d’après les schémas de Claude Code ; Escouade n’en a jamais vu sur un vrai flux.

## Quand son tour est fini mais que le travail continue

Un agent peut finir son tour et laisser du travail en cours : des sous-agents ou des tâches de fond, une tâche que Claude Code relancera seul plus tard, ou d’autres agents d’Escouade qu’il a mis au travail par le [serveur MCP](mcp). Sa carte ne dit alors pas « Terminé » mais « Attend », dans la couleur d’un agent en cours. Survole ce mot : l’infobulle détaille ce qui continue, par exemple « 1 tâche de fond en cours » ou « 2 agents pilotés au travail : pr-148, pr-149 ». La carte repasse à « Terminé » quand plus rien ne tourne.

### Les réveils

Claude Code peut programmer un tour pour plus tard : une tâche planifiée de la session (`CronCreate`, ce que fait `/loop` avec un intervalle) ou le prochain réveil d’une boucle (`ScheduleWakeup`, `/loop` sans intervalle). Rien ne tourne en attendant, mais l’agent reprendra seul à l’heure dite. Quand c’est tout ce qui reste, sa carte dit « Réveil à 14:20 », et l’infobulle donne ce que Claude Code fera. Une tâche supprimée (`CronDelete`) ou une boucle arrêtée n’est plus comptée. Ces réveils vivent dans le processus de l’agent : Escouade ne l’arrête pas pour inactivité tant qu’un réveil est à venir.

### Les agents pilotés

Un agent qui en pilote d’autres par le serveur MCP (il leur écrit, en crée, lance, renvoie ou reprend leur ticket) les voit listés sous sa carte, après ses tâches de fond, dans le groupe « Agents pilotés », tant que l’un d’eux travaille ou attend une réponse. Un clic sur un nom ouvre cet agent. Le groupe disparaît quand ils ont tous fini.

Ce qu’un agent confie hors d’Escouade (un message à une autre session de Claude Code, une routine dans le cloud) n’est pas suivi : seule la ligne de l’outil, dans sa conversation, le dit.

## Quand le plan disparaît

Quand toutes les tâches sont faites et que plus rien ne tourne (ni sous-agent, ni tâche de fond), le bandeau reste affiché (« 7/7 tâches ») jusqu’à ton prochain message : ce message commence un nouveau plan, et le bandeau repart de zéro avec ce que l’agent liste ensuite. Le bandeau marche aussi quand l’agent tourne dans [la vue Claude Code](claude-code-view).
