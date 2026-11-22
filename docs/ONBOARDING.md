# L'onboarding d'Escouade (2.3)

Conception du premier lancement et de la découverte des fonctionnalités : ESC-503 (ON-0), 2026-11-17. Ce document ne change rien au produit. Il fait foi pour ON-1 à ON-4 (ESC-504 à ESC-507), avec la direction de design que choisira l'utilisateur.

- **La demande** (2026-11-16) : « la phase d'onboarding sur Escouade : poser les questions de harness, etc, très didactique, pour setup rapidement Escouade et bien découvrir les différentes fonctionnalités (voix, etc). On fera le design et l'ux avec impeccable. »
- **Les maquettes** : `design/onboarding-2-3/`, trois directions (A « Essaie d'abord », B « Au bon endroit », C « La ligne »), leurs 60 captures et le `README.md` qui les compare.
- **La méthode** : le skill `impeccable` (cadrage `shape`, premier lancement `onboard`, tirage des structures `concept-seed`, plancher de qualité, vérification des rendus). Le monde visuel est celui de la 2.0 (`DESIGN.md`, `src/app.css`) : rien de neuf n'y est inventé, sauf les couleurs de voies du thème clair, à foncer si C est choisie (choix O14).

## 1. Ce qui existe déjà

Lu dans le code de `release/2.3` et dans la spec d'Escouade. Le parcours **réutilise** tout cela : il ne crée pas de second chemin vers un réglage.

| Pièce | Où | Ce qu'elle fait aujourd'hui | Dans le parcours |
| --- | --- | --- | --- |
| L'écran sans projet | `src/components/Welcome.svelte` | Trois états. Aucun harness : chaque harness connu avec sa commande d'installation à copier, « Chercher à nouveau » et « Réglages › Comptes et harness… ». Un harness sans compte (ESC-128) : chaque harness trouvé, son compte de la machine (connecté, sous quel e-mail) et « Ajouter un compte… ». Sinon : « Un premier projet », « Ajouter un projet… ». | Remplacé par la station ou l'écran du harness, puis du projet. Sa logique se reprend telle quelle : `app.enginesFound`, `api.accountMachine`, `api.claudeFound`, le harness « en tête ». |
| La question du mode | `FirstModeModal.svelte`, `src/lib/first-mode.ts` (SEC-09) | Première question du premier lancement : le mode des nouveaux agents parmi quatre, **Édits auto** recommandé (`RECOMMENDED_MODE = 'acceptEdits'`). Bypass n'y est jamais proposé : il attend « Autoriser le mode Bypass » dans Réglages › Agents. Fermée sans réponse, elle revient au lancement suivant, le mode recommandé en vigueur entre-temps. | La station « Réglages de départ », avec les mêmes textes (`settings.permissions.first.*`). Bypass y est **montré éteint**, avec pourquoi. |
| L'accord des statistiques | `TelemetryModal.svelte` | Ce qui part, vers où (Aptabase, en France, 25 mois), ce qui ne part jamais. Aucune des deux réponses n'est mise en avant. Fermée sans réponse : redemandée au lancement suivant. | La station « Statistiques », mêmes textes, même réglage (`telemetry`). |
| Le nom pour l'équipe | `TeamNameModal.svelte` (ESC-190) | Le nom de l'utilisateur, proposé d'après `git config user.name`. « Plus tard » le repousse au premier partage. | Sorti du premier lancement (choix O8) : demandé au premier partage, comme son « Plus tard » le fait déjà. |
| L'ordre des questions | `src/lib/state.svelte.ts` (le démarrage) | Le mode, puis les statistiques, puis le nom, en fenêtres l'une après l'autre. | Remplacé par le parcours. |
| Ajouter un compte | `AccountModal.svelte`, `addAccount()` de `src/lib/accounts.ts` | « Le compte de la machine » (son dossier habituel, tel quel) ou « Un nouveau compte » (un dossier à lui sous `~/.escouade`, sa propre connexion). Choisir un harness installe son plugin depuis l'amorce, sans réseau (ESC-373). | La station du harness et du compte appelle ces mêmes fonctions. Elle dit quel plugin s'installe. |
| L'amorce des plugins | spec « Les plugins d'Escouade (la première main) », ESC-373 | La 2.2 n'installe que les plugins de ce qui est configuré. Les autres restent proposés, jamais remis d'eux-mêmes. | Le parcours n'installe que le plugin du harness choisi, et le dit. |
| Le nouveau projet | `NewProjectModal.svelte`, `src/lib/wizard*.ts` | L'assistant par étapes : dossier, nom et couleur, d'où partent ses réglages, worktrees, lancement, Kanban, compte, intégrations, résumé. « Passer » sur les étapes facultatives. Clonage d'un projet de l'équipe (TS-8). | La station du premier projet en est la version courte : un dossier, un clone ou la démonstration. Les autres étapes gardent leurs valeurs par défaut et restent dans l'assistant habituel. |
| La carte de permission | `composer.permission.*` | « Autoriser », « Toujours autoriser », « Refuser », au clavier aussi. | Montrée et expliquée là où elle arrive, à la première demande. |
| La voix | `crates/escouade-voice-local`, Réglages › Voix | Dictée et conversation. Whisper en local (modèle « base », 148 Mo, téléchargé une fois) ou un service en ligne avec sa clé, éteint par défaut. | Une découverte, avec le poids dit avant le téléchargement. |
| Les valeurs par défaut | `src-tauri/src/accounts.rs` | Claude Code : modèle `sonnet`, effort `medium`. | Dites, pas demandées : ce sont celles du compte. |
| Les réglages imposés | spec « Réglages imposés » (2.3) | Les clés qui touchent le parcours : `permissions.allowBypass`, `permissions.modes`, `permissions.defaultMode`, `harnesses.allowed`, `harnesses.allowedModels`, `harnesses.maxEffort`, `telemetry.enabled`, `voice.onlineAllowed`, `voice.dictation`, `servers.remoteApi`, `mcp.externalClients`, `plugins.*`, `team.server`, `team.fingerprint`. Un réglage imposé est grisé, avec un cadenas et qui l'impose. C'est le cœur qui refuse. | Une étape imposée se montre comme telle, une étape sans objet disparaît (§ 5.6). |
| Les statistiques d'usage | `src-tauri/src/telemetry.rs` | 30 événements (`app_started`, `project_created`, `agent_created`, `message_sent`, `ticket_created`, `dictation_used`…), pseudonymes, sans contenu. | Les événements du parcours s'y ajoutent (§ 9). |
| L'aide | — | **Il n'y a pas d'entrée « Aide »** aujourd'hui. Le rail porte la vue d'ensemble, les projets, les stats, les plugins et les réglages. | Une entrée Aide à ajouter au rail (choix O10) : c'est là que se retrouvent le parcours et la découverte. |
| La fenêtre | spec « Fenêtre » | Taille minimale 1000 × 600, thème Système, Clair ou Sombre, zoom de 80 à 200 %. | Le parcours tient à 1024 × 728 et à 1440 × 900, dans les deux thèmes. |

Ce que la 2.3 ajoute et que la découverte doit connaître : la vue VS Code dans l'éditeur, les PR dans Escouade (fils de relecture d'un agent), le serveur d'équipe (entrer par un lien, projets de l'équipe, réglages imposés), Escouade à distance (l'API distante, le téléphone, le mode serveur).

## 2. Pour qui, et ce que chacun doit avoir fait en deux minutes

Le moment qui prouve qu'Escouade vaut le temps qu'on lui donne (le « aha » d'impeccable) : **un agent qui répond, dans un projet, avec un mode compris**. Tout le reste attend la découverte.

| Profil | Ce qu'Escouade trouve | En moins de 2 minutes | Le chemin |
| --- | --- | --- | --- |
| **P1. A déjà un harness connecté** (Claude Code, Codex…) | Un harness prêt, connecté sous un e-mail | Le compte de la machine ajouté, le mode choisi, un projet ouvert, un premier agent qui a répondu | Toutes les stations, chacune en un `Entrée` sur le choix recommandé. **Objectif mesuré par ON-4 : moins de 2 minutes.** |
| **P2. A un harness installé, pas connecté** | Le programme, sans connexion | Le harness choisi, sa connexion lancée dans un terminal de l'app, puis comme P1 | La station du harness dit « Tu t'y connecteras dans un terminal, juste après ». La connexion se fait par l'écran de compte qui existe déjà. |
| **P3. N'a rien d'installé** | Aucun harness | La commande copiée en un clic, la page du harness ouverte, l'app qui le trouve d'elle-même | Le temps d'installation n'est pas à nous : l'objectif est **la commande copiée en 15 secondes**. Puis Escouade cherche à nouveau dès que la fenêtre revient au premier plan, et la station passe d'elle-même. |
| **P4. Vient d'une équipe ou d'un poste géré** | Des stratégies du poste, ou un lien d'invitation `escouade://team/join` | Ce qui est imposé vu et compris, l'équipe rejointe, ses projets proposés au clonage | Les stations imposées se montrent verrouillées, avec qui les impose. La station du projet propose d'abord « Les projets de ton équipe ». |
| **P5. Revient après une mise à jour majeure** | Des projets, des comptes : un poste configuré | Rien d'imposé. Une bande discrète « Nouveau dans la 2.3 » | Jamais le parcours de lui-même. La bande ouvre la découverte filtrée sur ce qui est nouveau. |
| **P6. Refait le parcours** | Un poste configuré | Revoir une étape, ou changer un réglage de départ | Aide › Premier lancement : chaque station montre la valeur en place. Changer, c'est écrire le même réglage qu'ailleurs. |

## 3. Les principes du parcours

1. **Une question à la fois**, avec le choix recommandé déjà sélectionné et **une phrase qui dit pourquoi**.
2. **Tout se passe.** « Plus tard » sur chaque étape, « Tout passer » pour le parcours entier. Une étape passée reste « à reprendre » ; elle ne bloque rien.
3. **Le parcours lit l'état réel** de l'app, il ne tient pas sa propre vérité. Un compte ajouté dans les réglages coche la station du compte. Un harness installé pendant que la fenêtre attendait fait passer la station du harness.
4. **Rien n'est installé, connecté, téléchargé ni envoyé sans un geste**, et ce que le geste coûte est dit avant : le plugin installé, les 148 Mo du modèle, le quota que consomme un message.
5. **Le geste réel, jamais une diapositive.** Chaque explication est posée à côté de la vraie chose : la vraie carte de permission, le vrai micro, le vrai Kanban.
6. **Hors ligne, le parcours marche.** Le clone, la voix en ligne et la connexion d'un hébergeur disent qu'il leur faut le réseau ; le reste avance.
7. **Clavier, lecteur d'écran, deux tailles, deux thèmes, deux langues, « réduire les animations »** (§ 8).
8. **Le ton** : tutoiement, phrases courtes, pas de jargon sans sa définition en une ligne (harness, worktree, branche).

## 4. Le parcours, écran par écran

Les écrans sont les mêmes dans les trois directions ; la forme change (§ 6). Les textes ci-dessous sont ceux des maquettes. Les clés i18n à créer vivent sous `onboarding.*`. Les textes qui existent déjà sont repris tels quels : leur clé est dite.

### 4.0 L'en-tête du parcours

| | Français | English |
| --- | --- | --- |
| Titre | Premier lancement | First launch |
| Sous-titre | 6 étapes · environ 2 minutes | 6 steps · about 2 minutes |
| Passer tout | Tout passer | Skip all |
| Une étape | Plus tard | Later |
| Valider | Continuer | Continue |
| Retour | Retour | Back |
| Aide clavier | ↑ ↓ choisir · Entrée valider | ↑ ↓ choose · Enter confirm |
| Reprendre | Tout se passe et se reprend. Le parcours revient depuis Aide › Premier lancement. | You can skip anything and come back. Find this again in Help › First launch. |
| Étape passée | passé · à reprendre | skipped · to resume |

### 4.1 Le harness

**Ce qu'elle règle** : le programme qui fera tourner les agents. **Une question** : lequel. **Recommandé** : le premier harness connecté sur la machine, sinon le premier trouvé, sinon aucun (on installe).

| | Français | English |
| --- | --- | --- |
| Question | Avec quel harness tes agents travaillent-ils ? | Which harness will your agents run on? |
| Pourquoi | Un harness est le programme d'agent de code de ta machine. Escouade le lance et le pilote ; il garde ta connexion et tes réglages. | A harness is the coding agent program on your machine. Escouade starts and drives it; it keeps your sign-in and your settings. |
| État prêt | Connecté · {email} | Signed in · {email} |
| État installé | Installé, pas connecté | Installed, not signed in |
| Aide de l'état installé | Tu t'y connecteras dans un terminal, juste après. | You'll sign in from a terminal, right after. |
| État absent | Introuvable | Not found |
| Commande | (la commande du manifeste du plugin, `install.windows` / `install.unix`) + « Copier » | (the command from the plugin's manifest) + “Copy” |
| Copiée | Commande copiée | Command copied |
| Trop ancien | Trop ancienne : {version}, il faut {min} (reprend `settings.claude.states.tooOld`) | Too old: {version}, needs {min} |
| Chercher | Chercher à nouveau · Cherché dans le PATH (et dans WSL ({distro})) | Search again · Searched the PATH (and WSL ({distro})) |
| Aucun trouvé | Aucun harness n'est installé. Copie la commande de celui que tu veux, lance-la dans un terminal : Escouade le trouvera dès que tu reviens. | No harness is installed. Copy the command for the one you want and run it in a terminal: Escouade will find it when you come back. |
| Plugin | Escouade installe le plugin de {harness}, depuis l'app, sans réseau. | Escouade installs the {harness} plugin, from the app, offline. |
| Interdit | {harness} · non permis par la stratégie de ce poste | {harness} · not allowed by this device's policy |
| Commande d'un tiers | Commande donnée par le plugin {plugin}, pas par Escouade (reprend `plugins.harness.commandFrom`) | Command given by the {plugin} plugin, not by Escouade |

**États** : plusieurs harness prêts (le premier de « Ordre des harness » recommandé) ; un seul (il est choisi, la question devient une confirmation) ; aucun (les commandes, et la station attend en surveillant) ; un harness imposé par le poste (la station est faite, verrouillée : « Imposé par la stratégie de ce poste ») ; un harness interdit (grisé, jamais recommandé).

### 4.2 Le compte

**Ce qu'elle règle** : sur quel compte partent les agents. **Recommandé** : le compte de la machine quand le harness y est connecté. Dans ce cas, la station se fond dans la précédente (choix O6) : « Ses agents partent sur le compte déjà connecté ; un compte à part se crée dans Réglages. »

| | Français | English |
| --- | --- | --- |
| Question | Quel compte pour {harness} ? | Which account for {harness}? |
| Pourquoi | Les quotas, les statistiques et la connexion sont tenus par compte. Tu pourras en ajouter d'autres (perso, pro) : Escouade passe au suivant quand l'un atteint sa limite. | Quotas, statistics and sign-in are kept per account. You can add more later (personal, work): Escouade moves to the next one when one hits its limit. |
| Choix 1 | Le compte de la machine · {email} — Son dossier habituel, tel quel : ses réglages, sa connexion et ses conversations. (reprend `accounts.add.machine`, `machineHint`) | This machine's account · {email} — Its usual folder, as is: its settings, sign-in and conversations. |
| Choix 2 | Un compte à part — Un dossier à lui sous ~/.escouade, et sa propre connexion. (reprend `newHint`) | A separate account — Its own folder under ~/.escouade, and its own sign-in. |
| Pas connecté | {harness} n'est pas encore connecté sur cette machine : tu t'y connectes juste après, dans un terminal. (reprend `machineSignInLater`) | {harness} isn't signed in on this machine yet: you'll sign in right after, in a terminal. |

### 4.3 Les réglages de départ (les modes expliqués)

**Ce qu'elle règle** : le mode des nouveaux agents. Le modèle et l'effort sont **dits, pas demandés** : ce sont ceux du compte (Sonnet, moyen pour Claude Code), changés sous chaque message. **Recommandé** : Édits auto.

| | Français | English |
| --- | --- | --- |
| Question | Quand tes agents doivent-ils te demander ? | When should your agents ask you? |
| Pourquoi | Le mode des nouveaux agents ; tu le changes pour chacun, sous son message. Modèle {model}, effort {effort} : ceux de ton compte. | The mode for new agents; you can change it for each one, under its message. Model {model}, effort {effort}: your account's. |
| Édits auto (recommandé) | Les modifications dans le dossier de l'agent passent sans demander (elles restent dans son worktree, visibles dans son diff) ; chaque commande demande ton accord. Le plus prudent qui laisse un ticket avancer. (`settings.permissions.first.acceptEdits`) | Edits inside the agent's folder go through without asking (they stay in its worktree, visible in its diff); every command asks for your approval. The safest mode that still lets a ticket move. |
| Demander | Chaque action sensible demande ton accord, modifications comprises : un ticket s'arrête à chaque fichier. (`first.default`) | Every sensitive action asks for your approval, edits included: a ticket stops at every file. |
| Plan | L'agent analyse et propose un plan, sans rien modifier ni lancer, jusqu'à ce que tu l'approuves. (`first.plan`) | The agent analyses and proposes a plan, without changing or running anything, until you approve it. |
| Auto | Un classifieur approuve ce qu'il juge sûr, commandes comprises, et ne demande que pour le reste. (`first.auto`) | A classifier approves what it judges safe, commands included, and only asks about the rest. |
| Bypass (éteint) | Éteint : l'agent ferait tout sans rien te montrer, même ce qu'un contenu piégé lui dicterait. À réserver à une machine jetable ; il s'allume dans Réglages › Agents. | Off: the agent would do everything without showing you, even what a booby-trapped file told it to. Keep it for a throwaway machine; turn it on in Settings › Agents. |
| Mode manquant | {harness} n'a pas de mode {mode}. (`composer.mode.missing`) | {harness} has no {mode} mode. |
| Harness en Bypass seul | {harness} ne tourne qu'en Bypass : ses agents agiront sans demander. (ESC-489) | {harness} only runs in Bypass: its agents will act without asking. |
| Imposé | Imposé par la stratégie de ce poste / par l'équipe {team} | Set by this device's policy / by the {team} team |
| Bouton | Garder {mode} | Keep {mode} |

Version courte des modes, pour une colonne étroite (direction A) : « Les modifications passent ; chaque commande demande. » / « Tout ce qui est sensible demande, modifications comprises. » / « Il propose un plan et ne touche à rien avant ton accord. » / « Un classifieur approuve ce qui est sûr, demande le reste. » / « Éteint : aucune demande, même pour un contenu piégé. » — EN : “Edits go through; every command asks.” / “Anything sensitive asks, edits included.” / “It proposes a plan and touches nothing until you approve.” / “A classifier approves what's safe, asks about the rest.” / “Off: no asking at all, even for a booby-trapped file.”

### 4.4 Le premier projet

**Ce qu'elle règle** : où travaille le premier agent. **Recommandé** : un dossier de la machine (choix O4). Les quatre mots du produit y sont définis, avec un schéma de branches dans C.

| | Français | English |
| --- | --- | --- |
| Question | Où travaille ton premier agent ? | Where will your first agent work? |
| Pourquoi | Un projet est un dossier, avec ses agents, son Kanban et ses docs. Chaque agent y travaille dans son worktree, sur sa branche : deux agents ne se gênent pas, et rien n'entre dans ta branche sans toi. | A project is a folder, with its agents, its board and its docs. Each agent works in its own worktree, on its own branch: two agents never get in each other's way, and nothing lands on your branch without you. |
| Dossier | Un dossier de ta machine — Un dépôt que tu as déjà. Escouade le lit sans rien y changer. · Parcourir… | A folder on your machine — A repository you already have. Escouade reads it without changing anything. · Browse… |
| Clone | Cloner un dépôt — Une adresse git, ou un dépôt de GitHub ou GitLab si tu en as connecté un. | Clone a repository — A git URL, or a GitHub or GitLab repository if you've connected one. |
| Démonstration | Un projet de démonstration — Un petit dépôt jetable, créé sur ta machine sans réseau, pour essayer sans risque. Supprimé d'un clic. | A demo project — A small throwaway repository, created on your machine without network, to try things safely. Deleted in one click. |
| Équipe (P4) | Les projets de ton équipe — {team} en partage {n} : clone celui sur lequel tu travailles. | Your team's projects — {team} shares {n}: clone the one you work on. |
| Projet | Un projet : un dossier, avec ses agents, son Kanban et ses docs. | A project: a folder, with its agents, its board and its docs. |
| Agent | Un agent : une conversation avec ton harness, qui lit et modifie le code. | An agent: a conversation with your harness, which reads and changes code. |
| Worktree | Un worktree : une copie de travail à l'agent seul : deux agents ne se gênent pas. | A worktree: a working copy for that agent alone, so two agents never collide. |
| Branche | Une branche par agent : son travail reste à part jusqu'à ce que tu l'intègres. | A branch per agent: its work stays apart until you merge it. |
| Bouton | Ouvrir le projet | Open project |
| Sans git | Ce dossier n'est pas un dépôt git : Escouade peut en faire un, avec un premier commit. (le chemin de l'assistant actuel) | This folder isn't a git repository: Escouade can make it one, with a first commit. |
| `.escouade.yml` | Ce dépôt a un .escouade.yml : il règle {liste}. Ses commandes se relisent avant de tourner. | This repository has an .escouade.yml: it sets {list}. Its commands are reviewed before they run. |
| Clone hors ligne | Cloner demande le réseau. Choisis un dossier ou la démonstration, ou réessaie une fois en ligne. | Cloning needs the network. Pick a folder or the demo, or try again once you're online. |
| Clone en échec | Le clonage a échoué : {error}. Vérifie l'adresse et ton accès, puis réessaie. | Cloning failed: {error}. Check the URL and your access, then try again. |
| Sans droit d'écriture | Escouade ne peut pas écrire dans ce dossier ({path}) : choisis-en un autre. | Escouade can't write to this folder ({path}): pick another one. |

**Le projet de démonstration** (choix O3) : `~/.escouade/demo/bac-a-sable`, un petit dépôt git local créé par l'app (une liste de tâches en TypeScript, deux tests qui échouent exprès, sans dépendance à installer). Une bande le dit en haut du projet : « Bac à sable · un dépôt de démonstration sur ta machine · rien ne sort d'ici · Supprimer ». EN : “Sandbox · a demo repository on your machine · nothing leaves it · Delete”.

### 4.5 Le premier agent

**Ce qu'elle règle** : rien ; elle fait vivre le premier échange. L'agent est créé avec les réglages des stations précédentes. **Rien ne part sans le clic.**

| | Français | English |
| --- | --- | --- |
| Titre | Ton premier message | Your first message |
| Pourquoi | Choisis-en un, modifie-le, puis envoie-le. Il part sur ton compte {harness} et compte dans son quota. | Pick one, edit it, then send it. It goes out on your {harness} account and counts toward its quota. |
| Amorce 1 | Explique-moi ce dépôt en cinq lignes. | Explain this repository in five lines. |
| Amorce 2 | Lance les tests et dis-moi ce qui casse. | Run the tests and tell me what breaks. |
| Amorce 3 | Trouve un petit bug et propose un correctif. | Find a small bug and suggest a fix. |
| Vide | {agent} est prêt. Un message proposé t'attend : modifie-le ou choisis-en un autre. Rien ne part avant que tu l'envoies. | {agent} is ready. A suggested message is waiting: edit it or pick another. Nothing is sent until you send it. |
| Permission, titre | Ton agent te demande la permission | Your agent is asking for permission |
| Permission, pourquoi | Il veut lancer une commande. En {mode}, chaque commande te demande : **Autoriser** cette fois, **Toujours** pour cette commande dans ce projet, ou **Refuser** pour qu'il cherche autrement. | It wants to run a command. In {mode}, every command asks you: **Allow** this once, **Always** for this command in this project, or **Deny** so it finds another way. |
| Clavier | Au clavier : 1, 2 ou 3. Ctrl J t'amène au prochain agent qui attend. | From the keyboard: 1, 2 or 3. Ctrl J takes you to the next agent that's waiting. |
| Plus tard | Plus tard, une demande comme celle-ci fait clignoter la carte de l'agent. | Later on, a request like this one makes the agent's card blink. |
| Fini | L'agent a fini, dans son worktree, sur sa branche : ton dépôt n'a pas bougé. Voir le diff. | The agent is done, in its worktree, on its branch: your repository hasn't changed. See the diff. |
| Arrêter | Pour l'arrêter : Échap, ou ■ sous le message. | To stop it: Esc, or ■ under the message. |
| Limite d'usage | Ce compte a atteint sa limite d'usage jusqu'à {time}. Ajoute un autre compte, ou reviens après. | This account has hit its usage limit until {time}. Add another account, or come back later. |
| Hors ligne | {harness} n'a pas pu joindre son service : tu sembles hors ligne. Le message reste prêt. | {harness} couldn't reach its service: you seem to be offline. Your message stays ready. |
| Pas connecté | {harness} n'est pas connecté : reviens à l'étape du compte. | {harness} isn't signed in: go back to the account step. |

Les amorces dépendent du projet. « Lance les tests » n'est proposée que si le projet a une commande de test reconnue (`package.json`, `Cargo.toml`, `pyproject.toml`…), et seulement sur la démonstration si Node est sur la machine.

### 4.5 bis Les fonctionnalités (F-2, ESC-559)

Ajoutée après ON-4, par l'assistant des fonctionnalités (`docs/FEATURES.md` § 1) : **une station de plus, la 7ᵉ des 8, entre le premier agent et les statistiques**. Le premier agent reste le « aha » et n'arrive pas plus tard ; l'utilisateur, qui vient de voir l'app au travail, choisit ce qu'elle montre autour des agents en le sachant ; les statistiques restent la dernière question (O7).

**Ce qu'elle règle** : les interrupteurs de Réglages › Fonctionnalités, les mêmes. **Recommandé** : rien, une fonctionnalité par écran, chacune allumée par le geste ; une installation neuve les a coupées (`NEW_INSTALL`), sauf ce que « Ce que tu veux brancher » a allumé (une source de tickets : le Kanban ; la voix : la Voix).

| | Français | English |
| --- | --- | --- |
| Question | Que veux-tu voir autour de tes agents ? | What do you want to see around your agents? |
| Pourquoi | Une fonctionnalité par écran : allume celles qui te servent. L'essentiel est toujours là : les agents, le panneau Git, les terminaux et les lancements. | One feature per screen: turn on the ones you need. The essentials are always there: the agents, the Git panel, the terminals and the launches. |
| Un écran | Le Kanban · une phrase · trois points · « Activer le Kanban » | The Kanban · a sentence · three points · “Turn on the Kanban” |
| Utilisée | Tu l'utilises déjà sur 2 projets : il reste allumé. | You already use it on 2 projects: it stays on. |
| Plus tard | Passer : garder l'essentiel | Skip: keep the essentials |
| Sous la station | 2 allumées · l'essentiel seul | 2 on · the essentials only |
| Imposée | Toutes les fonctionnalités sont imposées sur ce poste : elles se montrent ici, sans se changer. | Every feature is imposed on this device: they are shown here, and cannot be changed. |

**Au clavier** : depuis la question, ↓ va à l'interrupteur, Espace le bascule, Entrée fait « Suivant » (« Continuer » au dernier écran), ← → changent d'écran, Échap passe. « Retour » revient d'un écran avant de revenir à la station d'avant. **Faite** par le geste seul (« Continuer » au dernier écran, ou l'assistant fini ailleurs) : ses interrupteurs ont toujours une valeur, l'état réel ne dit pas qu'ils ont été choisis. L'entonnoir la compte (`onboarding_step`, `step: features`).

### 4.6 Les statistiques

**Ce qu'elle règle** : l'accord des statistiques, la même question qu'aujourd'hui (`shell.telemetry.*`), à la fin du parcours (choix O7). Aucune réponse n'est mise en avant. **Ce qui reste sur le poste** est dit en une ligne.

| | Français | English |
| --- | --- | --- |
| Question | Aider à améliorer Escouade ? (`shell.telemetry.title`) | Help improve Escouade? |
| Pourquoi | `shell.telemetry.body` et `detail`, tels quels | as today |
| Ce qui reste | Tes projets, ton code, tes conversations et tes comptes restent sur ta machine, dans ~/.escouade. | Your projects, code, conversations and accounts stay on your machine, in ~/.escouade. |
| Boutons | Non merci · Partager les statistiques (`no`, `yes`) | No thanks · Share statistics |
| Imposé | Éteintes par la stratégie de ce poste. (la station est faite et verrouillée) | Turned off by this device's policy. |

### 4.7 La fin

| | Français | English |
| --- | --- | --- |
| Titre | Escouade est prêt | Escouade is ready |
| Texte | Ton agent travaille. Le reste se découvre en le faisant : la voix, le Kanban, les PR… Aide › À découvrir. | Your agent is at work. Discover the rest by doing it: voice, the board, PRs… Help › Discover. |
| Passé | {n} étapes passées : elles t'attendent dans Aide › Premier lancement. | {n} steps skipped: they're waiting in Help › First launch. |

## 5. Les états du parcours

L'état vit dans l'**état durable** de l'app (ESC-469), par installation, jamais dans `.escouade.yml`. Il est lu au démarrage et après chaque changement de réglage.

```
onboarding: {
  version: 1,                       // la version du parcours (une 2.4 qui change une étape la monte)
  started: "2026-11-17T09:12:00Z",
  finished: null | date,            // dernière station validée, ou « Tout passer »
  steps: { harness, account, defaults, project, agent, telemetry: "done" | "skipped" | "policy" },
  discover: { <id>: "done" | "dismissed" }, discoverHidden: false,
  seenRelease: "2.3"                // la dernière version majeure dont la bande « Nouveau » a été vue ou fermée
}
```

Une station est **faite** quand l'état réel le dit, pas quand on a cliqué :

- *harness* : `enginesFound` non vide et un harness choisi ;
- *account* : un compte ajouté ;
- *defaults* : `agentMode` non vide (comme `modeToAsk` aujourd'hui) ;
- *project* : un projet ouvert ;
- *agent* : un premier agent qui a fini un tour ;
- *telemetry* : `telemetry` non nul.

### 5.1 Premier lancement

Un dossier de données vide (aucun compte, aucun projet) ouvre le parcours à la première station qui n'est pas faite. Un harness déjà connecté et un compte déjà là (une migration) sautent leurs stations.

### 5.2 Repris

Fermée au milieu (la fenêtre, l'app, une coupure), l'app rouvre le parcours **à la première station ni faite ni passée**. Les stations faites restent faites, même si l'état a changé depuis : un compte retiré ne rouvre pas le parcours.

### 5.3 Passé

« Plus tard » marque la station *skipped* et passe à la suivante. « Tout passer » marque toutes les restantes. Une station passée :

- ne revient pas d'elle-même ;
- reste « à reprendre » dans Aide › Premier lancement ;
- garde, pour les réglages qu'elle portait, la valeur sûre : le mode recommandé (comme aujourd'hui), les statistiques **non envoyées** tant qu'il n'y a pas de réponse.

Les statistiques passées sont redemandées une fois au lancement suivant, comme aujourd'hui, puis plus jamais.

### 5.4 Refait

Aide › Premier lancement, et Réglages › Général › « Refaire le premier lancement », rouvrent le parcours **sur un poste configuré**. Chaque station montre sa valeur en place (« Claude Code · toi@exemple.dev », « Édits auto »). Changer y écrit exactement ce qu'écrivent les réglages. Rien n'est remis à zéro : refaire ne supprime ni compte ni projet.

### 5.5 Après une mise à jour majeure

Un poste configuré qui passe à une version majeure (`seenRelease` plus ancien que la version) ne voit **jamais** le parcours. Il voit une bande fine en haut de la place de travail : « Nouveau dans la 2.3 : les PR dans Escouade, l'éditeur VS Code, ton équipe, Escouade à distance. Voir » / “New in 2.3: PRs in Escouade, the VS Code editor, your team, Escouade remote. See”, et ×. « Voir » ouvre la découverte filtrée sur ce qui est nouveau. × la ferme pour cette version. Une version mineure n'affiche rien.

### 5.6 Poste géré, équipe

- **Imposé** : la station est faite, verrouillée, avec qui l'impose (« Imposé par la stratégie de ce poste », « Imposé par l'équipe Acme »). Elle se traverse en un `Entrée` et sert à expliquer, pas à demander.
- **Borné** : les choix interdits sont grisés avec leur raison (« Auto · non permis par la stratégie de ce poste ») et ne sont jamais recommandés.
- **Sans objet** : une étape qu'une stratégie rend vide disparaît. Statistiques éteintes : pas de question. Un seul harness permis et prêt : la station est une confirmation.
- **Une invitation d'équipe** (`escouade://team/join` ouvert au premier lancement) : le parcours fait d'abord « Rejoindre l'équipe », avec la vérification des six mots de son empreinte (spec « Entrer dans une équipe »). La station du projet propose ensuite les projets de l'équipe.
- Le parcours ne contourne jamais un refus du cœur. Il montre ce que le cœur refuserait.

### 5.7 Hors ligne

Tout le parcours marche sans réseau, sauf trois choses qui disent pourquoi et laissent continuer : le clone, la connexion d'un compte qui demande le navigateur, et la voix en ligne. La démonstration se crée hors ligne. Le premier message d'un agent demande le réseau du harness, pas celui d'Escouade : un échec dit « tu sembles hors ligne » et garde le message.

## 6. La forme : trois directions

Rendues dans `design/onboarding-2-3/` et comparées dans son `README.md`.

- **A « Essaie d'abord »** : un bac à sable dès l'ouverture, un guide « Premiers pas » en colonne dans la vraie fenêtre, chaque réglage demandé quand il sert, un fil teal du guide à la vraie chose.
- **B « Au bon endroit »** : trois questions dans l'écran sans projet, puis des bulles posées sur les vrais contrôles et une liste « À découvrir » dans la barre latérale.
- **C « La ligne »** : un plein écran, une station par question sur une ligne tracée comme le git graph. La conversation réelle est encadrée à la station du premier agent. La découverte est une carte du cycle (Planifier, Coder, Relire, Livrer, Piloter).

**Recommandation : C**, avec les bulles au bon moment de B pour la découverte dans l'app (choix O1, O2).

## 7. La découverte des fonctionnalités

Après la fin du parcours. **Chaque découverte tient en une phrase, un geste (« Essayer ») et se coche par l'état réel.** Elle n'apparaît que si elle est possible ici. Écartée, elle ne revient jamais d'elle-même. « Tout masquer » cache tout. Tout se retrouve dans Aide › À découvrir.

| Id | Moment | Découverte (fr / en) | La phrase | Le geste | Faite quand | Visible si | Ce qui est dit avant |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `ticket` | Planifier | Écrire un ticket / Write a ticket | Un ticket et ses critères ; un agent boucle jusqu'à les atteindre. / A ticket and its criteria; an agent loops until they're met. | ouvre le Kanban, « Nouveau ticket » | `ticket_created` | toujours | — |
| `autopilot` | Planifier | Le pilote auto / The autopilot | Il lance les tickets prêts, un agent chacun ; jamais de merge sans toi. / It starts ready tickets, one agent each; never a merge without you. | Kanban › Pilote auto | pilote allumé une fois | un ticket existe | le quota consommé |
| `docs` | Planifier | La spec et les décisions / The spec and decisions | L'onglet Docs, tenu à jour par les agents des tickets. / The Docs tab, kept up to date by ticket agents. | ouvre Docs | Docs ouvert | toujours | — |
| `dictation` | Coder | Dicter un message / Dictate a message | Maintiens le micro, parle, relâche : ton texte arrive dans le champ. / Hold the mic, speak, release: your text lands in the field. | télécharge le modèle, puis le micro | `dictation_used` | un micro, `voice.dictation` permis | « Whisper “base”, 148 Mo, téléchargé une fois ; rien de ce que tu dis ne quitte ta machine. » |
| `voice` | Coder | Parler à un agent / Talk to an agent | Une conversation à voix haute avec un agent. / A spoken conversation with an agent. | ouvre la conversation vocale | `voice_conversation` | micro et sortie son | le modèle, et le service en ligne s'il est choisi |
| `terminal` | Coder | Un terminal / A terminal | Dans le dossier du projet ou le worktree d'un agent. / In the project folder or an agent's worktree. | `+` des terminaux | `terminal_opened` | toujours | — |
| `editor` | Coder | L'éditeur / The editor | Ouvrir un fichier, voir ce qu'un agent a changé. / Open a file, see what an agent changed. | Ctrl P | un fichier ouvert | toujours | (la vue VS Code peut télécharger Node à la demande : dit à ce moment-là) |
| `second-agent` | Coder | Un deuxième agent / A second agent | Deux agents en parallèle, chacun sur sa branche. / Two agents in parallel, each on its branch. | Ctrl N | deux agents dans un projet | toujours | le quota |
| `launch` | Relire | Lancer le projet / Run the project | Les commandes qui lancent le projet, proposées par un agent, relues avant de tourner. / The commands that run your project, suggested by an agent, reviewed before they run. | Réglages du projet › Lancement › « Proposer des commandes » | `launch_started` | toujours | un message à un agent |
| `forge` | Relire | Connecter GitHub / Connect GitHub | Les PR et la CI de tes tickets, dans Escouade. / Your tickets' PRs and CI, inside Escouade. | Réglages › Intégrations | `integration_connected` | le plugin de l'hébergeur est installé (sinon : « Installer depuis Plugins ») | « un jeton, gardé dans le trousseau du système » |
| `pr-view` | Relire | Une PR dans Escouade / A PR in Escouade | Sa description, ses fichiers, sa CI, sa discussion. / Its description, files, CI and discussion. | ouvre la vue PR | une PR ouverte dans l'app | un hébergeur connecté | — |
| `pr-review` | Relire | Relecture par un agent / Review by an agent | Un agent commente la PR ligne par ligne, comme un relecteur. / An agent comments on the PR line by line, like a reviewer. | « Relire avec un agent » | `pr_review_started` | une PR ouverte | le quota |
| `open-pr` | Livrer | Ouvrir une PR / Open a PR | La branche d'un agent poussée, sa PR ouverte, sa CI suivie. / An agent's branch pushed, its PR opened, its CI followed. | menu de l'agent › Ouvrir une PR | `pr_opened` | un hébergeur connecté | — |
| `approve` | Livrer | Valider un ticket / Approve a ticket | Merge, PR ou push : le ticket entre. / Merge, PR or push: the ticket lands. | un ticket À tester › Valider | `ticket_approved` | un ticket À tester | — |
| `plugin` | Piloter | Un plugin / A plugin | Le store : harness, hébergeurs, voix, thèmes. / The store: harnesses, code hosts, voice, themes. | ouvre Plugins | `plugin_installed` | `plugins.*` le permet | le réseau |
| `mcp` | Piloter | Le serveur MCP / The MCP server | Piloter Escouade depuis un autre client (Claude Code dans un terminal). / Drive Escouade from another client (Claude Code in a terminal). | Réglages › MCP | `mcp_enabled` | `mcp.externalClients` ≠ `denied` | « écoute sur cette machine seulement, derrière un jeton » |
| `remote` | Piloter | Ton téléphone / Your phone | Escouade à distance : répondre à tes agents depuis ton téléphone. / Escouade remote: answer your agents from your phone. | Réglages › À distance | l'API distante allumée | `servers.remoteApi` permis, la version l'a | — |
| `team` | Piloter | Ton équipe / Your team | Le Kanban, la spec et les décisions partagés avec ton équipe. / The board, spec and decisions shared with your team. | Rejoindre par un lien | une équipe rejointe | un serveur d'équipe connu ou permis | — |

**Au bon moment** (choix O11) : trois bulles seulement, une par session au plus, jamais pendant la frappe ni pendant une demande de permission, jamais deux fois :

1. le micro survolé la première fois → `dictation` ;
2. le premier ticket créé → `autopilot` ;
3. la première PR ouverte par Escouade → `pr-review`.

## 8. Les règles

- **Clavier** : tout le parcours se fait au clavier. `↑ ↓` pour choisir (un `radiogroup` par question, comme `FirstModeModal`), `Entrée` pour valider, `Échap` pour « Plus tard », `Tab` dans l'ordre visuel. Le focus est visible (anneau teal) ; il arrive sur la question à chaque station.
- **Lecteur d'écran** : la ligne des stations est une `nav` avec `aria-current="step"`. Chaque question est un `h1`, son pourquoi est sa description (`aria-describedby`). Une bulle est un `dialog` non modal, annoncé par `aria-live="polite"`, que `Échap` ferme. Le fil de A et les voies de C sont décoratifs (`aria-hidden`) : leur sens est dans le texte.
- **Tailles** : 1440 × 900 et 1024 × 728, sans défilement horizontal. En 1024, A replie les étapes de son guide en une ligne, C passe la carte sur une colonne et pose le détail en bas. Avec le zoom de l'app, les mêmes règles qu'ailleurs.
- **Thèmes** : clair et sombre, contrastes AA (`src/lib/contrast.test.ts` les vérifie aussi pour les nouveaux jetons). Les voies de C, foncées en clair (O14).
- **Langues** : français et anglais, tout le texte par `t()`, aucune chaîne en dur. Pluriels par les règles de la langue. Espaces insécables dans les guillemets français.
- **Animations** : un seul moment animé, l'arrivée d'une station (une montée de 8 px en 240 ms, `--ease`). Sous `prefers-reduced-motion`, aucun mouvement, un fondu de 120 ms au plus.
- **Rien sans geste** : aucun téléchargement, aucune installation, aucune connexion ni aucun message envoyé par le parcours lui-même.

## 9. Ce qui se mesure

Avec l'accord des statistiques seulement, pseudonymes comme les autres. Jamais un contenu, un nom de projet, un chemin ni un e-mail. Les valeurs sont des listes fermées. À ajouter à `src-tauri/src/telemetry.rs`, à la page de confidentialité du site (fr et en, ses tests les comptent) et, s'il le faut, au registre `docs/registre-rgpd.md`.

| Événement | Propriétés | Quand |
| --- | --- | --- |
| `onboarding_started` | `trigger` : `first` \| `redo` \| `update` ; `profile` : `ready` \| `installed` \| `none` \| `managed` (ce qu'Escouade a trouvé) | le parcours s'ouvre |
| `onboarding_step` | `step` : `harness` \| `account` \| `defaults` \| `project` \| `agent` \| `telemetry` ; `outcome` : `done` \| `skipped` \| `policy` ; `choice` : `recommended` \| `other` | une station se ferme |
| `onboarding_finished` | `skipped` : 0 à 6 ; `seconds` : `<60` \| `60-120` \| `120-300` \| `>300` | la dernière station, ou « Tout passer » |
| `first_reply` | `seconds` : mêmes tranches, depuis `onboarding_started` ; `demo` : yes \| no | le premier agent finit son premier tour |
| `discover_done` | `item` : l'id de la découverte ; `from` : `list` \| `tip` | une découverte se coche |
| `discover_dismissed` | `item` | une découverte écartée |
| `discover_hidden` | — | « Tout masquer » |

Le taux d'arrivée au premier agent : `first_reply` ÷ `onboarding_started{trigger=first}`, par semaine. L'entonnoir : `onboarding_step` par `step` et `outcome`. Le temps médian se lit sur les tranches de `seconds`. Les profils `none` et `installed` disent combien d'utilisateurs arrivent sans harness prêt.

## 10. Les choix à trancher

**Tranché par l’utilisateur le 2026-11-17** (ces lignes font foi pour ON-1 à ON-4) :

- **O1 : la direction C « La ligne »** (« le C me plaît bien »).
- **Une station de plus, « Ce que tu veux brancher »** (demandée le même jour) : le premier lancement propose les fonctionnalités optionnelles, chacune passable, rien d’installé ni de connecté sans le geste :
  - **les sources de tickets** (Jira, Trello, GitHub, Linear, ClickUp, GitLab, Azure Boards : celles que le store et l’amorce offrent), avec ce que chacune importe et ce qu’elle demande (un jeton, un projet) ;
  - **la voix**, si on veut s’en servir : le fournisseur de la dictée et de la conversation, Whisper en local (son poids dit avant le téléchargement) ou un service en ligne (Mistral, ElevenLabs, Deepgram, OpenAI, Groq… : ceux qui sont offerts), avec sa clé ;
  - et les autres branchements du même genre que la conception retient (l’hébergeur de code pour les PR et la CI, un second harness, le serveur MCP pour un autre client), sans allonger le parcours : une seule station, des cartes à cocher, « Plus tard » pour tout.
  - Elle n’installe que les plugins de ce qui est choisi (le tri de l’amorce), et chaque choix se retrouve dans la découverte s’il est passé. Ticket : ON-1b (ESC-512).
- **O2 à O14** : les recommandations ci-dessous sont retenues par défaut, tant que l’utilisateur n’en change pas une.

Chacun avec ma recommandation. L'utilisateur tranche ; ON-1 à ON-4 suivent.

1. **O1. La direction** : A « Essaie d'abord », B « Au bon endroit » ou C « La ligne ». **Recommandé : C.** C'est la seule qui ait la place d'être « très didactique », elle se fait au clavier, et sa ligne (le git graph) est propre à Escouade.
2. **O2. La forme de la découverte** : la carte de C (une page, Aide › À découvrir), la liste de B dans la barre, ou le guide de A. **Recommandé : la carte de C, plus les trois bulles au bon moment de B** (§ 7) et un point sur l'icône Aide du rail tant qu'il reste des découvertes ni faites ni écartées. Pas de liste permanente dans la barre : elle prend la place des agents.
3. **O3. Le projet de démonstration** : le créer, et quoi. **Recommandé** : oui, une des trois réponses de la station du projet (jamais imposée). `~/.escouade/demo/bac-a-sable`, un dépôt git local écrit par l'app : une liste de tâches en TypeScript, deux tests qui échouent exprès, **sans dépendance à installer** (`node --test`). L'amorce « Lance les tests » n'est proposée que si Node est sur la machine. Une bande le signale, « Supprimer » l'efface ; jamais retiré sans geste.
4. **O4. Le choix recommandé de la station du projet** : un dossier, ou la démonstration. **Recommandé : un dossier** (la demande est de « setup rapidement Escouade » : le vrai projet d'abord). La démonstration reste à un clic.
5. **O5. Le premier message** : envoyé par le parcours, ou proposé. **Recommandé : proposé seulement**, trois amorces selon le projet, la première sélectionnée, modifiable, envoyée par le clic de l'utilisateur. Le quota consommé est dit.
6. **O6. Le compte** : une station à part, ou fondue dans celle du harness. **Recommandé : fondue** quand le harness est connecté sur la machine (« ses agents partent sur le compte déjà connecté ; un compte à part se crée dans Réglages »). À part sinon : connexion, ou compte séparé.
7. **O7. Les statistiques** : au début (aujourd'hui, juste après le mode) ou à la fin. **Recommandé : à la fin**, après le premier agent. Les mêmes textes, aucune réponse mise en avant, redemandées une fois si passées.
8. **O8. Le nom pour l'équipe** (ESC-190) : dans le premier lancement, ou au premier partage. **Recommandé : au premier partage seulement.** Son « Plus tard » le fait déjà ; le premier lancement n'a pas à le demander.
9. **O9. Après une mise à jour majeure** : rien, la bande « Nouveau dans la 2.3 », ou le parcours. **Recommandé : la bande**, qui ouvre la découverte filtrée sur le nouveau, et se ferme pour la version.
10. **O10. L'aide** : où se retrouvent le parcours et la découverte. **Recommandé : une icône Aide dans le bas du rail**, au-dessus des stats. Elle porte un menu : Premier lancement, À découvrir, Raccourcis, Documentation (le site), Signaler un problème. Plus « Refaire le premier lancement » dans Réglages › Général.
11. **O11. Les bulles au bon moment** : aucune, trois, ou une par découverte. **Recommandé : trois** (le micro, le premier ticket, la première PR), une par session au plus.
12. **O12. Le poste géré** : cacher ou montrer ce qui est imposé. **Recommandé : montrer, verrouillé, avec qui l'impose** ; cacher seulement ce qui n'a plus d'objet (statistiques éteintes, un seul harness permis).
13. **O13. La mesure** : ajouter les sept événements du § 9. **Recommandé : oui**, avec la page de confidentialité du site et ses tests dans le même ticket (ON-4).
14. **O14. Les voies du git graph en thème clair** : les foncer (L ≈ 0,55) pour tenir 3:1 sur le fond clair, ce que la ligne de C demande. **Recommandé : oui**, dans `src/app.css`, avec `contrast.test.ts`. Le git graph en profite aussi.

## 11. Le découpage en tickets

Estimé au rythme d'agents de Claude Code (6 en parallèle), en heures d'agent. ON-1 à ON-4 existent comme ébauches (ESC-504 à ESC-507) ; voici ce qu'il faut y changer une fois la direction choisie. Les estimations valent pour C ; A coûterait environ 3 heures de plus (le bac à sable au centre, le fil, le guide), B environ 2 heures de moins.

| Ticket | Ce qui change dans l'ébauche | Estimation |
| --- | --- | --- |
| **ON-1 (ESC-504)** · le socle et les stations harness, compte, réglages de départ, statistiques | **Ajouter** : l'état `onboarding` du § 5 dans l'état durable ; les stations faites par l'état réel ; la ligne (le composant des stations) et le plein écran de C ; l'icône Aide du rail et son menu (O10) ; « Refaire le premier lancement » dans Réglages › Général ; la bande « Nouveau dans la 2.3 » (O9) ; les voies foncées du thème clair (O14). **Retirer** : l'enchaînement des fenêtres `firstMode` → `telemetry` → `teamName` de `state.svelte.ts`, remplacé par le parcours, et `teamName` sorti du premier lancement (O8). Garder `FirstModeModal` et `TelemetryModal` tant qu'un autre chemin les ouvre, sinon les retirer. **Préciser** : l'e2e avec un dossier de données vide, avec puis sans harness, et un poste géré (`ESCOUADE_TEST_POLICIES`) qui impose un harness et interdit Bypass. | 2 h 30 |
| **ON-2 (ESC-505)** · les stations du premier projet et du premier agent | **Préciser** : la démonstration selon O3 (`node --test`, sans dépendance) ; les amorces selon le projet (O5) ; la conversation réelle encadrée dans la station, la permission expliquée à côté de la vraie carte ; la station du projet propose les projets de l'équipe pour P4. **Ajouter** : l'invitation d'équipe ouverte au premier lancement (§ 5.6). **Retirer** : « retiré de lui-même quand on le quitte » : la démonstration n'est retirée que par un geste. | 2 h 30 |
| **ON-3 (ESC-506)** · la découverte | **Préciser** : la carte de C comme page Aide › À découvrir ; les 18 découvertes du § 7 avec leur condition de visibilité et ce qui est dit avant ; les trois bulles au bon moment (O11) ; le point sur l'icône Aide. **Ajouter** : le filtre « nouveau dans la version » qu'ouvre la bande d'ON-1. Le reste de l'ébauche tient. | 3 h |
| **ON-4 (ESC-507)** · la finition, l'e2e de bout en bout, le site, la mesure | **Préciser** : les sept événements du § 9, la requête du taux d'arrivée, la page de confidentialité ; l'objectif de temps de P1 (moins de 2 minutes) mesuré. Le reste de l'ébauche tient. | 1 h 30 |

Total : **4 tickets, environ 9 h 30 d'agent**. ON-2 et ON-3 partent ensemble après ON-1 : environ **7 heures** de bout en bout.

Ce qui attend l'utilisateur, à part : les choix O1 à O14 (avant ON-1), et les essais réels d'un premier lancement sur un poste vierge (Windows, macOS, Linux) avec un vrai harness à installer. Ceux-ci, aucun e2e ne les remplace.
