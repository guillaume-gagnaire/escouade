# Dépannage

Les problèmes que rencontre le plus souvent Escouade, chacun avec son symptôme, sa cause et sa solution : Claude Code introuvable, WSL, proxy, quotas, limite d’usage, worktree, git, notifications. À la fin, où lire le journal de l’app et comment signaler un bug.

## Claude Code

### Claude Code est introuvable

**Symptôme.** L’écran d’accueil dit « Claude Code est introuvable sur ce poste ». Le Kanban affiche « Claude Code introuvable — aucun ticket ne démarre », et Réglages › À propos « Claude Code introuvable ».

**Cause.** Escouade cherche `claude` dans cet ordre :

1. Le chemin de Réglages › Harness, champ « Chemin de l’exécutable », quand il est rempli. S’il est rempli, il décide seul : un chemin faux donne « introuvable », même si `claude` est dans le `PATH`.
2. Sinon, le `PATH`, puis les emplacements habituels des installeurs. Sous Windows : `%USERPROFILE%\.local\bin\claude.exe`, `%USERPROFILE%\.claude\local\claude.exe` et `%USERPROFILE%\AppData\Roaming\npm\claude.cmd`. Sous macOS et Linux : `~/.local/bin/claude`, `~/.claude/local/claude`, `/opt/homebrew/bin/claude` et `/usr/local/bin/claude`.
3. Sous Windows, à défaut, un Claude Code installé dans WSL (voir plus bas).

Le message le dit avec la commande qui le réinstalle : « Claude Code est introuvable : réinstalle-le (…), ou indique son chemin dans Réglages › Harness. », à la connexion d’un compte, au lancement d’un agent et dans Réglages › Harness.

**Solution.** Installe Claude Code et connecte-le, ou indique son chemin dans Réglages › Harness (voir les [réglages des harness](settings-claude-code)). Le champ accepte `~` pour ton dossier personnel, et sous Windows on peut y omettre `.exe` ou `.cmd`. La ligne sous le champ répond tout de suite : « Trouvé : … » avec le chemin retenu, ou « Introuvable. Indique le chemin de claude.exe, … ». Vide le champ pour revenir à la détection automatique (« vide = détection automatique »).

Un compte Claude peut avoir son propre exécutable (Réglages › Comptes, champ « Exécutable », « Celui des réglages » par défaut). Un chemin qui existe mais ne se lance pas donne « impossible de lancer » suivi de ce chemin.

### Claude Code a disparu après une mise à jour interrompue

**Symptôme.** La connexion de chaque compte s’arrête aussitôt, ou Escouade dit « L’installation de Claude Code a été interrompue (il en reste …) ». Dans un terminal, la commande `claude` n’existe plus.

**Cause.** Une mise à jour de Claude Code installé par npm (`npm install -g`) a été coupée à mi-chemin : npm avait retiré l’ancien `claude` et n’a pas posé le nouveau. Il en reste un lien temporaire `.claude-XXXX` à côté de l’ancien (dans `/opt/homebrew/bin` par exemple), ou l’ancien paquet mis de côté (`@anthropic-ai/.claude-code-XXXX`). Jusqu’à la 1.8.4, un `claude` lancé par Escouade pouvait commencer cette mise à jour, et Escouade la couper en l’arrêtant ; depuis, ses `claude` ne se mettent plus à jour eux-mêmes et Escouade lance `claude update` à part (voir les [réglages des harness](settings-claude-code)).

**Solution.** Dans un terminal, relance l’installation :

```sh
npm install -g @anthropic-ai/claude-code
```

Si npm refuse (`ENOTEMPTY`), supprime d’abord ce qui reste (le chemin que donne le message), puis relance. Escouade retrouve Claude Code tout seul.

### Claude Code est introuvable quand l’app démarre depuis le Finder (macOS)

**Symptôme.** `claude` répond dans le Terminal, mais Escouade, ouverte depuis le Finder ou le Dock, ne le trouve pas, ni `git` ou `node` si tu les as installés avec Homebrew.

**Cause.** macOS donne aux apps lancées depuis le Finder ou le Dock un `PATH` minimal, pas celui de ton terminal. Au démarrage, Escouade le complète avec celui de ton shell de connexion (elle l’interroge par `$SHELL -ilc`), puis ajoute `/opt/homebrew/bin`, `/usr/local/bin` et `~/.local/bin`. Elle n’attend le shell que cinq secondes : un profil qui demande une saisie, ou qui met plus longtemps, annule cette étape. La même chose vaut pour un lanceur de bureau sous Linux.

**Solution.** Indique le chemin complet de `claude` dans Réglages › Harness (`which claude` dans un terminal le donne), ou allège le profil de ton shell pour qu’il réponde vite.

### Claude Code est installé dans WSL

**Symptôme.** Claude Code n’est installé que dans WSL, et Escouade ne le trouve pas, ou un agent ne peut pas travailler dans un dossier.

**Cause.** Sous Windows, Escouade ne regarde dans WSL que si Windows n’a pas de `claude`. Elle cherche dans la distribution de Réglages › Terminaux (champ « Distribution WSL », « vide = distribution par défaut »), à `~/.local/bin/claude`, `~/.claude/local/claude`, `/usr/local/bin/claude` et `/usr/bin/claude`. Cette recherche peut démarrer la distribution et prendre quelques secondes.

**Solution.** Indique le chemin dans Réglages › Harness, sous l’une de ces formes :

- `~/.local/bin/claude`
- `/home/<utilisateur>/.local/bin/claude`
- `\\wsl.localhost\<distribution>\home\<utilisateur>\.local\bin\claude`

La ligne sous le champ confirme : « Trouvé dans WSL (Ubuntu) : /home/ada/.local/bin/claude », par exemple. Escouade lance alors `claude` par `wsl.exe`, dans le dossier de l’agent (vu de WSL, `/mnt/c/…`), et lit ses sessions et sa connexion dans le dossier personnel Linux. Un dossier de projet que WSL ne peut pas atteindre (un partage réseau, le dossier d’une autre distribution) est refusé : « Le dossier … n’est pas joignable depuis WSL (…) : le Claude Code qui y est installé ne peut pas y travailler. »

### Les outils MCP et le suivi de la vue Claude Code ne marchent pas depuis WSL

**Symptôme.** Sous le champ du chemin, Réglages › Harness affiche : « Le réseau de WSL est en NAT : le serveur MCP d’Escouade et le suivi de la vue Claude Code ne joignent pas l’app. »

**Cause.** En mode NAT, WSL n’est pas sur le réseau de Windows : le serveur MCP d’Escouade et les hooks de la vue Claude Code, qui écoutent sur le `127.0.0.1` de Windows, lui sont hors de portée.

**Solution.** Passe WSL en réseau miroir. Dans `%UserProfile%\.wslconfig` :

```ini
[wsl2]
networkingMode=mirrored
```

Puis, dans un terminal Windows, lance `wsl --shutdown` et relance Escouade.

### Une variable du profil du shell n’arrive pas à Claude Code dans WSL

**Symptôme.** Claude Code, dans WSL, ne voit pas une variable posée dans `~/.profile` ou `~/.bashrc`.

**Cause.** Escouade le lance sans le profil du shell : ces fichiers ne sont pas lus.

**Solution.** Le proxy de Réglages › Réseau, lui, est transmis à Claude Code dans WSL. Pour une autre variable, définis-la côté Windows (dans les variables d’environnement de ton compte), ajoute son nom à la variable Windows `WSLENV` (les noms s’y séparent par `:`) et relance Escouade. Escouade garde ta valeur de `WSLENV` et y ajoute les variables qu’elle transmet elle-même.

## Réseau

### Escouade est derrière un proxy

**Symptôme.** Les quotas, les intégrations, les mises à jour ou Claude Code n’arrivent pas à se connecter depuis un réseau d’entreprise.

**Solution.** Dans Réglages › Réseau, renseigne « Proxy HTTP(S) » (par exemple `http://utilisateur:motdepasse@proxy:3128`) et, si besoin, « Exclusions » (la liste `NO_PROXY`, séparée par des virgules). Le proxy est transmis aux processus Claude Code, à la lecture des quotas, aux intégrations et aux mises à jour. Pour un agent, il s’applique au prochain (re)démarrage de son processus. Pour que les terminaux intégrés, les commandes de lancement et les commandes de préparation et de démontage des worktrees l’utilisent aussi, active « Appliquer aussi le proxy aux terminaux intégrés » (désactivé par défaut). Voir les [réglages Réseau](settings-network).

### La mise à jour est refusée par un proxy d’entreprise

**Symptôme.** Le message « Téléchargement de la mise à jour impossible » ou « Vérification des mises à jour impossible » parle d’un « 403 Forbidden », demande si c’est « bloqué par le réseau ou un proxy ? », dit que « le proxy de ton entreprise » a refusé l’installateur, ou dit que le proxy demande une authentification.

**Cause.** Un proxy d’entreprise (Zscaler, Netskope, Squid…) laisse souvent passer la recherche mais refuse l’installateur à un client qui n’est pas un navigateur. Escouade s’annonce honnêtement (`Escouade/<version>`) et ne se fait pas passer pour un navigateur ; quand le proxy refuse la requête, elle ne la refait pas sans lui. Une recherche automatique ne montre son erreur qu’après plusieurs échecs de suite, sur plus d’une heure.

**Solution.** Si le message dit que le proxy demande une authentification, mets l’identifiant et le mot de passe dans l’adresse du proxy, dans Réglages › Réseau (par exemple `http://utilisateur:motdepasse@proxy:3128`). Sinon, clique sur « Télécharger dans le navigateur » dans le message : la page de la release s’ouvre, et le proxy peut demander de confirmer un avertissement. Pour que la mise à jour intégrée passe, demande au service informatique d’autoriser `github.com` et `release-assets.githubusercontent.com`, et le téléchargement des fichiers de la release pour le client dont le User-Agent commence par `Escouade/`. Le détail est dans [« Derrière un proxy d’entreprise »](updates#derriere-un-proxy-d-entreprise).

### Un certificat d’entreprise fait échouer les connexions

**Symptôme.** Une erreur « UnknownIssuer », ou un autre échec TLS, pour les intégrations, les quotas, les mises à jour ou Claude Code.

**Cause.** Un proxy d’entreprise déchiffre le trafic avec un certificat que personne ne reconnaît. Les requêtes d’Escouade (intégrations, quotas, mises à jour) font confiance aux certificats reconnus et à ceux installés sur le système, celui d’un proxy d’entreprise le plus souvent. Claude Code, lui, a sa propre liste.

**Solution.** Installe le certificat de l’entreprise dans le système : Escouade lui fait alors confiance pour ses propres requêtes. Claude Code a sa propre liste : pour lui, ou si tu ne peux pas installer le certificat, active « Ignorer la vérification des certificats TLS » (Réglages › Réseau, groupe « Certificats »). Le réglage s’applique aux intégrations, aux quotas, aux mises à jour, aux processus Claude Code (au prochain démarrage de leur processus) et aux commandes que lancent les agents (npm, node…).

> [!WARNING]
> Avec ce réglage, une connexion interceptée, jetons compris, ne serait plus détectée. Réserve-le à un réseau de confiance.

## Quotas et limite d’usage

### Les quotas ne s’affichent pas

**Symptôme.** Dans la barre d’état, `5h` et `7j` ont une barre vide et un « — » à la place du décompte, et l’infobulle dit « Quota indisponible ». Dans Réglages › Comptes, le compte dit « Pas connecté », « Pas connecté par un compte claude.ai (clé d’API ?) » ou « Connexion expirée : relance Claude Code pour ce compte. »

**Causes possibles.**

- Les quotas sont ceux d’un abonnement claude.ai : Escouade les lit avec la connexion que Claude Code garde pour le compte. Un Claude Code utilisé avec une clé d’API n’en a pas.
- La connexion a expiré.
- Anthropic n’est pas joignable (réseau, proxy, certificat : voir plus haut). Les dernières valeurs restent affichées, et Escouade réessaie à la lecture suivante, environ toutes les cinq minutes.
- Sous macOS, Claude Code garde sa connexion dans le trousseau. Si tu refuses l’accès quand macOS le demande, Escouade ne le redemande pas avant dix minutes.

**Solution.** Dans Réglages › Comptes, « Se connecter… » (ou « Se reconnecter… ») ouvre un terminal où Claude Code te connecte (commande `/login` s’il ne le propose pas). Pour Principal, c’est la connexion de ton propre Claude Code. Voir [les comptes Claude](accounts).

### Un agent s’est arrêté sur la limite d’usage

**Symptôme.** La carte « Le tour s’est terminé en erreur » affiche le message de Claude Code sur la limite d’usage, et l’agent s’arrête.

**Cause.** La fenêtre de 5 h ou la semaine de ton abonnement est épuisée.

**Solution.** Ce que fait l’app dépend de tes réglages :

- **Reprise automatique.** Le réglage « Reprise automatique après la limite d’usage » (Réglages › Harness, activé par défaut) envoie « continue » une fois le quota réinitialisé. Quand Claude Code indique l’heure de remise à zéro, la carte dit « Reprise automatique » suivi du moment, avec un bouton « Annuler la reprise ».
- **Un autre compte.** Avec plusieurs comptes, le bouton « Reprendre sur Pro » (avec le nom d’un autre compte), et son menu « Reprendre sur un autre compte », font repartir l’agent avec sa session sur un autre compte. Les comptes inactifs, pas connectés ou au-delà de « Pause au-delà du quota » n’y figurent pas.
- **Les agents de ticket** attendent la remise à zéro de leur compte, sauf si tu actives « Reprendre sur un autre compte un agent de ticket arrêté par la limite » (Réglages › Harness, avec plusieurs comptes).

Pour ne pas arriver à la limite, le [pilote auto](autopilot) se met en pause avant, au seuil « Pause au-delà du quota ».

### Un agent s’arrête sur « OAuth session expired and could not be refreshed »

**Symptôme.** Un tour s’arrête sur « Failed to authenticate: OAuth session expired and could not be refreshed », alors que le compte est bien connecté (`/login` passe).

**Cause.** Les processus Claude Code d’un même compte partagent sa connexion. Quand le jeton expire, l’un d’eux le rafraîchit, sous un verrou du dossier du compte. Si le serveur refuse le jeton de rafraîchissement, par exemple parce qu’un autre processus l’a déjà utilisé, Claude Code le garde pour mort en mémoire. Ce processus échoue alors à chaque tour, jusqu’à 30 secondes, même si une bonne connexion est arrivée sur le disque entre-temps.

**Solution.** Escouade s’en occupe. À la fin d’un tel tour, il relit la connexion du compte :

- **Elle est bonne**, ou ce n’est plus celle avec laquelle le processus a démarré : Escouade relance le processus de l’agent sur sa session et lui envoie « continue », au plus deux fois (après 3 puis 10 secondes). La conversation le dit en une ligne : « Session du compte rafraîchie : l’agent reprend. »
- **Elle a expiré sans être rafraîchie, ou elle est absente** (Claude Code vide la connexion quand son jeton de rafraîchissement est refusé), ou les deux relances ont échoué : la carte « Connexion à refaire » nomme le compte. Son bouton « Se reconnecter » ouvre le compte, avec « Se connecter… ». Une fois connecté, l’agent reprend tout seul en moins de 20 secondes : « Compte Principal reconnecté : l’agent reprend. »

Un agent de ticket reprend de la même façon : son ticket n’est pas bloqué. Pour limiter ces refus, les lancements d’un compte dont le jeton a expiré partent l’un après l’autre : le premier rafraîchit le jeton, les autres attendent le nouveau, 20 secondes au plus.

## Agents

### Un agent s’est arrêté

**Symptôme.** L’agent passe en « Erreur ». La conversation dit « Claude Code s’est arrêté (code N) », suivi de ce que Claude Code a écrit en sortant, et un bandeau ajoute « L’agent s’est arrêté. Envoie un message pour relancer Claude sur la même session. »

**Cause.** Le processus Claude Code de l’agent s’est terminé de lui-même. Le message qu’il a laissé dit le plus souvent pourquoi (connexion, proxy, version…).

**Solution.** Envoie un message : Escouade relance Claude sur la même session. Un processus arrêté exprès n’est pas une erreur : Escouade arrête les processus inactifs après un délai (Réglages › Harness, « Arrêter les processus des agents inactifs après », 30 minutes par défaut, 0 pour jamais) et les relance à la prochaine action.

### Session Claude introuvable

**Symptôme.** Une notice dit « Session Claude introuvable : une nouvelle session sera démarrée au prochain message. »

**Cause.** Escouade a relancé l’agent sur sa session, mais Claude Code ne la connaît plus (par exemple parce que le dossier de configuration du compte a été vidé ou changé).

**Solution.** Il n’y a rien à faire : le message suivant démarre une nouvelle session. La conversation reste affichée dans Escouade, mais Claude n’en a plus le contexte.

## Worktrees

### La préparation d’un worktree échoue

**Symptôme.** La conversation de l’agent affiche « La préparation du worktree a échoué sur `npm ci` (code 1). », suivie des dernières lignes de la sortie et de « Sortie complète » avec un chemin.

**Cause.** Une commande de préparation du projet a échoué (Réglages › Projets, groupe « Worktrees », section « À l’ouverture d’un worktree »). Escouade les lance dans l’ordre et s’arrête à la première qui échoue. La raison est entre parenthèses : « code 1 », « arrêtée », « pas finie en 20 min » (une commande dispose de 20 minutes), « ne démarre pas » suivi de la cause, ou « le dossier … n’existe pas dans le worktree ».

**Solution.** Lis la sortie complète dans `~/.escouade/logs/<agent>-setup.log` : chaque étape y a son nom et sa fin. Corrige la commande dans les réglages du projet (voir [les réglages des projets](settings-projects)), ou lance-la à la main dans un terminal ouvert dans le worktree. Les commandes reçoivent `ESCOUADE_PROJECT_DIR`, `ESCOUADE_WORKTREE_DIR` et `ESCOUADE_BRANCH`. L’agent reste utilisable ; pour un agent de ticket, l’échec est ajouté à son premier message, et Claude doit faire le nécessaire pour pouvoir travailler et tester. Le log est remplacé à chaque nouvelle préparation, et disparaît avec l’agent.

À la suppression d’un agent, une commande de démontage qui échoue est signalée de la même façon : « Agent supprimé, mais le démontage du worktree a échoué sur … ». L’agent est supprimé quand même.

## Git

### Une commande git est interrompue

**Symptôme.** Un message dit « git status interrompu : toujours pas terminé après 120 s » (la commande et la durée changent), dans l’aperçu d’un commit, un merge, une synchronisation…

**Cause.** Escouade arrête une commande git qui ne finit pas, pour que la fenêtre ne reste pas bloquée : une lecture (statut, log, diff…) au bout de 2 minutes, une commande qui joint le dépôt distant (push, pull, fetch) au bout de 3 minutes (1 minute pour le fetch lancé en arrière-plan), et une commande qui lance les hooks de git (commit, merge, rebase, changement de branche) au bout de 30 minutes.

**Solution.** Lance la même commande à la main dans un terminal ouvert dans le dossier du projet pour voir ce qui la bloque (un hook, les identifiants du dépôt distant, le réseau), puis refais l’action dans Escouade.

## Notifications

### Les notifications n’apparaissent pas

**Symptôme.** Pas de notification, ou pas de son, quand un agent pose une question, termine ou tombe en erreur.

**Causes possibles.**

- Le type est désactivé. Dans Réglages › Notifications, « Me prévenir pour » liste « Questions et autorisations », « Tâches terminées », « Erreurs » et « Tickets (prêt à tester, bloqué) ». Un type désactivé n’a ni notification système ni carillon ; l’onglet, la carte et la barre des tâches (le Dock sous macOS) signalent toujours l’agent.
- La notification système n’apparaît que si la fenêtre d’Escouade n’est pas au premier plan (affichée, active et non réduite) : au premier plan, seul le carillon sonne. Elle demande aussi que l’interrupteur du groupe « Canaux » soit activé (« Notifications Windows quand l’app n’est pas au premier plan », « Notifications macOS… » sous macOS).
- Le son est coupé : le bouton « ♪ » de la barre d’état et le réglage « Son activé » le commandent, et « Tester le son » l’essaie.
- Sous Linux, Escouade n’envoie pas de notification système. Le carillon est joué par `paplay` : sans lui, pas de son.
- Vérifie aussi que le système autorise les notifications d’Escouade, et qu’un mode « ne pas déranger » ne les masque pas.

Voir les [notifications](notifications).

## Données et jetons

### Un fichier de données est illisible

**Symptôme.** Escouade démarre sans ses projets, ou avec ses réglages par défaut.

**Cause.** `state.json` ou `settings.json` (ou `integrations.json`, `sync-queue.json`) n’était pas du JSON valide. Escouade en a gardé une copie sous un nom comme `state.broken.json` et a démarré avec les valeurs par défaut pour ce fichier.

**Solution.** Ouvre la copie pour retrouver ce qu’elle contenait, par exemple la liste des projets. `app.log` dit quel fichier posait problème. Voir les [données locales](data).

### Le trousseau du système est illisible

**Symptôme.** Réglages › Intégrations dit « Trousseau du système illisible : relance Escouade ou reconnecte le compte. » ou « Trousseau du système indisponible : le jeton reste dans ~/.escouade/integrations.json. »

**Solution.** Relance Escouade ; si le message revient, reconnecte le compte (le jeton est alors enregistré de nouveau). Dans le second cas, le jeton reste dans `integrations.json`, lisible par toi seul sous macOS et Linux.

## Terminaux

### Un terminal est lent

**Symptôme.** Une lettre tapée ou un texte collé dans un [terminal](terminals) met du temps à s’afficher, alors qu’un terminal répond d’ordinaire en quelques centièmes de seconde.

**Cause.** Le temps passe à l’une des étapes du chemin d’une frappe : la fenêtre qui l’envoie au backend, le programme du terminal (le shell, Claude Code dans la vue Claude Code) qui renvoie l’écho, le retour dans la fenêtre, puis l’affichage. Un collage de quelques milliers de caractères dans PowerShell prend de lui-même une à quelques dixièmes de seconde : PSReadLine redessine la ligne à mesure qu’elle grandit, dans Windows Terminal comme dans Escouade.

**Solution.** Ouvre le journal de l’app (voir « Où est le journal de l’app », plus bas) : chaque écho qui a mis plus de 300 ms à s’afficher y est noté (au plus une fois toutes les 10 secondes par terminal), avec le temps de chaque étape, par exemple « echo of a 1-byte write shown after 837 ms: written 0 ms after the backend got it, first output 835 ms after, the echo 835 ms after, to and from the window 1 ms (answer to the write 2 ms), painted 1 ms later ». Ici, presque tout est avant « the echo » : c’est le programme qui a tardé. Un long « to and from the window » vient de la fenêtre, un long « painted » de son affichage. Le journal dit aussi, à l’ouverture de chaque terminal, son moteur de rendu : « renderer webgl » suivi du nom de la carte graphique (un nom comme SwiftShader veut dire un rendu sans carte graphique, plus lent), ou « renderer dom ». Signale-le avec ces lignes (voir « Signaler un bug »).

**Lire la ligne.** Une seule étape porte presque tout le temps :

- **« first output » et « the echo » longs** : le programme du terminal a tardé à répondre. Ouvre le même shell hors d’Escouade (dans Windows Terminal, par exemple) : s’il est lent aussi, la cause est le shell ou le poste, pas Escouade. Dans PowerShell, essaie `pwsh -NoProfile` (un module du profil peut travailler à chaque touche) et `Set-PSReadLineOption -PredictionSource None` (une prédiction fournie par un module, comme `Az.Tools.Predictor`, est calculée à chaque touche).
- **« written … after the backend got it » long** : le programme ne lisait plus ce qu’on lui tapait, occupé ailleurs.
- **« to and from the window » long** : le temps s’est perdu entre la fenêtre et le backend. Signale-le.
- **« painted » long** : l’affichage. Regarde la ligne « renderer » du même terminal.

Sur ce chemin, une frappe prend d’ordinaire 10 à 40 ms, et rarement plus de 150 ms, même avec des agents au travail, une suite de tests et un modèle local qui occupent le processeur.

**Sur un poste d’entreprise.** Ce qu’ajoute un poste géré peut ralentir chaque frappe sans charger le processeur :

- **Un EDR ou un antivirus** (Microsoft Defender for Endpoint, CrowdStrike, SentinelOne…) inspecte les processus que lance le terminal et ce qu’ils échangent avec la pseudo-console de Windows. Le temps apparaît alors dans « first output » et « the echo ». Si le même shell est lent hors d’Escouade aussi, demande au service informatique de regarder ce que l’outil fait sur `escouade.exe`, `conhost.exe` et le shell (`pwsh.exe`, `bash.exe`), ou de les exclure de l’inspection en temps réel.
- **Un bureau à distance** (RDP, Citrix, un poste virtuel) transmet l’image par le réseau : l’écho est affiché à temps dans Escouade, mais arrive tard sur ton écran. Le journal ne note alors rien, puisque rien n’a tardé dans l’app, et un terminal hors d’Escouade, dans la même session, est aussi lent. Sans carte graphique, la ligne « renderer » nomme SwiftShader ou dit « renderer dom » : l’affichage est fait par le processeur.
- **Le profil du shell** fourni par l’entreprise (modules chargés, invite qui interroge le réseau ou git) : `pwsh -NoProfile` le montre tout de suite.

Si tu vois le retard et que le journal ne note rien, le temps n’est pas passé dans Escouade : il est dans l’affichage du poste (un bureau à distance), ou l’écho a mis plus de 10 secondes (une frappe restée sans réponse est alors oubliée). Seule une fenêtre visible mesure ses frappes.

## Voix

### La dictée ou la conversation n’entend rien

**Symptôme.** La dictée ou la [conversation vocale](chat#parler-avec-l-agent-conversation-vocale) s’arrête sur un message au lieu d’écouter, ou une dictée finit sur « Rien entendu : parle un peu plus près du micro, puis réessaie. »

**Cause et solution.** Le message dit laquelle :

- « Aucun micro trouvé. Branche un micro, ou choisis-en un dans Réglages › Voix. » : aucun micro n’est branché, ou celui choisi dans [Réglages › Voix](settings-voice#micro) a disparu avec celui du système.
- « Le système refuse le micro à Escouade. » ou « Aucun son n’est arrivé du micro : il est coupé, ou le système le refuse à Escouade. » : le message donne la marche à suivre de ton système.
  - **macOS** demande la permission à la première dictée, et ne refuse rien ensuite : une app privée de micro reçoit du silence. Ouvre Réglages Système › Confidentialité et sécurité › Microphone, coche Escouade, puis relance l’app.
  - **Windows** : Paramètres › Confidentialité et sécurité › Microphone, allume « Accès au microphone » puis « Autoriser les applications de bureau à accéder à votre microphone ».
  - **Linux** : vérifie que PulseAudio ou PipeWire tourne, que le micro n’est pas coupé (pavucontrol, ou les réglages du son), et que ton compte peut s’en servir (groupe `audio`).
- « Le micro est pris par une autre application. » : ferme-la, ou choisis un autre micro dans Réglages › Voix.
- « Aucun système de son ne répond. » (Linux) : démarre PulseAudio ou PipeWire (ou vérifie ALSA), puis réessaie.
- « Rien entendu » : le micro marche, mais Whisper n’a pas trouvé de mots. Parle plus près du micro, vérifie dans Réglages › Voix que « Micro utilisé » est le bon, et que « Langue parlée » est la tienne.

### La dictée en ligne échoue

**Symptôme.** Avec « Transcrite par » sur « En ligne », la dictée finit sur un message et rien n’est écrit.

**Solution.** Le message nomme le service :

- « Aucune clé OpenAI n’est enregistrée. » : colle ta clé dans [Réglages › Voix](settings-voice#la-dictee-en-ligne), ou repasse en « Local (Whisper) ». Sous Linux, la clé ne peut pas être enregistrée : reste en local.
- « OpenAI refuse la clé. » : vérifie-la chez le service, puis « Remplacer ».
- « OpenAI ne répond pas » : vérifie le réseau et le proxy ([Réglages › Réseau](settings-network)), ou repasse en local.
- « OpenAI n’a pas transcrit la dictée », avec le code et la raison que donne le service : règle ce qu’elle dit sur ton compte chez lui, ou repasse en local.

### La conversation vocale ne démarre pas

**Symptôme.** Un clic sur « Conversation » affiche « La conversation vocale n’est pas disponible : le moteur de la voix (ONNX Runtime) est introuvable dans l’app. Réinstalle Escouade. », ou le téléchargement d’un modèle ou d’une voix échoue.

**Solution.** ONNX Runtime est livré avec l’app : s’il manque, réinstalle Escouade depuis [la page de téléchargement](installation). Un téléchargement qui échoue passe par le proxy des réglages réseau : derrière un proxy d’entreprise, vérifie [Réglages › Réseau](settings-network) et que `huggingface.co` est joignable. Ce qui se télécharge, et où, est dans [Réglages › Voix](settings-voice#ce-qui-se-telecharge-et-ou).

## Journal et signalement

### Où est le journal de l’app

Le journal est `~/.escouade/app.log` (`C:\Users\<toi>\.escouade\app.log` sous Windows). Chaque ligne porte la date, le niveau et le texte : démarrage et version, arrêts d’agents, erreurs, migrations, mises à jour. Quand il dépasse environ 5 Mo, le démarrage suivant le range sous le nom `app.old.log`. Tout éditeur de texte l’ouvre.

### Poser une question

Pour une question, une idée ou un coup de main, rejoins le [Discord d’Escouade](https://discord.gg/m3KYtN8gxP).

### Signaler un bug

Ouvre une issue sur [GitHub](https://github.com/guillaume-gagnaire/escouade/issues) avec :

- la version d’Escouade (Réglages › À propos, « Escouade 1.7.5 » par exemple) et ton système (Windows, macOS ou Linux, et sa version) ;
- ce que tu faisais, ce que tu attendais, ce qui s’est passé, et une capture si elle aide ;
- les lignes utiles de `app.log`. Relis-les avant de les joindre : elles peuvent citer des chemins de tes dossiers et des noms de projets. Ne colle jamais un jeton.
