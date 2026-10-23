# Plusieurs comptes

Escouade peut lancer tes agents sur plusieurs comptes de chaque harness (personnel, travail, client) et choisit seule celui de chaque nouvel agent. Cette page explique comment les ajouter, les ordonner, les réserver à des projets et reprendre un agent arrêté par la limite d’usage sur un autre compte.

## À quoi servent plusieurs comptes

Un compte Claude, c’est une connexion à claude.ai avec ses quotas (une fenêtre de 5 heures et une fenêtre hebdomadaire) et ses conversations. Avec plusieurs comptes, tu sépares le perso du pro, ou tu gardes un compte à part pour les projets d’un client. Quand un compte atteint le seuil de pause, les nouveaux agents partent sur un autre ; avec « Changer de compte à », les agents déjà lancés aussi, à la fin de leur tour.

Chaque compte est Claude Code lancé avec son propre dossier de configuration (la variable `CLAUDE_CONFIG_DIR`) : il a donc sa connexion, ses sessions et son quota. Escouade ne partage jamais la connexion d’un compte avec un autre.

Cette page explique l’usage ; la liste des options de l’onglet est dans [les réglages des comptes](settings-accounts).

## Aucun compte au départ

Une première installation n’a aucun compte, pour aucun harness, Claude Code compris : seuls les comptes que tu ajoutes existent et comptent (choix du compte d’un agent ou d’un ticket, bascule, passation à un autre harness, quotas, statistiques, serveur MCP, textes rédigés). Réglages › Comptes montre chaque harness installé, avec « Aucun compte » et le bouton « Ajouter un compte Codex… » tant qu’il n’en a pas.

Sans compte pour son harness, rien ne démarre en silence : un nouvel agent, un ticket lancé, le pilote auto, une passation ou un texte rédigé est refusé (« Aucun compte Claude Code : ajoute-en un dans Réglages › Comptes. »), avec un bouton « Ajouter un compte… » sur ce harness. L’écran d’accueil et l’étape « Compte et commit » de l’assistant « Nouveau projet » y mènent aussi ; depuis l’assistant, la fenêtre s’ouvre par-dessus et ce que tu as saisi reste.

> [!NOTE]
> À la mise à jour vers la 1.8.1, le compte Principal de chaque harness reste, comme un compte ajouté (mêmes agents, tickets et statistiques), s’il est connecté ou s’il a déjà servi. Un Principal jamais connecté ni utilisé disparaît. Rien d’autre ne change si tu n’avais que ton compte Claude Code.

## Le compte de la machine (Principal)

« Principal » est ton compte habituel du harness, celui de son dossier habituel (`~/.claude` pour Claude Code), une fois ajouté comme « Le compte de la machine ». Sa fiche le marque « Compte de la machine ».

- Escouade le lance comme avant : elle ne lui donne pas de dossier de configuration. Il garde ta connexion et tes conversations. Son dossier est celui de la variable `CLAUDE_CONFIG_DIR` quand elle est définie pour l’app, sinon `~/.claude`. Sous macOS, lui donner un dossier te déconnecterait : Claude Code change alors le nom de l’entrée du trousseau où il garde la connexion.
- Il s’appelle « Principal » tant que tu ne l’as pas renommé, dans la langue de l’interface.
- Il se retire comme les autres comptes : le retrait ne déconnecte pas la machine (la confirmation le dit). Comme eux, il se renomme, se désactive (tant qu’un autre compte du harness est actif), se réserve à des projets et peut avoir son propre exécutable.
- S’il n’est pas connecté par un compte claude.ai (avec une clé d’API, par exemple), l’onglet l’indique : « Pas connecté par un compte claude.ai (clé d’API ?) ».

## Ajouter un compte

1. Ouvre les réglages (`Ctrl+,`, ou ⚙ dans la barre de statut) et l’onglet « Comptes ».
2. Clique sur « Ajouter un compte… » (ou sur « Ajouter un compte Codex… » dans le groupe d’un harness sans compte).
3. Choisis le compte :
   - « Le compte de la machine » : le compte déjà connecté sur cette machine, dans le dossier habituel du harness, proposé quand le harness y est connecté (« Claude Code est connecté sur cette machine · ada@exemple.dev ») et qu’il n’est pas déjà ajouté (« Déjà ajouté »). « Ajouter ce compte » l’ajoute tel quel : il n’y a rien à connecter.
   - « Un nouveau compte » : un dossier à lui sous `~/.escouade`, et sa propre connexion. Continue alors ci-dessous.
4. Écris le nom du compte dans « Nom » (le champ suggère « Pro, Équipe… »). Deux comptes ne peuvent pas porter le même nom, majuscules ou non.
5. Choisis ce que le compte reprend du compte de la machine, sous « Partager avec… » (voir ci-dessous).
6. Clique sur « Créer et se connecter ». Le compte est créé, actif et placé en dernier dans la liste, puis la fenêtre ouvre sa connexion.

Dès que deux harness sont installés, la fenêtre commence par « Harness » : le harness du compte, Claude Code, Codex ou Copilot CLI (voir [les comptes Codex](#comptes-codex) et [les comptes Copilot CLI](#comptes-copilot-cli)).

Escouade crée pour le compte un dossier sous `~/.escouade/claude/`, nommé d’après le compte. Elle ne reprend jamais le dossier d’un compte supprimé : un nouveau compte a toujours un dossier neuf.

> [!NOTE]
> Sous macOS, `Ctrl` devient `⌘` dans les raccourcis (sauf `Ctrl+Tab`).

### Ce qu’un compte reprend de Principal

Un nouveau compte peut reprendre une partie de ce que tu as réglé dans Claude Code pour Principal. Seuls les éléments que le dossier de Principal contient sont proposés, et ils sont tous cochés au départ. Si Principal n’a rien à partager, la fenêtre le dit.

| Élément | Type |
|---|---|
| `settings.json` | Fichier : les réglages de Claude Code |
| `CLAUDE.md` | Fichier : tes consignes |
| `skills`, `agents`, `commands` | Dossiers : skills, agents et commandes |
| `plugins`, `hooks` | Dossiers : plugins et hooks |
| `output-styles` | Dossier : styles de sortie |

Le choix « Partage » dit comment ces éléments sont repris :

- « Lier (un changement vaut pour les deux comptes) » (par défaut) : le compte pointe vers l’élément de Principal. Une modification faite d’un côté se retrouve de l’autre. Sous Windows, les dossiers sont liés par une jonction, qui ne demande pas de droits d’administrateur, mais les fichiers sont copiés, parce qu’un lien vers un fichier en demande ; la fenêtre le rappelle : « copiés (un lien de fichier demande des droits d’administrateur) ».
- « Copier » : le compte reçoit une copie, et chacun modifie ensuite la sienne.

Deux cas font exception. Quand le dossier de Principal est dans une distribution WSL, tout est copié, quel que soit le choix. Sous Windows, un chemin qui contient `%` ou un guillemet droit ne peut pas être lié : le message te demande alors de choisir « Copier ».

> [!NOTE]
> La connexion (`.credentials.json`) et le fichier `.claude.json` (l’adresse e-mail, les serveurs MCP et les projets du compte) ne sont jamais partagés : chaque compte a les siens.

### Se connecter au compte

Après « Créer et se connecter », la fenêtre change de titre (« Se connecter au compte » suivi du nom du compte) et affiche un terminal où tourne le `claude` du compte, avec son dossier de configuration.

1. Connecte-toi dans ce terminal, comme dans Claude Code : suis les instructions, le navigateur s’ouvre. Si Claude Code ne propose pas la connexion, tape `/login`.
2. Escouade cherche la connexion toutes les 2 secondes. Dès qu’elle la trouve, la fenêtre écrit « Connecté au compte ada@exemple.dev » (ou « Connecté » quand l’adresse n’est pas encore connue), ferme le terminal une seconde et demie plus tard et lit le quota du compte.
3. Clique sur « Terminé ».

Si Claude Code s’arrête avant la connexion, la fenêtre écrit « Claude Code s’est arrêté avant la connexion. » et propose « Relancer ». Si tu fermes la fenêtre en cours de route, le compte reste dans la liste, « Pas connecté » : reprends avec « Se connecter… ».

« Se connecter… » sert aussi à reconnecter un compte dont la connexion a expiré. Quand le compte est connecté et à jour, le bouton devient « Se reconnecter… » : utile pour passer à un autre compte claude.ai.

Quand Claude Code n’arrive pas à rafraîchir la connexion d’un compte, l’agent qui s’arrête reprend tout seul si la connexion sur le disque est bonne, sinon la carte « Connexion à refaire » propose « Se reconnecter » et l’agent reprend une fois connecté. Voir [le dépannage](troubleshooting).

## Comptes Codex

Un compte Codex, c’est Codex lancé avec son propre dossier (la variable `CODEX_HOME`) : sa connexion à ChatGPT, ses sessions et son quota. Ils se gèrent dans le groupe « Comptes Codex » de l’onglet, comme ceux de Claude Code.

- Son « Principal » est ton Codex habituel, `~/.codex` : Escouade le lance sans toucher à `CODEX_HOME`. « Ajouter un compte… » le propose comme « Le compte de la machine » quand Codex y est connecté.
- « Ajouter un compte… », harness « Codex », crée son dossier sous `~/.escouade/codex/`. Il peut reprendre de Principal `config.toml` (ses réglages, dont le bac à sable de Windows et la confiance de tes dossiers), `AGENTS.md` et ses dossiers `skills`, `prompts` et `rules`, par lien ou par copie, comme un compte Claude Code.
- La connexion (`auth.json`) n’est jamais copiée : son jeton change à chaque renouvellement, et deux dossiers qui s’en serviraient se déconnecteraient l’un l’autre.
- « Créer et se connecter » ouvre un terminal où tourne `codex login` dans le dossier du compte : le navigateur s’ouvre sur la connexion de ChatGPT. Dès qu’`auth.json` est écrit, la fenêtre écrit « Connecté au compte » suivi de l’adresse du compte ChatGPT.

Le quota d’un compte Codex vient de Codex lui-même : de ses agents pendant qu’ils tournent, sinon d’un app-server du compte lancé toutes les 5 minutes le temps de le lire, sans tour ni conversation. Ses fenêtres prennent un libellé selon leur durée (« 5h », « 7j », ou une seule fenêtre « 30j » avec l’offre gratuite). Un tour arrêté par la limite attend la remise à zéro puis reprend seul, et les nouveaux agents Codex partent sur le compte Codex suivant sous le seuil, comme chez Claude Code. Reprendre un agent Codex sur un autre compte n’est pas proposé : il attend sa remise à zéro, ou continue sur un autre harness.

## Comptes Copilot CLI

Un compte Copilot CLI, c’est Copilot lancé avec son propre dossier (la variable `COPILOT_HOME`) : sa connexion à GitHub, ses sessions et ses crédits d’IA. Ils se gèrent dans le groupe « Comptes Copilot CLI » de l’onglet.

- Son « Principal » est ton Copilot habituel, `~/.copilot` : Escouade le lance sans toucher à `COPILOT_HOME`.
- « Ajouter un compte… », harness « Copilot CLI », crée son dossier sous `~/.escouade/copilot/`. Il peut reprendre de Principal `mcp-config.json` (tes serveurs MCP), `copilot-instructions.md` et ses dossiers `agents` et `skills`, par lien ou par copie.
- La connexion n’est jamais copiée : `config.json` (qui dit quel utilisateur GitHub est connecté) et `settings.json` restent à Principal, et le jeton reste dans le trousseau du système.
- « Créer et se connecter » ouvre un terminal où tourne `copilot login` dans le dossier du compte : il donne un code à saisir sur github.com/login/device. Dès que la connexion est faite, la fenêtre écrit « Connecté au compte » suivi de `@` et de ton nom d’utilisateur GitHub. Escouade ne prend ni ne recopie aucun jeton, pas même celui du GitHub CLI.

Deux comptes ne se mélangent pas. Faute de connexion à lui, Copilot prend de lui-même le compte du GitHub CLI (`gh`), et il préfère un jeton de l’environnement (`GH_TOKEN`, `GITHUB_TOKEN`, `COPILOT_GITHUB_TOKEN`) à sa connexion : un compte autre que Principal ne reçoit jamais ces jetons, et sur le compte du GitHub CLI il est tenu pour non connecté (son agent ne démarre pas et la conversation dit de le connecter). Principal, lui, peut tourner sur le compte du GitHub CLI : la conversation le dit.

Copilot compte en crédits d’IA, par mois : la barre « crédits » dit ce qui en est utilisé et quand ils reviennent (le 1ᵉʳ du mois). Ils viennent de chaque appel de ses agents, sinon d’un `copilot` du compte lancé au plus toutes les 5 minutes le temps de les lire, sans session ni modèle. Au-delà de « Pause au-delà du quota », le pilote auto se met en pause comme pour les fenêtres de Claude Code. Un tour arrêté par les crédits épuisés reprend seul à leur retour. Si une session a un plafond de crédits (`--max-ai-credits`) et l’atteint, une carte propose d’ajouter autant de crédits, de lever le plafond ou d’arrêter là.

## Compte OpenCode

OpenCode n’a qu’un compte dans Escouade : son « Principal », ta config habituelle d’OpenCode et les fournisseurs que tu y as connectés (« Se connecter… » lance `opencode auth login`). « Ajouter un compte… » le propose comme « Le compte de la machine » dès qu’OpenCode est installé (« OpenCode est installé sur cette machine »).

OpenCode n’a pas de quota : son coût est celui que donnent tes fournisseurs, 0 avec un modèle local. La barre d’état montre, à la place des barres, le coût du jour de son compte. Pour le borner, remplis « Budget par jour ($) » sur son bloc, dans Réglages › Comptes (vide par défaut) : une fois le coût du jour atteint, le pilote auto se met en pause jusqu’à minuit, comme au quota épuisé, et la barre d’état le montre (« 0,42 $ / 5,00 $ »). « Reprendre maintenant » lève la pause jusqu’à minuit.

## Gérer les comptes

L’onglet « Comptes » des réglages liste les comptes. Chaque changement est enregistré tout de suite, sans passer par l’enregistrement des autres réglages (« Annuler » ne le défait pas). Une ligne par compte :

| Élément | Rôle |
|---|---|
| Nom | Un champ modifiable. Le changement est pris quand tu quittes le champ ou que tu tapes `Entrée`. |
| État | « Vérification… », « Connecté · ada@exemple.dev », « Connecté », « Pas connecté » ou « Inactif ». Une connexion expirée est signalée en ambre sous la ligne : « Connexion expirée : relance Claude Code pour ce compte. » |
| Actif | Un interrupteur. Un compte inactif ne reçoit plus de nouvel agent. Le dernier compte actif ne peut pas être coupé (« Il faut au moins un compte actif. »). |
| ↑ et ↓ | L’ordre des comptes. Les nouveaux agents les essaient dans cet ordre (« Monter Pro », « Descendre Pro »). |
| Dossier | Le dossier de configuration du compte, en lecture seule. |
| Exécutable | Le `claude` que ce compte lance. Vide (« Celui des réglages »), il prend l’exécutable de l’onglet « Harness ». |
| Se connecter… / Se reconnecter… | Ouvre la connexion du compte (voir plus haut). |
| Nouveaux agents | Le « Modèle », l’« Effort » et le « Mode » avec lesquels partent les nouveaux agents de ce compte, parmi ceux de son harness (les modèles que son harness liste pour lui, les efforts de son modèle, les modes de son harness). « Celui du harness » prend le défaut du harness. |
| Supprimer | Retire le compte (voir plus bas), le compte de la machine compris. |
| Réservé aux projets | Les projets auxquels le compte est réservé (voir plus bas). |

Sous la liste, une ligne ambre apparaît quand un projet n’a plus aucun compte actif pour le servir.

## Sur quel compte part un agent

### Le choix automatique

Un nouvel agent, ou un ticket que le pilote auto prend, part sur :

1. le « Compte préféré » du projet, s’il en a un, que ce compte est actif et qu’il sert ce projet ;
2. sinon le premier compte actif de la liste, dans l’ordre, qui sert le projet et dont les quotas ne sont pas au seuil de « Pause au-delà du quota » (un compte qui vient de buter sur sa limite d’usage est aussi écarté) ;
3. si aucun compte n’est sous le seuil, le premier compte actif. Le [pilote auto](autopilot) se met alors en pause.

L’ordre de la liste décide donc : mets en tête le compte que tu veux user en premier.

Le seuil se règle dans Réglages › Kanban, groupe « Pilote auto » : 80, 90, 95 ou 100 % (100 % par défaut), pour tous les projets. Il est détaillé dans [les réglages du Kanban](settings-board).

Quand le compte qui recevait les nouveaux agents passe le seuil et qu’un autre est disponible, une notification le dit : « Le compte Principal a atteint 91 % : les nouveaux agents partent sur Pro. »

### Le compte préféré d’un projet

Avec plusieurs comptes, les réglages d’un projet (onglet « Projets », groupe « Identité ») ont un choix « Compte préféré » : le compte sur lequel partent les nouveaux agents et les tickets du projet.

- « Automatique » (par défaut) : le choix automatique ci-dessus.
- Un compte précis : il est pris même si son quota est au seuil. Les tickets du projet n’attendent alors que ce compte-là.

Un compte préféré qui est devenu inactif, qui est réservé à d’autres projets ou qui n’existe plus est ignoré : le projet repasse en automatique. Supprimer un compte efface aussi la préférence des projets qui l’avaient choisi.

### Choisir le compte à la main

Avec plusieurs comptes, le champ de saisie d’un agent (voir [le chat](chat)) a un menu « Compte » à côté du modèle. Tant que tu n’as rien écrit à l’agent, il propose :

- « Automatique (Pro) » : le compte que le choix automatique prendrait maintenant (« Celui du projet, sinon le premier compte sous les seuils de pause et de changement de compte. ») ;
- chaque compte actif qui sert le projet.

Choisir un compte change celui de l’agent avant qu’il ne démarre.

### Chaque agent reste sur son compte

Dès le premier message, le compte d’un agent ne change plus : le menu devient en lecture seule (« Le compte d’un agent ne change plus après son premier message (sa conversation est rangée dans ce compte). »). Claude Code range en effet chaque conversation dans le dossier du compte qui la lance.

Deux choses déplacent un agent, avec sa session : « Reprendre sur… » et « Changer de compte à » (plus bas). Les statistiques comptent chaque tour sur le compte qu’avait l’agent à ce moment-là.

## Réservé aux projets

Chaque compte, Principal compris, peut être gardé pour certains projets. Sous un compte, la ligne « Réservé aux projets » liste tes projets avec une case à cocher chacun. Tant qu’aucune case n’est cochée, la ligne finit par « · Tous les projets » : le compte sert tous les projets. Le survol de l’intitulé rappelle le principe : « Coche des projets pour garder ce compte à eux seuls (les projets d’un client, par exemple) : les autres n’y partent jamais. »

Un projet que tu ne coches pas ne peut jamais utiliser le compte :

- ni par le choix automatique, pour un agent comme pour un ticket ;
- ni par le menu « Compte » de l’agent ;
- ni comme « Compte préféré », que le projet ne propose pas ;
- ni par « Reprendre sur… », ni par la reprise automatique.

Les agents déjà partis sur le compte y restent.

C’est fait pour isoler un client : coche ses projets sur son compte, et ses projets ne partent que sur lui.

Si aucun compte actif ne sert un projet (par exemple parce que chaque compte est réservé à d’autres projets), rien ne démarre pour lui :

- le Kanban du projet écrit « Aucun compte Claude pour ce projet — aucun ticket ne démarre » ;
- un nouvel agent est refusé, avec un message qui dit où cocher le projet ;
- l’onglet « Comptes » le dit sous la liste, en ambre, par exemple : Aucun compte actif ne sert « Client » : ses agents et ses tickets ne démarrent pas.

Pour y remédier, coche le projet sur un compte actif, ou décoche toutes les cases d’un compte pour qu’il serve de nouveau tous les projets.

« Fermer le projet » (zone sensible des réglages du projet) garde l’historique du projet, et le projet reste dans la liste « Réservé aux projets » des comptes qui lui étaient réservés : il n’en sort que lorsque son historique est effacé (la case « Effacer aussi son historique » à la fermeture, ou « Effacer l’historique… » dans « Projets fermés »).

Un compte qui n’était réservé qu’à ce projet ne sert donc aucun projet ouvert tant que le projet est fermé. Si c’était ton seul compte actif, plus rien ne démarre pour les autres projets, et l’onglet « Comptes » le dit sous la liste, comme plus haut.

Cet onglet montre le projet fermé dans la ligne « Réservé aux projets » du compte, coché et marqué « (fermé) », par exemple « Client (fermé) ». Décoche-le pour libérer le compte : sans case cochée, il sert de nouveau tous les projets. Quand tu rouvres le projet (voir [Projets fermés](settings-projects#projets-fermes)), ses réservations sont toujours là : il n’y a rien à refaire. Si tu effaces l’historique, à la fermeture ou plus tard dans « Projets fermés », le projet sort de la liste de chaque compte. Le « Compte préféré » du projet, lui, repasse sur « Automatique » à la réouverture si son compte n’existe plus.

### Un compte pour un projet privé

Deux projets sur le même compte partagent son dossier : les sessions, la mémoire, l’historique et les réglages globaux du harness. Un [projet privé](mcp#les-projets-prives) n’est cloisonné tout à fait qu’avec un compte réservé à lui seul : coché pour ce projet et aucun autre.

Dès que tu allumes « Projet privé » dans les réglages du projet, la rangée « Comptes » dit où ses agents peuvent partir. Un compte réservé à lui seul : « Compte réservé à ce projet : « Pro ». Sessions, mémoire et historique de Claude Code n’y sont partagés avec aucun autre projet. » Un compte encore partagé, en ambre : « Ses agents peuvent partir sur « Principal », un compte partagé avec d’autres projets : sessions, mémoire, historique et réglages globaux de Claude Code y sont communs. » Pour y remédier :

- « Ajouter un compte réservé… » ouvre « Ajouter un compte », sur un nouveau compte, avec son propre dossier. Le compte créé est réservé au projet seul, puis tu reviens à ses réglages.
- « Réserver Principal à ce projet » réserve tout de suite un compte existant au projet seul : les autres projets n’y partent plus.

L’onglet « Comptes » le dit aussi, sous un compte où part encore un projet privé : « Le projet privé « Client » part encore sur ce compte, partagé avec d’autres projets. », avec « Réserver à « Client » ».

Les agents d’un projet privé qui a un compte à lui ne basculent jamais d’eux-mêmes sur un compte partagé : au-delà du seuil, ses nouveaux agents et ses tickets restent sur son compte, la reprise automatique d’un agent de ticket ne va que sur un autre compte à lui, et le passage automatique à un autre harness seulement si le compte qu’il y prendrait est à lui. Sinon, ils attendent la remise à zéro. Sans compte à lui, le projet garde ses comptes partagés, mais aucune bascule automatique ne le fait changer de compte. Pour le permettre, allume « Comptes partagés à la limite » dans les réglages du projet. « Reprendre sur… » et « Continuer sur… » restent possibles à la main.

Ce qu’un nouveau compte partage avec Principal à sa création (réglages, instructions, skills…) reste partagé, jamais ses sessions ni sa mémoire.

## Changer de compte avant la limite

Le réglage « Changer de compte à » (Réglages › Comptes, groupe « Quotas ») fait passer un agent sur un autre compte quand le sien approche de la limite : à 80 %, 90 % ou 95 % de sa fenêtre de 5 h ou de sa fenêtre hebdomadaire, la première atteinte. Par défaut, il est « À la limite » : rien ne bouge avant, comme avant ce réglage.

Au seuil, l’agent ne s’arrête jamais en plein travail :

- il finit son tour sur son compte, puis son process repart sur l’autre compte, avec sa session : son tour suivant y part ;
- s’il attend ta réponse (une question, une permission) ou si une tâche tourne en arrière-plan dans son process (une commande, une surveillance, un sous-agent), il reste tant qu’elles sont là, et passe à la première fin de tour sans elles ;
- les agents ordinaires comme ceux des tickets passent ; les nouveaux agents partent directement sur l’autre compte.

Le compte d’arrivée est le compte préféré du projet, sinon le premier de la liste, à condition qu’il serve le projet, soit connecté et reste lui-même sous le seuil. Un projet privé qui n’a pas allumé « Comptes partagés à la limite » ne passe que sur un compte réservé à lui seul. Seul Claude Code déplace un agent déjà lancé : un agent Codex, Copilot CLI ou OpenCode reste sur son compte, ses nouveaux agents suivent le seuil.

La conversation le dit (« L’agent passe sur le compte Pro : Principal est à 96 % de sa fenêtre de 5 h. »), une notification aussi, et le menu « Compte » de l’agent montre le nouveau compte une fois son process relancé dessus.

Sans compte utilisable, l’agent reste où il est et sa conversation dit pourquoi, une fois : « Principal est à 96 % de sa fenêtre de 5 h, mais aucun autre compte Claude Code ne peut prendre l’agent (sous le seuil, connecté, servant ce projet) : il y reste. » Si ce compte continue en usage supplémentaire, elle ajoute : « Ce compte est en usage supplémentaire, facturé à part. »

Pas de va-et-vient : un agent passé sur un compte ne le quitte que quand celui-ci atteint le seuil à son tour, et le premier ne redevient un choix qu’à sa remise à zéro.

## Reprendre un agent arrêté par la limite d’usage

### Ce qui se passe à la limite

Quand Claude refuse un tour parce que le quota du compte est épuisé, l’agent s’arrête. La carte d’erreur du tour, la carte de l’agent dans la barre latérale (« Reprise à 15:00 ») et le Kanban donnent l’heure de la reprise automatique.

Par défaut, Escouade envoie « continue » à l’agent une trentaine de secondes après la remise à zéro du quota, quand elle en connaît l’heure. C’est le réglage « Reprise automatique après la limite d’usage » (Réglages › Harness, groupe « Processus »). Le bouton « Annuler la reprise » de la carte l’annule pour cet agent.

### Reprendre sur un autre compte

Avec plusieurs comptes, la carte propose « Reprendre sur Pro » : le premier compte éligible dans l’ordre de la liste. Quand plusieurs comptes conviennent, le bouton ▾ (« Reprendre sur un autre compte ») ouvre le menu des autres.

Un compte est éligible s’il est actif, différent de celui de l’agent, au service de son projet, connecté et sous le seuil de pause : un compte déjà au seuil est écarté, l’agent y retrouverait la limite.

En cliquant, tu fais ceci :

1. Escouade arrête le processus Claude de l’agent.
2. Elle copie sa session (la conversation et ce que ses sous-agents et ses outils y ont gardé) dans le dossier de l’autre compte.
3. L’agent passe sur ce compte, et sa reprise automatique est annulée.
4. Escouade lui envoie « continue », comme pour une reprise automatique.

La conversation garde une trace : « L’agent reprend sur le compte Pro (il était sur Principal). »

Le déplacement a quelques conditions : l’agent doit avoir fini son tour (« L’agent travaille encore : reprends quand son tour est fini. ») et avoir une session. Une session ne passe pas non plus d’un compte qui lance Claude Code sous Windows à un compte qui le lance dans WSL, ni l’inverse.

Si le tour qui suit échoue sur le nouveau compte, la carte écrit « La reprise sur Pro a échoué : … » et propose « Revenir sur Principal ». Ce bouton ramène la session sur le compte d’origine, avec ce que le tour raté y a ajouté, et l’agent attend de nouveau la remise à zéro qu’il attendait.

### Tout seul, pour les agents de ticket

Le réglage « Reprendre sur un autre compte un agent de ticket arrêté par la limite » (Réglages › Harness, groupe « Processus », visible avec plusieurs comptes) fait la même chose sans toi. Il est désactivé par défaut ; sa description dit : « Sinon il attend la remise à zéro de son compte ».

Activé, l’agent d’un ticket arrêté par la limite passe de lui-même sur un autre compte quand l’un convient : le compte préféré du projet, sinon le premier de la liste. Il faut que ce compte soit connecté, sous le seuil, qu’aucune pause après une limite d’usage ne le retienne, qu’aucun agent n’y attende sa propre remise à zéro, et qu’il soit du même côté de WSL que l’agent. S’il n’y en a pas, ou si le déplacement échoue (la conversation écrit alors « Reprise sur un autre compte impossible : … »), l’agent attend la remise à zéro de son compte, comme avant.

Le compte quitté ne reçoit plus de nouveau ticket jusqu’à sa remise à zéro (30 minutes quand elle n’est pas connue). Un projet dont le « Compte préféré » est le compte de l’agent garde son agent là : il attend.

### Continuer sur un autre harness

Quand plus aucun compte du harness de l’agent qui sert son projet ne peut le prendre (limite atteinte, ou au-delà de « Pause au-delà du quota »), la carte propose « Continuer sur Codex » : le harness suivant du projet. Un projet passe d’abord par son harness par défaut, puis par les autres dans l’« Ordre des harness » (Réglages › Comptes), seulement ceux qui ont un compte actif qui le sert : un compte « Réservé aux projets » ne sert que ceux qu’il coche. Le bouton ▾ (« Continuer sur un autre harness ») ouvre le menu des suivants. Sans harness de secours, l’agent attend la remise à zéro, comme avant.

Une session ne passe pas d’un harness à l’autre : c’est une passation. L’agent garde sa conversation, son worktree et sa branche ; un nouveau processus du harness cible démarre sur le compte que choisit sa règle, avec le « Modèle de passation » de ce harness (son meilleur par défaut, Réglages › Harness) et le même mode, ou le plus proche qui n’autorise jamais plus. Son premier message est un dossier de passation écrit par Escouade depuis la conversation, sans rien demander au harness quitté : la demande d’origine et les messages suivants, les derniers messages de l’agent, son avancement et les tâches de son plan, les fichiers modifiés (`git status`, `git diff --stat`, les commits de la branche), le dernier tour s’il a été coupé et où, une question ou une permission restée sans réponse. Un ticket garde son protocole par les consignes ajoutées. Le dossier est borné à la fenêtre de contexte du modèle cible ; il est replié dans la conversation sous « Dossier de passation ».

La conversation le marque : « Suite sur Codex · quota Claude Code épuisé jusqu’à 18:00 », puis chaque tour dit le harness qui l’a fait, et ses statistiques et son coût lui restent.

Le menu de l’agent propose aussi « Continuer sur un autre harness… » à tout moment, hors d’un tour en cours.

Un agent en vue terminal (Vue Claude Code, Vue Codex…) en change aussi : le dossier de passation part dans la vue Escouade, puis, son tour fini, l’agent passe dans la vue terminal du harness cible, sur sa nouvelle session. Un harness sans vue terminal le laisse en vue Escouade, comme un agent qui y était déjà.

Le harness quitté est mis de côté jusqu’à sa remise à zéro. Ensuite, la carte et le menu proposent « Revenir sur Claude Code » : une passation dans l’autre sens. L’agent n’y revient jamais seul.

Le réglage « Continuer sur le harness suivant quand tous les comptes d’un harness sont épuisés » (Réglages › Harness, groupe « Processus », visible avec deux harness) le fait sans toi. Il est désactivé par défaut. Activé, l’agent arrêté par la limite passe seul sur le premier harness de secours qui a un compte pour le prendre, jamais sur celui qu’il a quitté ; le pilote auto suit son ticket sur le nouveau harness.

## Désactiver ou supprimer un compte

**Désactiver.** L’interrupteur « Actif » garde le compte dans la liste, avec sa connexion, son dossier et ses conversations, mais plus aucun nouvel agent, ticket ou reprise ne part dessus. L’état devient « Inactif ». Les agents qui y sont déjà y restent ; leur menu « Compte » l’écrit « Pro (inactif) ».

**Supprimer.** Le bouton « Supprimer » demande confirmation. Pour le compte de la machine, la fenêtre s’intitule « Retirer le compte de la machine » et dit que le harness reste connecté sur cette machine : le retirer ne déconnecte rien. La fenêtre commence par « Supprimer le compte » suivi du nom du compte, puis dit : « Les nouveaux agents ne partiront plus dessus. Son dossier reste sur le disque, avec sa connexion et ses conversations : » et le chemin du dossier.

- Tant que des agents non archivés tournent sur le compte, la suppression est refusée : « Le compte sert encore à 2 agents. » Archive-les ou supprime-les d’abord.
- Le dossier du compte reste sur le disque : Escouade ne l’efface pas, et ne le reprend pas si tu ajoutes un compte du même nom. Pour retrouver le compte, ajoute-en un nouveau et reconnecte-toi.
- Les projets qui avaient ce compte pour « Compte préféré » repassent en « Automatique ».
- Ses tours restent dans les statistiques, sous « Compte supprimé (pro) ».
- Un agent archivé sur ce compte, que tu restaures plus tard, part sur le compte où irait un nouvel agent de son projet sur le même harness ; sans compte de ce harness, il ne démarre pas et le dit.

## Quotas et statistiques par compte

- La barre de statut montre le quota du compte sur lequel partirait un nouvel agent ; un clic ouvre le détail de chaque compte. Voir [les quotas et le coût](usage).
- Les [statistiques](stats) se filtrent par compte, et leur tableau « Par compte » met les comptes côte à côte.
- Quand un compte dépasse son quota de 5 h mais continue en usage supplémentaire (facturé à part, si ton abonnement l’a activé), la barre de statut le dit : les agents déjà lancés restent sur ce compte, seuls les nouveaux partent sur un autre. Pour déplacer un agent, utilise « Reprendre sur un autre compte », ou règle « Changer de compte à » sous la limite : ils passent alors sur un autre compte à la fin de leur tour.
- Un compte qui a son propre dossier ne prend jamais l’identité d’une variable de ton environnement (`ANTHROPIC_API_KEY`, `CLAUDE_CODE_OAUTH_TOKEN`, `ANTHROPIC_AUTH_TOKEN`, `CLAUDE_SECURESTORAGE_CONFIG_DIR`) : Escouade les écarte pour lui. Principal les garde, comme hors d’Escouade.
