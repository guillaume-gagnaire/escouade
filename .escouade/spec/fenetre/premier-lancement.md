---
title: "Premier lancement"
subject: "fenetre"
sources: ["docs/ONBOARDING.md","design/onboarding-2-3/README.md","design/onboarding-2-3/maquette.html","design/onboarding-2-3/captures/c-harness.png","design/onboarding-2-3/captures/c-modes-clair-1024.png","design/onboarding-2-3/captures/c-projet.png","design/onboarding-2-3/captures/c-agent.png","design/onboarding-2-3/captures/c-agent-clair-1024.png","src-tauri/src/onboarding.rs","src-tauri/src/telemetry.rs","website/content/privacy/fr.md"]
tickets: ["ESC-503","ESC-504","ESC-505","ESC-506","ESC-512","ESC-507"]
decisions: [195,215,223]
compared: null
---

## Ce que c'est

Le parcours qui règle Escouade sur un poste neuf, une question à la fois : le harness, son compte, le mode des nouveaux agents, ce qu'on veut brancher (sources de tickets, voix, hébergeur de code), le premier projet, le premier agent et l'accord des statistiques. Il prend toute la fenêtre, sauf la barre de titre et la barre de statut, et dessine ses stations comme les commits du git graph (direction C « La ligne » de `docs/ONBOARDING.md`). Il ne règle rien à sa façon : chaque station écrit ce qu'écrivent les réglages, par les mêmes appels et les mêmes composants, et chacune se passe. Il se prolonge ensuite dans la découverte des fonctionnalités, page « À découvrir ».

## Comportement

### Quand il s'ouvre

- **De lui-même**, au premier démarrage d'une **nouvelle installation** seulement : des réglages sans mode pour les nouveaux agents (SEC-09 : un `settings.json` d'avant reçoit `auto` à son chargement) et aucun projet. Jamais sur un poste déjà réglé, même sans compte, même après une mise à jour.
- **Repris** : fermé au milieu (la fenêtre, l'app, une coupure), il rouvre au démarrage suivant, à la première station ni faite ni passée. Fini ou « Tout passer », il ne revient plus de lui-même.
- **Refait** : Aide › Premier lancement (l'icône Aide du rail) et Réglages › Général › « Refaire le premier lancement » le rouvrent sur n'importe quel poste. Toutes les stations s'y montrent depuis la première, avec la valeur en place sur la ligne (« Claude Code », « Principal », « Demander », « 2 branchés »). Rien n'est remis à zéro : refaire ne retire ni compte ni projet. Il se ferme par « Fermer ».
- Un client de l'API distante ne le voit jamais.

### L'état gardé

- Par installation, dans `state.json` (l'état durable, ESC-469), jamais dans `.escouade.yml` : la version du parcours, quand il a commencé et fini, s'il est refait, ce que l'utilisateur a fait à chaque station (faite, passée) et à chaque carte de « Ce que tu veux brancher » (`hookups` : branchée, passée).
- **Le parcours lit l'état réel** : une station est faite quand ce qu'elle règle l'est, où que ce soit (un compte ajouté dans Réglages coche celle du compte). Une station faite le reste, même si l'état change ensuite : un compte retiré ne rouvre rien.
  - harness et compte : un compte ajouté ;
  - réglages de départ : le mode des nouveaux agents choisi ;
  - ce que tu veux brancher : quelque chose de branché, où que ce soit (un compte d'une source de tickets, d'un hébergeur de code, ou la dictée en ligne avec la clé de son service) ;
  - premier projet : un projet ouvert ;
  - premier agent : un agent qui a fini un tour sans erreur (un message envoyé, l'agent au repos, ni arrêté par la limite d'usage ni en échec) ;
  - statistiques : l'accord donné ou refusé.
- Le cœur relit la vue à chaque changement de ces réglages, des projets, de l'état des agents ou des réglages imposés, et à chaque geste sur une carte de ce qu'on branche. La vue dit aussi où vit le projet de démonstration (`demo`), qu'il existe ou non.
- Le parcours et la découverte ne se cochent pas l'un l'autre : le premier projet, la démonstration et le premier agent ne font aucune découverte (un seul agent par projet n'est pas « Un 2ᵉ agent », une commande que lance l'agent n'est pas « Lancer le projet »), et une découverte faite ne fait aucune station.

### La fenêtre

- **À gauche, la ligne** (une `nav`, « Les stations du premier lancement ») : « Premier lancement », « 7 stations · environ 2 minutes », puis une station par question. Faite : un nœud plein et sa valeur (le nom du projet, celui du premier agent) ; en cours : teal, « maintenant » (`aria-current="step"`) ; passée : creuse, « passé · à reprendre » ; imposée : pleine, « imposé » ; à faire : en pointillés. Chaque station est un bouton qui y mène. Sous la ligne, les voies de la découverte (« Ensuite : À découvrir », Planifier · Coder · Relire · Livrer · Piloter, décoratives) et, en bas, « Tout se passe et se reprend. Le parcours revient depuis Aide › Premier lancement. » avec « Tout passer » (« Fermer » quand il est refait).
- **À droite, la station** : « Station 2 · le compte », sa question (le `h1`, qui prend le focus à chaque station) et son pourquoi (sa description), ses choix (un groupe radio, le choix recommandé déjà pris, « Recommandé » en pastille ; des cases à cocher, aucune cochée, à la station de ce qu'on branche).
- **Le pied** : « Retour », « ↑ ↓ choisir · Entrée valider » (caché en 1024), « Plus tard » et le bouton principal (« Continuer », « Garder Édits auto », « Ouvrir le projet »…).
- **Au clavier** : ↑ ↓ vont d'un choix à l'autre en sautant ceux qui ne se prennent pas (depuis la question, ↑ ↓ amènent aux choix), Espace prend le choix (ou coche la carte) qui a le focus, Entrée fait le bouton principal, Échap fait « Plus tard » (dans un champ, Échap quitte seulement le champ). Rien de cela tant qu'une fenêtre est ouverte par-dessus (« Ajouter un compte… », l'assistant d'un projet, le diff), ni dans la conversation encadrée du premier agent, qui garde ses touches (Échap y arrête l'agent, ses raccourcis répondent à sa carte).
- **Aucune bulle de la découverte** ne vient sur le parcours tant qu'il est à l'écran, pas même celle du micro survolé dans le compositeur du premier agent : une bulle qui attend son moment attend comme pendant la frappe (six secondes au plus), puis passe.
- **Tailles** : 1440 × 900 et 1024 × 728, sans défilement en travers ; en 1024, la ligne passe à 248 px et la station se resserre. Une station trop haute défile en elle-même.
- **Animation** : l'arrivée d'une station monte de 8 px en 240 ms ; avec « réduire les animations », un fondu seulement.
- Thèmes clair et sombre ; les voies du thème clair sont foncées pour tenir 3:1 sur le cadre et le panneau (choix O14), le git graph en profite.

### Station 1, le harness

- « Avec quel harness tes agents travaillent-ils ? » — « Un harness est le programme d'agent de code de ta machine. Escouade le lance et le pilote ; il garde ta connexion et tes réglages. »
- Chaque harness qu'Escouade connaît, avec sa version et son état : « Connecté · ada@atlas.dev » (son compte de la machine connecté, ou un compte déjà ajouté), « Installé, pas connecté » (« Tu t'y connecteras dans un terminal, juste après. »), « Trop ancien », « Version non prise en charge », « Introuvable ». Les prêts d'abord, puis les installés, puis les autres, dans « Ordre des harness ».
- **Recommandé** : le premier connecté, sinon le premier trouvé. Un harness trop ancien ne se prend pas.
- **Introuvable** : choisi, il montre sa commande d'installation (celle du manifeste de son plugin, Windows ou Unix) et « Copier » (« Commande copiée »), avec la ligne du plugin tiers qui la donne (SEC-30). Aucun trouvé : « Aucun harness n'est installé. Copie la commande de celui que tu veux, lance-la dans un terminal : Escouade le trouvera dès que tu reviens. » « Continuer » attend un harness trouvé ; « Chercher à nouveau » relit le PATH (« Cherché dans le PATH », et WSL), et la fenêtre revenue au premier plan aussi.
- **Ce que « Continuer » écrit**, comme Réglages › Comptes et harness : le harness en tête de « Ordre des harness » (« Le harness choisi passe en tête de l'ordre des harness : c'est celui des nouveaux projets. ») ; s'il est connecté sur la machine et sans compte, son compte de la machine, ajouté comme « Ajouter un compte… » l'ajoute (la station du compte se fond alors dans celle-ci : « Ses agents partent sur le compte déjà connecté ; un compte à part se crée dans Réglages › Comptes et harness. ») ; sinon son plugin seul.
- **Les plugins** : seul le plugin du harness choisi s'installe, depuis l'amorce, sans réseau (le tri de l'amorce), et la station le dit avant : « Escouade installe le plugin de Codex, depuis l'app, sans réseau. »

### Station 2, le compte

- « Quel compte pour Claude Code ? » — les quotas, les statistiques et la connexion sont tenus par compte ; d'autres s'ajoutent ensuite.
- « Le compte de la machine » (avec son e-mail quand il est connecté ; « Son dossier habituel, tel quel ») ou « Un compte à part » (« Un dossier à lui sous ~/.escouade, et sa propre connexion »). Le compte de la machine ne se prend pas quand il est hors de portée d'Escouade (Gemini CLI) ou déjà ajouté.
- « Le compte de la machine » l'ajoute ; pas encore connecté, sa connexion suit dans la fenêtre « Se connecter… » (son programme dans un terminal). « Un compte à part » ouvre « Ajouter un compte… » ; fermée, la fenêtre revient au parcours, et le compte fait coche la station.
- Un compte déjà là : « Compte ajouté : Principal », et la station est faite.

### Station 3, les réglages de départ

- « Quand tes agents doivent-ils te demander ? » — « Le mode des nouveaux agents ; tu le changes pour chacun, sous son message. Modèle Sonnet, effort moyen : ceux de ton compte. » Le modèle et l'effort sont dits, pas demandés : ceux du compte (Claude Code : Sonnet, moyen). Un harness sans modèle à lui : « Effort moyen : celui de ton compte, avec le modèle par défaut de ton harness. »
- Les quatre modes de SEC-09, chacun en une phrase (les textes de la question d'avant) : Édits auto (recommandé), Demander, Plan, Auto. Un mode que le harness n'a pas est grisé (« Codex n'a pas de mode Auto. »), un mode qu'une stratégie interdit aussi, avec sa raison.
- **Bypass** est montré, éteint, jamais proposé : « Éteint : l'agent ferait tout sans rien te montrer, même ce qu'un contenu piégé lui dicterait. À réserver à une machine jetable ; il s'allume dans Réglages › Agents. » Déjà permis : « Permis dans Réglages › Agents : jamais proposé comme mode de départ, il se choisit agent par agent. »
- Un harness qui ne tourne qu'en Bypass le dit (ESC-489) : « Mistral Vibe ne tourne qu'en Bypass : ses agents agiront sans demander. »
- « Garder Édits auto » écrit « Mode par défaut des agents » (`agentMode`), comme Réglages › Agents.

### Station 4, ce que tu veux brancher (ESC-512)

- « Que veux-tu brancher ? » — « Rien n'est coché : coche ce qui te sert. Rien ne s'installe, ne se connecte ni ne se télécharge sans ton geste ; ce que tu passes t'attend dans À découvrir et dans les Réglages. »
- Après les réglages de départ et avant le premier projet : un hébergeur branché ici donne à la station du projet ses dépôts à cloner (ON-2).
- **Trois cartes à cocher** (un groupe « Les branchements », chaque carte une case `checkbox` décrite par sa phrase), **aucune cochée d'avance**, chacune avec ce qu'elle sert et ce qu'elle demande, et son propre « Plus tard » une fois cochée (« Plus tard : Une source de tickets ») :
  - **« Une source de tickets »** — « Les tickets de Jira, Trello, GitHub Issues, Linear, ClickUp, GitLab Issues et Azure Boards arrivent dans le Kanban : un agent les prend et les fait avancer. Il faut un jeton de la source, puis un projet ou un tableau à relier. »
  - **« La voix »** — « Veux-tu dicter ou parler à tes agents ? Whisper sur ta machine, rien n'en sort ; ou un service en ligne, avec ta clé. »
  - **« Un hébergeur de code »** — « Les PR et la CI des tickets dans Escouade, sur GitHub, GitLab, Bitbucket, Gitea et Azure DevOps. Il faut un jeton de l'hébergeur, gardé dans le trousseau du système. »
- **La liste est celle du cœur**, jamais écrite dans la fenêtre (`onboarding_offers`) : les sources du registre, les services de voix et les hébergeurs des tables du tri de l'amorce, chacun avec son plugin, s'il est installé, si l'amorce le tient, et ce qu'une stratégie en refuse. Les autres plugins du store de la même catégorie, d'après son catalogue déjà lu (rien n'est demandé au réseau), sont nommés : « Aussi dans le store : … Ils s'installent depuis Plugins. »
- **Cocher une carte** ouvre ses choix ; rien ne s'installe encore. **Choisir une chose** (Trello, Mistral, GitLab) est le geste : son plugin, et lui seul, s'installe depuis l'amorce, sans réseau (`onboarding_need`, le tri de l'amorce ; « Plugin Trello installé. »). Un plugin que l'amorce ne tient pas vient du store : « Son plugin vient du store : installe-le depuis Plugins, ses permissions sous les yeux. » ; la station ne l'installe pas.
- **Une source** : chacune dit ce qu'elle importe et ce qu'elle demande (« Les cartes d'un tableau Trello · une clé et un jeton d'API » ; une source que la fenêtre ne connaît pas : « Ses tickets dans le Kanban · un jeton de la source »). Choisie, son formulaire de connexion s'ouvre **dans la station**, le bloc même de Réglages › Services connectés (`SourceAccount`) : mêmes champs, même vérification, ses secrets gardés là où ils le sont toujours (le trousseau, ou le fichier dit sans trousseau, ESC-499). GitLab Issues et Azure Boards se connectent par le formulaire de leur hébergeur (`ForgesSection`). « Le projet ou le tableau se relie ensuite à chaque projet, dans ses Réglages › Tickets importés. »
- **La voix** : « Qui transcrit ce que tu dis ? »
  - « Whisper, sur ta machine » — « Le modèle « base » : 141 Mo à télécharger une fois, gardés sur ton disque. Rien de ce que tu dis ne quitte ta machine. » Le poids est dit avant ; « Télécharger (141 Mo) » le télécharge, suivi d'une jauge (« Téléchargement : 52 Mo sur 141 Mo »), annulable (« Annuler le téléchargement » : rien n'est gardé). Prêt : « Modèle « base » prêt ».
  - « Un service en ligne » — « Plus rapide sur une petite machine. Chaque dictée part chez le service choisi, avec ta clé. » Les services sont ceux du cœur (OpenAI, Groq, Mistral, ElevenLabs, Deepgram). Choisi : son plugin, puis la dictée en ligne sur lui (`voice.transcription`, `voice.onlineService`, ce qu'écrit Réglages › Voix), ce qui part dit (« Ce qui part chez Mistral (api.mistral.ai) : l'audio de tes dictées, rien d'autre. Ta clé reste dans le trousseau du système. ») et la ligne de sa clé, celle de Réglages › Voix.
  - Prête, la voix s'essaie : « Essaie le micro » — « Clique sur le micro, dis une phrase, puis reclique : elle s'écrit ici. Rien n'est envoyé à un agent. » (le bouton « Dicter » des champs de l'app, dans un champ de la station). « La voix qui lit les réponses dans une conversation se règle dans Réglages › Voix. »
- **Un hébergeur** : choisi, son plugin, puis le formulaire « Connecter… » de Réglages › Services connectés, ouvert sur son type.
- **« Continuer »** : chaque carte branchée est faite (une carte se coche aussi par l'état réel : un compte d'une source ou d'un hébergeur, la dictée en ligne avec sa clé ; la voix sur la machine, par son modèle prêt) ; chaque autre carte, cochée ou non, est **passée**. « Plus tard » au pied passe la station et toutes ses cartes ; « Tout passer » aussi. Sous la station, la ligne dit « 2 branchés » ou « rien de branché ».
- **Une carte passée** se retrouve dans la découverte (Aide › À découvrir, ON-3 : la source de tickets, la dictée, l'hébergeur) et dans sa page de Réglages (Services connectés, Voix ; installer son plugin se fait dans Plugins, les réglages ne listant toujours que ce qu'apportent les plugins installés). Le parcours refait la montre « Passé · à reprendre ».
- **Hors ligne** (le système le dit) : ce qui demande le réseau le dit et attend (« Hors ligne : ceci demande le réseau. La carte t'attend dans À découvrir, tu la reprendras une fois en ligne. ») ; « Télécharger » ne se prend pas ; le plugin de l'amorce s'installe tout de même.
- Le second harness et le serveur MCP pour un autre client n'y sont pas : le premier est la station du harness et Réglages › Comptes et harness ; le second ouvre une porte de la machine, que le premier lancement n'ouvre pas. Les deux restent à la découverte.

### Station 5, le premier projet

- « Où travaille ton premier agent ? » — « Un projet est un dossier, avec ses agents, son Kanban et ses docs. Chaque agent y travaille dans son worktree, sur sa branche : deux agents ne se gênent pas, et rien n'entre dans ta branche sans toi. »
- **Les mots d'Escouade**, à côté d'un dessin de branches (ta branche, « agent 1 · son worktree », « agent 2 », décoratif) : « Un projet : un dossier, avec ses agents, son Kanban et ses docs. » ; « Un agent : une conversation avec ton harness, qui lit et modifie le code. » ; « Un worktree : une copie de travail à l'agent seul : deux agents ne se gênent pas. » ; « Une branche par agent : son travail reste à part jusqu'à ce que tu l'intègres. »
- **Trois réponses** (quatre avec une équipe), une seule à la fois, rien d'ouvert, de cloné ni d'écrit avant le bouton principal :
  - **« Un dossier de ta machine »** (recommandé, choix O4) — « Un dépôt que tu as déjà. Escouade le lit sans rien y changer. » Un champ et « Parcourir… ». Le dossier lu dit avant d'ouvrir : « Ce dossier n'existe pas. » ; « Escouade ne peut pas écrire dans ce dossier (C:\lecture-seule) : choisis-en un autre. » (le bouton reste éteint) ; sans git, « Ce dossier n'est pas un dépôt git : Escouade peut en faire un, avec un premier commit. » et « Faire un premier commit » coché ; avec un `.escouade.yml`, « Ce dépôt a un .escouade.yml : il règle board.merge_strategy, launch.commands. Ses commandes se relisent avant de tourner. » (la relecture des commandes qui existe, dans le projet). « Ouvrir le projet » l'ajoute comme l'assistant le fait sans ses étapes (un worktree par agent, ses autres réglages par défaut), puis le premier commit s'il est coché.
  - **« Cloner un dépôt »** — « Une adresse git, ou un dépôt de GitHub ou GitLab si tu en as connecté un. » L'adresse (les dépôts des hébergeurs connectés y sont proposés), « Dans le dossier » (celui des projets du poste, sinon le dossier personnel) et « Nom du dossier » (le nom du dépôt). « Cloner et ouvrir » clone par les accès git habituels de l'utilisateur (jamais un jeton d'Escouade), dans un dossier neuf, puis l'ouvre. L'adresse est vérifiée avant git : https ou ssh, pas de dossier local, pas de mot de passe ni de jeton dedans. Hors ligne (le système le dit, ou git n'a pas joint l'hôte) : « Cloner demande le réseau. Choisis un dossier ou la démonstration, ou réessaie une fois en ligne. » Un échec : « Le clonage a échoué : remote: Repository not found. Vérifie l'adresse et ton accès, puis réessaie. », et le bouton devient « Réessayer ». Un dossier qui existe et n'est pas vide est refusé et laissé tel quel ; un clone coupé ne laisse rien.
  - **« Un projet de démonstration »** — « Un petit dépôt jetable, créé sur ta machine sans réseau, pour essayer sans risque. Supprimé d'un clic. » Il dit où : « Il s'écrit dans ~/.escouade/demo/bac-a-sable : une liste de tâches en TypeScript, deux tests qui échouent exprès. Rien ne sort de ta machine. » « Créer la démonstration » l'écrit et l'ouvre (le projet `bac-a-sable`) ; demandé de nouveau, c'est le même.
  - **« Les projets de ton équipe »** (un poste qui a rejoint une équipe, P4 ; alors en tête et choisi) — « Acme partage ses projets : clone celui sur lequel tu travailles. » « Voir les projets d'Acme » ouvre « Projets de l'équipe », dont « Cloner » mène à l'assistant ; le projet fait coche la station.
- Un projet ouvert : « Projet ouvert : demo-api », et la station est faite. Refait, les réponses restent offertes sous cette ligne.

### Le projet de démonstration

- Écrit par l'app dans `~/.escouade/demo/bac-a-sable` (le dossier de données) et nulle part ailleurs : `README.md`, `package.json` (`npm test` = `node --test test/todo.test.ts`, aucune dépendance), `src/todo.ts`, `test/todo.test.ts` (quatre tests, deux échouent exprès), `.gitignore`. Un dépôt git à lui, un commit sur `main` signé « Escouade », sans remote : rien ne peut en sortir. Les worktrees de ses agents sont dedans (`.claude/worktrees`).
- Un dossier qui est déjà là sans être la démonstration n'est jamais touché : « … existe déjà et n'est pas le projet de démonstration : déplace-le, puis réessaie. »
- **Sa bande**, en haut du projet (hors du parcours) : « **Bac à sable** · un dépôt de démonstration sur ta machine (~/.escouade/demo/bac-a-sable) · rien ne sort d'ici » et « Supprimer ». « Supprimer » demande « Supprimer le projet de démonstration ? » (« Le projet « bac-a-sable », ses agents et leurs conversations, et son dossier … s'effacent. Rien d'autre n'est touché. »), puis ferme le projet en effaçant son historique et retire son dossier entier ; « Projet de démonstration supprimé. ». Le cœur refuse de supprimer ainsi tout autre projet ou dossier. Il n'est jamais retiré de lui-même.

### Station 6, le premier agent

- **Créé par le parcours** dans le projet de la station 5, avec ce qu'ont réglé les stations d'avant : son harness, le modèle et l'effort de son compte, le mode des nouveaux agents. Le projet avait déjà un agent : c'est lui. Sans projet : « Ton premier agent a besoin d'un projet : choisis-en un à la station précédente. » (« Revenir au premier projet ») ; sans compte : « Ton premier agent a besoin d'un compte connecté : ajoute-le à la station du compte. » (« Revenir à l'étape du compte ») ; aucun agent n'est créé alors.
- **À droite, sa vraie conversation, encadrée** (« La conversation de ton premier agent ») : son en-tête, ses messages, sa carte de permission et son compositeur, comme partout dans l'app. En 1024, elle passe sous l'explication.
- **Le premier message** : « Ton premier message » — « Choisis-en un, modifie-le, puis envoie-le. Il part sur ton compte Claude Code et compte dans son quota. » Les messages proposés (un groupe radio) : « Explique-moi ce dépôt en cinq lignes. », « Lance les tests et dis-moi ce qui casse. » (seulement si le projet a une commande de test qu'Escouade reconnaît et qui peut tourner ici : `Cargo.toml`, `pyproject.toml`, `setup.py`, `go.mod`, `pom.xml`, `build.gradle`, ou un `package.json` avec un script `test` quand Node est sur la machine), « Trouve un petit bug et propose un correctif. ». Le premier est pris ; sur la démonstration, c'est « Lance les tests ». Le message pris est mis dans le compositeur, modifiable ; un autre pris le remplace. « tests-qui-cassent est prêt. Un message proposé t'attend : modifie-le ou choisis-en un autre. Rien ne part avant que tu l'envoies. » **Rien ne part sans le clic** de l'utilisateur sur le bouton d'envoi du compositeur (ou Entrée dans le champ).
- **Envoyé, en préparation** : le premier message attend que le worktree de l'agent soit fait (au premier message, sous son nom) ; la station le dit aussitôt : « Ton agent prépare son worktree » — « Ton message est parti : il attend que la copie de travail de l'agent soit prête, sur sa branche à lui. Sa réponse suit, à droite. », et la conversation le montre « En attente de la préparation… ». Jamais « Ton premier message » une fois envoyé.
- **Pendant le tour** : « Ton agent travaille » — « Sa réponse arrive en direct, à droite. Il travaille dans son worktree, sur sa branche : ton dépôt ne bouge pas. » ; « Pour l'arrêter : Échap, ou ■ sous le message. » ; « Ton message est parti sur ton compte Claude Code et compte dans son quota. »
- **La première permission**, expliquée à côté de la vraie carte : « Ton agent te demande la permission » — « Il veut lancer une commande. En Édits auto, chaque commande te demande : **Autoriser** cette fois, **Toujours** pour cette commande dans ce projet, ou **Refuser** pour qu'il cherche autrement. » ; « Au clavier : Ctrl+Entrée autorise, Ctrl+Maj+Entrée autorise toujours. Ctrl+J t'amène au prochain agent qui attend. » (les raccourcis de la carte, écrits comme le système les écrit : ⌘ sur macOS) ; « Plus tard, une demande comme celle-ci fait clignoter la carte de l'agent. » Une question de l'agent : « Ton agent te pose une question » — « Réponds sur sa carte, à droite : il reprend dès que tu as répondu. »
- **Fini** : « Ton agent a répondu » — « L'agent a fini, dans son worktree, sur sa branche : ton dépôt n'a pas bougé. » ; « Voir le diff » (« 1 fichier changé ») ouvre ses changements (« Ce que tests-qui-cassent a changé ») ; rien de changé : « Il n'a rien changé : il n'y a pas de diff à voir. » ; « Tu le retrouves dans la barre de ton projet, sous son nom « tests-qui-cassent » ; d'autres messages s'écrivent sous sa conversation. » « Continuer » n'est allumé qu'une fois le tour fini ; « Plus tard » l'est toujours.
- **Les tours qui s'arrêtent court**, chacun dit quoi faire et laisse reprendre :
  - la limite d'usage : « Ce compte est à sa limite » — « Ce compte a atteint sa limite d'usage jusqu'à 18:05. Ajoute un autre compte, ou reviens après. » et « Ajouter un compte… » (la fenêtre revient au parcours) ;
  - hors ligne (le harness n'a pas joint son service) : « Hors ligne » — « Claude Code n'a pas pu joindre son service : tu sembles hors ligne. Le message reste prêt. » et « Remettre le message » (le dernier message envoyé, de nouveau dans le compositeur) ;
  - pas connecté (clé refusée, connexion perdue) : « Claude Code n'est pas connecté » — « Claude Code n'est pas connecté : reviens à l'étape du compte. » et « Revenir à l'étape du compte » ;
  - tout autre échec : « Le tour a échoué : {ce qu'a dit le harness}. Réessaie, ou passe à la suite avec « Plus tard ». » et « Remettre le message ».

### Station 7, les statistiques

- Posée au premier lancement quoi que l'utilisateur ait passé avant (le premier projet, le premier agent), avant que rien ne parte : la réponse reste vide jusque-là (SEC-09).
- La question de toujours (« Aider à améliorer Escouade ? », ses textes, son lien « Ce qui est envoyé, en détail »), le même réglage, plus : « Tes projets, ton code, tes conversations et tes comptes restent sur ta machine, dans ~/.escouade. » « Non merci » et « Partager les statistiques » ont le même poids. Répondre finit le parcours.
- Passées, elles sont redemandées **une fois** au démarrage suivant, dans la fenêtre d'avant, puis plus jamais. Une installation qui n'a jamais vu le parcours garde la règle d'avant : redemandées à chaque démarrage tant qu'elles n'ont pas de réponse.

### La fin, et la suite : « À découvrir »

- « Escouade est prêt » — « Le reste se découvre en le faisant : la voix, le Kanban, les PR… La carte t'attend dans Aide › À découvrir. », et « 2 étapes passées : elles t'attendent dans Aide › Premier lancement. ».
- « Commencer » (le bouton principal) ferme le parcours, sur le premier projet s'il y en a un. « Voir ce qu'il y a à découvrir » le ferme aussi et ouvre la carte « À découvrir » (page « À découvrir ») ; « Retour au projet » y ramène au premier projet. La carte ne s'ouvre jamais d'elle-même, et ses bulles au bon moment ne viennent qu'une fois le parcours fermé.

### Ce qui a changé autour

- Le parcours remplace l'enchaînement du premier démarrage : la fenêtre du mode (`FirstModeModal`, retirée), puis celle des statistiques, puis celle du nom pour l'équipe. Le nom n'est plus demandé au premier lancement : il l'est au premier partage (choix O8).
- L'écran sans projet (`Welcome`) reste celui d'après le parcours, et celui d'un poste réglé.
- Le bloc d'une source de tickets de Réglages › Services connectés est un composant (`SourceAccount`), que la station de ce qu'on branche montre aussi.

### Réglages imposés

- Une station qu'une stratégie impose est faite et verrouillée, avec qui l'impose (« Imposé par la stratégie de ce poste », « Imposé par l'équipe Acme ») : le mode des nouveaux agents (`permissions.defaultMode`), les statistiques (`telemetry.enabled` : « Éteintes par la stratégie de ce poste. »). Elle se traverse en un « Continuer ».
- Un harness que `harnesses.allowed` ne permet pas est grisé, « non permis par la stratégie de ce poste », jamais recommandé ; un seul permis : « Codex est le seul harness permis sur ce poste. » Un mode que `permissions.modes` refuse est grisé avec sa raison ; « Autoriser le mode Bypass » imposé est dit sous Bypass.
- **Ce que tu veux brancher** (choix O12, montrer verrouillé avec qui l'impose) : la dictée coupée (`voice.dictation`) verrouille la carte de la voix (« Verrouillé · Imposé par la stratégie de ce poste ») ; la voix en ligne interdite (`voice.onlineAllowed`) grise « Un service en ligne », Whisper sur la machine restant ; un plugin que `plugins.denied`, `plugins.allowed` ou `plugins.allowedOrigins` refuse grise sa source ou son hébergeur, avec la raison du cœur ; une carte dont tout est refusé est verrouillée. Le cœur refuse d'installer ce qu'une stratégie refuse (`onboarding_need`), quoi que montre la fenêtre.
- Le parcours ne contourne jamais un refus du cœur : il montre ce que le cœur refuserait.

### L'aide

- L'icône « Aide » du rail, au-dessus des stats (et au premier démarrage, entre `+` et « Réglages ») ouvre un menu : « Premier lancement » (le parcours refait), « À découvrir » (la carte de la découverte des fonctionnalités, page « À découvrir »), « Documentation » (le site, dans la langue de l'interface), « Signaler un problème » (les tickets du dépôt sur GitHub). Un point teal s'y pose tant qu'il reste des découvertes ni faites ni écartées.

## Réglages

- Aucun réglage à lui : il écrit `engineOrder`, les comptes, `agentMode`, `voice.transcription` et `voice.onlineService`, les comptes des sources et des hébergeurs, la clé d'un service de voix, les projets et `telemetry`, comme leurs pages.
- Réglages › Général › Toi › « Premier lancement » › « Refaire le premier lancement ».

## Commandes

- `onboarding_view`, `onboarding_act` : la vue du parcours et ses gestes (la fenêtre seule le montre).
- `onboarding_offers` : ce qu'offrent les cartes de ce qu'on branche ; `onboarding_need` : le plugin de ce qui y est choisi, depuis l'amorce, et lui seul.
- `onboarding_folder` : un dossier lu pour la station du projet (là, écrit, une commande de test qui peut tourner) ; `onboarding_clone` : le clone par une adresse ; `demo_create`, `demo_remove` : le projet de démonstration. Servies à distance comme les autres commandes de chemins (`docs/REMOTE.md`).

## Limites

- La bande « Nouveau dans la 2.3 » d'un poste mis à jour et la découverte (Aide › À découvrir) ont leur page, « À découvrir » ; une nouvelle installation ne voit jamais la bande. Les événements de statistiques du parcours sont d'ON-4 (ESC-507) ; la station de ce qu'on branche n'en ajoute aucun (un plugin installé compte déjà comme `plugin_installed`).
- Une invitation d'équipe ouverte au premier lancement n'est pas encore une station : la station du projet propose les projets d'une équipe déjà rejointe.
- Le clone ne propose que les dépôts des hébergeurs connectés qui servent aussi une source de tickets (GitHub, GitLab) ; un dépôt d'un autre se clone par son adresse.
- « Lance les tests » sur la démonstration demande Node 22.18 ou plus récent pour tourner ; la station ne vérifie que la présence de Node.
- Le parcours dit les raccourcis réels de la carte de permission (Ctrl+Entrée, Ctrl+Maj+Entrée), pas « 1, 2 ou 3 » des maquettes : l'app n'en a pas d'autres.
- Le modèle de Whisper proposé à la station de ce qu'on branche est celui des réglages (« base » par défaut) ; les autres se choisissent dans Réglages › Voix.
- Un poste qui a fermé la question du mode sans répondre avant la 2.3 (son mode vide, des projets) n'est pas un poste neuf : le parcours ne s'ouvre pas, le mode recommandé reste en vigueur.

## La mesure de l'entonnoir (ESC-507)

- **Avec l'accord seulement**, pseudonymes comme les autres statistiques (page « Statistiques d'usage » de la confidentialité du site) : jamais un nom de projet, un chemin, une adresse, un e-mail, un message ni ce qui a été choisi (ni harness, ni compte, ni dossier). Les valeurs sont des listes fermées et des tranches.
- **Quatre événements du parcours**, dits une fois qu'un passage du parcours se termine (sa dernière station ou « Tout passer », « Fermer » quand il est refait) :
  - `onboarding_started` : `trigger` (`first` une nouvelle installation, reprise comprise ; `redo` refait depuis Aide ou Réglages) et `profile`, ce qu'Escouade a trouvé à la station du harness (`managed` une stratégie impose le harness ou le mode ; sinon `ready` un harness connecté, `installed` installé seulement, `none` aucun) ;
  - `onboarding_step`, un par station quittée : `step` (`harness`, `account`, `defaults`, `hookups`, `project`, `agent`, `telemetry`), `outcome` (`done`, `skipped`, `policy`) et, là où un choix est recommandé et la station faite par lui, `choice` (`recommended` gardé, `other`) : le harness, le compte de la machine quand il est connecté, « Édits auto », « Un dossier de ta machine » (« Les projets de ton équipe » sur un poste qui a rejoint une équipe) ;
  - `onboarding_finished` : `skipped` (le nombre de stations passées) et `seconds` (sa durée, `<60`, `60-120`, `120-300`, `>300`) ;
  - `first_reply`, quand le premier agent a fini son premier tour pendant ce passage : `seconds` (depuis son début, mêmes tranches) et `demo` (dans le projet de démonstration ou non).
- **Rien ne part avant la réponse** : la question des statistiques est la dernière station. Ce qu'en dit le parcours est lu dans son état gardé (`state.json` : le début du passage, le profil, les choix gardés ou non, l'heure de la première réponse, et s'il a été dit), jamais dans une file d'événements : envoyé au oui, oublié au non (`told`), gardé tant qu'il n'y a pas de réponse ; un oui donné plus tard (la question redemandée au démarrage suivant, ou Réglages › Général) l'envoie alors, une fois. Un passage refait a son propre entonnoir ; la première réponse d'avant n'y compte pas.
- **Le taux d'arrivée au premier agent** : `first_reply` ÷ `onboarding_started{trigger=first}`, par semaine (le même `installId`). L'entonnoir : `onboarding_step` par `step` et `outcome`. Le temps se lit dans les tranches de `seconds` ; `profile` dit combien arrivent sans harness prêt.
- La station de ce qu'on branche n'a pas d'événement à elle : elle est une station de `onboarding_step`, et un plugin installé compte déjà comme `plugin_installed`.
- Un premier tour fini après la fin du parcours (le premier agent passé) n'est pas un `first_reply`.

## Les mots relus (ESC-507)

- Le parcours dit « station » partout (« stop » en anglais), jamais « étape » : « Revenir à la station du compte » et « … n'est pas connecté : reviens à la station du compte. » (au lieu de « l'étape du compte » plus haut), « 2 stations passées : elles t'attendent dans Aide › Premier lancement. » (au lieu de « 2 étapes passées »).
- La page « Premiers pas » de la documentation du site (`first-steps`) décrit le parcours, avec des captures de la vraie app (clair et sombre) prises par le script des captures du site (`tests/site-shots/first-launch.spec.ts`) ; elle est reliée depuis l'installation et la présentation.

- Station 5 : son pourquoi ne redit plus ce qu'est un projet (les mots d'Escouade, juste dessous, le disent) : « Dans un projet, chaque agent travaille dans son worktree, sur sa branche : deux agents ne se gênent pas, et rien n'entre dans ta branche sans toi. » Sous la station dans la ligne, à faire : « dossier, clone ou démo ».
- Station 6, fini : « Pour l'arrêter : Échap, ou ■ sous le message. » n'est dit que pendant le tour, plus une fois qu'il est fini.
- En 1024 × 728, la ligne ne montre plus les voies à venir (le dessin et « Ensuite : À découvrir », décoratifs, que la fin du parcours dit) : « Tout passer » reste à l'écran sans faire défiler la ligne.
